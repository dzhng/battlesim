// @vitest-environment node
import { describe, expect, test } from "vitest";
import {
  add,
  isMapInputChange,
  lobbyCandidate,
  preset,
  remove,
  reroll,
  seatHuman,
  setFaction,
  setRegion,
  setSize,
  setType,
  toUrl,
  type LobbyState,
} from "../../apps/battle-lab/src/lobby/model";
import { askedBattle } from "../../apps/battle-lab/src/battleLinks";
import { skirmishRequest } from "../../apps/battle-lab/src/skirmishSetup";
import { loadWasm } from "../src/battle/sim/module";

const ids = (state: LobbyState) => state.players.map((player) => player.id);

describe("lobby model", () => {
  test("adds and removes uneven rosters within the lobby limit", () => {
    let state = preset("1v1");
    state = add(state, "blue");
    state = add(state, "blue");
    state = add(state, "red");
    expect(ids(state)).toEqual(["blue-1", "blue-2", "blue-3", "red-1", "red-2"]);
    state = add(state, "blue");
    state = add(state, "blue");
    expect(() => add(state, "blue")).toThrow("maximum");
    // Players are numbered by place, as a link names them: removing one
    // closes the gap.
    state = remove(state, "blue-3");
    expect(ids(state)).toEqual(["blue-1", "blue-2", "blue-3", "blue-4", "red-1", "red-2"]);
    for (let n = 0; n < 3; n++) state = remove(state, "blue-2");
    expect(ids(state)).toEqual(["blue-1", "red-1", "red-2"]);
    expect(() => remove(state, "blue-1")).toThrow("Human");
  });

  test("seats a different Blue row while keeping one Human", () => {
    let state = preset("3v3");
    state = seatHuman(state, "blue-2");
    expect(state.players.filter((player) => player.controller === "human")).toEqual([
      expect.objectContaining({ id: "blue-2", side: "blue" }),
    ]);
    expect(state.players.find((player) => player.id === "blue-2")?.controller).toBe("human");
    expect(state.players.find((player) => player.id === "red-1")?.controller).toBe("active_ai");
  });

  test.each(["1v1", "3v3", "5v5"] as const)(
    "keeps exactly one Blue human after every controller operation (%s)",
    (kind) => {
      let state = preset(kind);
      if (kind === "1v1") state = add(state, "blue");
      const red = state.players.find((player) => player.side === "red")!;
      expect(() => seatHuman(state, red)).toThrow("Blue");
      const target = state.players.find(
        (player) => player.side === "blue" && player.controller !== "human",
      )!;
      const seated = seatHuman(state, target);
      expect(seated.players.filter((player) => player.controller === "human")).toEqual([
        expect.objectContaining({ id: target.id, side: "blue" }),
      ]);
    },
  );

  test("refuses removing the human row", () => {
    expect(() => remove(preset("3v3"), "blue-1")).toThrow("Human");
  });

  test("ratchets size upward and never down", () => {
    let state = preset("1v1");
    state = setSize(state, "xl");
    expect(state.map.size).toBe("xl");
    state = add(state, "red");
    state = remove(state, "red-2");
    expect(state.map.size).toBe("xl");
    expect(setSize(state, "medium").map.size).toBe("medium");
  });

  test("removing players lowers the size floor to the new minimum but keeps the size", () => {
    let state = add(add(preset("1v1"), "red"), "red");
    expect(state.map.size).toBe("large");
    state = remove(remove(state, "red-3"), "red-2");
    expect(state.map.size).toBe("large");
    expect(state.sizeFloor).toBe("small");
    expect(setSize(state, "small").map.size).toBe("small");
  });

  test("a preset reseats the lobby it is applied to, keeping its map and factions", () => {
    let state = setFaction(preset("1v1"), "red", "europe");
    state = setRegion(setType(reroll(state, "42"), "metro"), "paris");
    const reseated = preset("5v5", state);
    expect(reseated.players.filter((player) => player.side === "blue")).toHaveLength(5);
    expect(reseated.players.every((p) => p.side === "blue" || p.faction === "europe")).toBe(true);
    expect(reseated.map).toMatchObject({ type: "metro", region: "paris", seed: "42", size: "xl" });
    // Back to a smaller preset: the size drops to that preset's floor.
    expect(preset("1v1", reseated).map.size).toBe("small");
  });

  test("map type and region are map inputs; no region leaves it to the seed", () => {
    const state = preset("1v1");
    expect(isMapInputChange(state, setType(state, "open"))).toBe(true);
    const paris = setRegion(state, "paris");
    expect(isMapInputChange(state, paris)).toBe(true);
    expect(setRegion(paris, undefined).map).not.toHaveProperty("region");
  });

  test("reroll only changes the map seed", () => {
    const state = preset("3v3");
    const next = reroll(state, "99");
    expect(next.map.seed).toBe("99");
    expect({ ...next, map: { ...next.map, seed: state.map.seed } }).toEqual(state);
  });

  test("controller and faction changes do not change map inputs", () => {
    const state = preset("1v1");
    const three = preset("3v3");
    expect(isMapInputChange(three, seatHuman(three, "blue-2"))).toBe(false);
    expect(isMapInputChange(state, setFaction(state, "blue", "europe"))).toBe(false);
    expect(isMapInputChange(state, add(state, "blue"))).toBe(true);
  });

  test("map input entries use the three-entry admission buckets", () => {
    let state = preset("1v1");
    expect(isMapInputChange(state, add(state, "blue"))).toBe(true);
    state = add(state, "blue");
    expect(isMapInputChange(state, add(state, "blue"))).toBe(true);
    state = add(state, "blue");
    expect(isMapInputChange(state, add(state, "blue"))).toBe(false);
  });

  test.each(["1v1", "3v3", "5v5"] as const)("URL round trips %s", (kind) => {
    const state = reroll(preset(kind), "42");
    expect(askedBattle(toUrl(state))).toEqual({
      kind: "generated",
      players: state.players,
      map: { type: state.map.type, size: state.map.size, seed: "42" },
      battleSeed: state.battleSeed,
    });
  });

  test("an added player fields its side's faction", () => {
    const state = add(setFaction(preset("1v1"), "red", "europe"), "red");
    expect(state.players.filter((p) => p.side === "red").map((p) => p.faction)).toEqual([
      "europe",
      "europe",
    ]);
  });
});

test("Deploy's link prepares the very request the lobby previews, after removing and re-seating", async () => {
  const wasm = await loadWasm();
  let state = remove(preset("3v3"), "blue-2");
  const last = state.players.filter((player) => player.side === "blue").at(-1)!;
  state = seatHuman(state, last);
  const previewed = lobbyCandidate(wasm, state)(state.map.seed);
  const asked = askedBattle(toUrl(state));
  if ("error" in asked || asked.kind !== "generated") throw new Error(JSON.stringify(asked));
  // The battle route builds its request from what the link asks.
  expect(skirmishRequest(wasm, asked)).toEqual(previewed);
  expect(previewed.map_source.request.entries_per_side).toBe(3);
});
