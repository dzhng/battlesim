import { lightingPresetNames } from "../../packages/skirmish-setup/src/lighting";
// @vitest-environment node
import { describe, expect, test } from "vitest";
import {
  add,
  isMapInputChange,
  lobbyPlayers,
  preset,
  remove,
  reroll,
  seatHuman,
  setFaction,
  setLighting,
  setOccupant,
  setRegion,
  setSize,
  setType,
  sizeFloor,
  type LobbyState,
} from "../../packages/skirmish-setup/src/lobby";
import { battlePlayers } from "../../packages/skirmish-setup/src/seats";
import { askedBattle, toUrl } from "../../apps/battle-lab/src/battleLinks";
import { DEFAULT_LIGHTING } from "../../apps/battle-lab/src/lobby/lighting";
import { battleRequestForAsked, lobbyCandidate } from "../../apps/battle-lab/src/skirmishData";
import { loadWasm } from "../src/battle/sim/module";

const ids = (state: LobbyState) => state.seats.map((seat) => seat.id);
const seat = (state: LobbyState, side: "blue" | "red", index = 0) =>
  state.seats.filter((candidate) => candidate.side === side)[index]!;

describe("lobby model", () => {
  test("seat minting is deterministic and never reuses a removed id", () => {
    const initial = preset("1v1");
    expect(preset("3v3")).toEqual(preset("3v3"));
    expect(add(initial, "blue")).toEqual(add(initial, "blue"));
    const added = add(initial, "blue");
    const removed = seat(added, "blue", 1);
    const next = add(remove(added, removed), "blue");
    expect(next.seats.map((candidate) => candidate.id)).not.toContain(removed.id);
  });

  test("a side takes seats up to its limit and refuses one more", () => {
    let state = preset("1v1");
    for (let count = 1; count < 5; count++) state = add(state, "blue");
    expect(lobbyPlayers(state).filter((player) => player.side === "blue")).toHaveLength(5);
    expect(() => add(state, "blue")).toThrow("maximum");
  });

  test("removing a seat leaves every other seat id alone", () => {
    let state = preset("1v1");
    const original = ids(state);
    state = add(add(add(state, "blue"), "blue"), "red");
    const removed = seat(state, "blue", 1);
    const before = ids(state).filter((id) => id !== removed.id);
    state = remove(state, removed);
    expect(ids(state)).toEqual(before);
    expect(ids(state).filter((id) => original.includes(id))).toEqual(original);
  });

  test("seats may hold several humans on either side", () => {
    let state = preset("3v3");
    state = seatHuman(state, seat(state, "blue", 1), "second");
    state = seatHuman(state, seat(state, "red"), "red-player");
    expect(lobbyPlayers(state).filter((player) => player.controller === "human")).toHaveLength(3);
  });

  test("open seats are dropped and filled seats close up in seat order", () => {
    let state = preset("3v3");
    const open = seat(state, "blue", 1);
    state = setOccupant(state, open.id, { kind: "open" });
    const result = battlePlayers(state.seats);
    expect(
      result.players.filter((player) => player.side === "blue").map((player) => player.id),
    ).toEqual(["blue-1", "blue-2"]);
    expect(result.playerOf[open.id]).toBeUndefined();
    expect(state.map.size).toBe("large");
  });

  test("a side without a human or AI is refused", () => {
    const state = preset("1v1");
    const empty = state.seats
      .filter((seat) => seat.side === "red")
      .reduce((next, seat) => setOccupant(next, seat.id, { kind: "open" }), state);
    expect(() => battlePlayers(empty.seats)).toThrow(/red/);
  });

  test("size floor follows filled seats while open seats do not add roads", () => {
    let state = preset("1v1");
    state = add(state, "blue");
    expect(sizeFloor(state)).toBe("medium");
    const open = seat(state, "blue", 1);
    const opened = setOccupant(state, open.id, { kind: "open" });
    expect(sizeFloor(opened)).toBe("small");
    expect(isMapInputChange(state, opened)).toBe(true);
  });

  test("an all-open side remains editable and sized, but cannot become a battle", () => {
    const initial = preset("1v1");
    const state = setOccupant(initial, seat(initial, "blue").id, { kind: "open" });
    expect(sizeFloor(state)).toBe("small");
    expect(setSize(state, "medium").map.size).toBe("medium");
    expect(isMapInputChange(initial, state)).toBe(false);
    const added = add(state, "blue");
    const secondOpen = setOccupant(added, seat(added, "blue", 1).id, { kind: "open" });
    expect(() => remove(secondOpen, seat(secondOpen, "blue", 1))).not.toThrow();
    expect(() => lobbyPlayers(state)).toThrow(/blue/);
    expect(() => battlePlayers(state.seats)).toThrow(/blue/);
  });

  test("an open seat can be removed, but never the side's last seat", () => {
    let state = add(preset("1v1"), "blue");
    const open = seat(state, "blue", 1);
    state = setOccupant(state, open.id, { kind: "open" });
    expect(remove(state, open).seats.some((candidate) => candidate.id === open.id)).toBe(false);
    const initial = preset("1v1");
    const onlyOpen = setOccupant(initial, seat(initial, "blue").id, { kind: "open" });
    expect(() => remove(onlyOpen, onlyOpen.seats[0]!)).toThrow(/at least one seat/);
  });

  test("filling an open seat raises the size to fit the players who will now fight", () => {
    let state = add(add(preset("1v1"), "blue"), "blue");
    const third = seat(state, "blue", 2);
    state = setOccupant(state, third.id, { kind: "open" });
    state = setSize(state, "medium");
    expect(state.map.size).toBe("medium");
    expect(sizeFloor(setOccupant(state, third.id, { kind: "ai" }))).toBe("large");
    expect(setOccupant(state, third.id, { kind: "ai" }).map.size).toBe("large");
    expect(seatHuman(state, third).map.size).toBe("large");
    // Emptying a seat lowers the floor and, like a removal, keeps the size.
    const full = setOccupant(state, third.id, { kind: "ai" });
    expect(setOccupant(full, third.id, { kind: "open" }).map.size).toBe("large");
  });

  test("size rises with the roster, keeps its choice as the floor falls, then follows a smaller choice", () => {
    let state = add(preset("1v1"), "red");
    expect(state.map.size).toBe("medium");
    state = add(state, "red");
    expect(state.map.size).toBe("large");
    state = setSize(state, "xl");
    state = add(state, "red");
    expect(state.map.size).toBe("xl");
    state = remove(state, seat(state, "red", 3));
    expect(sizeFloor(state)).toBe("large");
    expect(state.map.size).toBe("xl");
    state = remove(state, seat(state, "red", 2));
    expect(sizeFloor(state)).toBe("medium");
    expect(state.map.size).toBe("xl");
    state = remove(state, seat(state, "red", 1));
    expect(sizeFloor(state)).toBe("small");
    expect(state.map.size).toBe("xl");
    const chosen = state;
    expect(setSize(state, "small").map.size).toBe("small");
    expect(preset("1v1", chosen).map.size).toBe("small");
  });

  test("preset keeps map and factions while replacing its seats", () => {
    let state = setFaction(preset("1v1"), "red", "europe");
    state = setRegion(setType(reroll(state, "42"), "metro"), "paris");
    const reseated = preset("5v5", state);
    expect(reseated.seats).toHaveLength(10);
    expect(reseated.factions.red).toBe("europe");
    expect(reseated.map).toMatchObject({ type: "metro", region: "paris", seed: "42", size: "xl" });
  });

  test("map type and region are map inputs", () => {
    const state = preset("1v1");
    expect(isMapInputChange(state, setType(state, "open"))).toBe(true);
    const paris = setRegion(state, "paris");
    expect(isMapInputChange(state, paris)).toBe(true);
    expect(setRegion(paris, undefined).map).not.toHaveProperty("region");
  });

  test("reroll changes only the map seed", () => {
    const state = preset("3v3");
    const next = reroll(state, "99");
    expect(next.map.seed).toBe("99");
    expect({ ...next, map: { ...next.map, seed: state.map.seed } }).toEqual(state);
  });

  test("controller and faction changes are not map inputs", () => {
    const state = preset("3v3");
    expect(isMapInputChange(state, seatHuman(state, seat(state, "blue", 1)))).toBe(false);
    expect(isMapInputChange(state, setFaction(state, "blue", "europe"))).toBe(false);
  });

  test("player admission changes map inputs only at the three-entry buckets", () => {
    let state = preset("1v1");
    let next = add(state, "blue");
    expect(isMapInputChange(state, next)).toBe(true);
    state = next;
    next = add(state, "blue");
    expect(isMapInputChange(state, next)).toBe(true);
    state = setSize(next, "xl");
    for (let index = 0; index < 2; index++) {
      next = add(state, "blue");
      expect(isMapInputChange(state, next)).toBe(false);
      state = next;
    }
  });

  test.each(["1v1", "3v3", "5v5"] as const)("URL round trips %s exactly", (kind) => {
    const state = reroll(preset(kind), "42");
    const asked = askedBattle(toUrl(state));
    expect(asked).toEqual({
      kind: "generated",
      players: lobbyPlayers(state),
      map: { type: state.map.type, size: state.map.size, seed: state.map.seed },
      battleSeed: state.battleSeed,
      lighting: state.lighting,
    });
  });

  test("the lighting choice travels on the link and defaults to the fixture's look", () => {
    const state = reroll(preset("1v1"), "42");
    expect(state.lighting).toBe(DEFAULT_LIGHTING);
    expect(toUrl(state)).not.toMatch(/light=/);
    const [other] = lightingPresetNames().filter((name) => name !== DEFAULT_LIGHTING);
    const golden = setLighting(state, other);
    expect(toUrl(golden)).toContain(`light=${other}`);
    expect(askedBattle(toUrl(golden))).toMatchObject({ kind: "generated", lighting: other });
    const random = setLighting(state, "random");
    expect(askedBattle(toUrl(random))).toMatchObject({ kind: "generated", lighting: "random" });
  });

  test("an added seat fields its side's faction", () => {
    const state = add(setFaction(preset("1v1"), "red", "europe"), "red");
    expect(
      lobbyPlayers(state)
        .filter((player) => player.side === "red")
        .map((p) => p.faction),
    ).toEqual(["europe", "europe"]);
  });

  test("the model removes a human's seat like any other: keeping the offline player seated is the page's rule", () => {
    let state = preset("3v3");
    state = remove(state, seat(state, "blue"));
    const blue = lobbyPlayers(state).filter((player) => player.side === "blue");
    expect(blue).toHaveLength(2);
    expect(blue.some((player) => player.controller === "human")).toBe(false);
  });
});

test("Deploy's link prepares the request the lobby previews after removing and re-seating", async () => {
  const wasm = await loadWasm();
  let state = preset("3v3");
  state = remove(state, seat(state, "blue"));
  state = seatHuman(state, seat(state, "blue", 1));
  state = reroll(state, "42");
  const previewed = lobbyCandidate(wasm, state)(state.map.seed);
  const asked = askedBattle(toUrl(state));
  if ("error" in asked || asked.kind !== "generated") throw new Error(JSON.stringify(asked));
  expect(battleRequestForAsked(wasm, asked)).toEqual(previewed);
  expect(previewed.map_source.request.entries_per_side).toBe(3);
});
