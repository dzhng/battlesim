// @vitest-environment node
import { describe, expect, test } from "vitest";
import {
  add,
  fromUrl,
  isMapInputChange,
  preset,
  remove,
  reroll,
  seatHuman,
  setController,
  setFaction,
  setSize,
  toRequest,
  toUrl,
  type LobbyState,
} from "../../apps/battle-lab/src/lobby/model";

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
    state = remove(state, "blue-3");
    state = remove(state, "blue-2");
    state = remove(state, "blue-4");
    state = remove(state, "blue-5");
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
      expect(() => setController(state, red, "human")).toThrow("Blue");
      expect(() => seatHuman(state, red)).toThrow("Blue");
      expect(() => setController(state, "blue-1", "active_ai")).toThrow("Human");
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

  test("reroll only changes the map seed", () => {
    const state = preset("3v3");
    const next = reroll(state, "99");
    expect(next.map.seed).toBe("99");
    expect({ ...next, map: { ...next.map, seed: state.map.seed } }).toEqual(state);
  });

  test("controller and faction changes do not change map inputs", () => {
    const state = preset("1v1");
    expect(isMapInputChange(state, setController(state, "red-1", "passive_ai"))).toBe(false);
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
    const state = preset(kind);
    const roundTrip = fromUrl(toUrl(state));
    expect(roundTrip).toEqual(state);
  });

  test("request carries the shared entry count and ordered players", () => {
    const request = toRequest(preset("1v1"));
    expect(request.players).toHaveLength(2);
    expect(request.battle_seed).toBeTypeOf("number");
    expect(request.map_source.request.entries_per_side).toBe(1);
  });
});
