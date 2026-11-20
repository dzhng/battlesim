// @vitest-environment node
import { expect, test } from "vitest";
import {
  battleSetup,
  canStart,
  createRoom,
  deserted,
  roomCode,
  ROOM_CODE_ALPHABET,
  recordBattle,
  roomReduce,
  roomView,
  type Room,
} from "../../../packages/multiplayer/src/room";
import { fromBattleSetup } from "../../../packages/multiplayer/src/setup";
import {
  battleRequest,
  setOccupant,
  type SetupData,
} from "../../../packages/skirmish-setup/src/lobby";

const host = {
  who: "browser-a",
  callsign: "Alpha",
};
const guest = {
  who: "browser-b",
  callsign: "Bravo",
};
const room = (visibility: "public" | "private" = "public") =>
  createRoom(host, visibility, "TANGO-4821", "7", 10);
const changed = (state: Room, by: string, action: Parameters<typeof roomReduce>[2], now = 20) => {
  const next = roomReduce(state, by, action, now);
  if ("refused" in next) throw new Error(next.refused);
  return next;
};

test("room creation assigns its first seat and presence time from the room inputs", () => {
  const legacyShape = { ...host, seat: "seat-99", ready: true, since: -1 };
  const state = createRoom(legacyShape, "public", "TANGO-4821", "7", 50);
  expect(state.members[0]).toMatchObject({
    who: host.who,
    callsign: host.callsign,
    seat: "seat-1",
    since: 50,
  });
});

test("a room seats a joiner on the side with fewer humans in its first open seat", () => {
  let state = room();
  state = changed(state, guest.who, { kind: "join", callsign: guest.callsign });
  expect(state.members.find((member) => member.who === guest.who)).toMatchObject({
    seat: "seat-2",
    ready: false,
  });
  expect(state.lobby.seats.find((seat) => seat.id === "seat-2")?.occupant).toEqual({
    kind: "human",
    who: guest.who,
  });
});

test("a full room and a started room refuse a joiner", () => {
  let state = room();
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  state = changed(state, host.who, { kind: "add_seat", side: "red" });
  state = changed(state, host.who, { kind: "add_seat", side: "red" });
  state = changed(state, host.who, { kind: "add_seat", side: "red" });
  state = changed(state, host.who, { kind: "add_seat", side: "red" });
  for (const seat of state.lobby.seats.filter((seat) => seat.occupant.kind !== "human")) {
    state = changed(state, host.who, { kind: "set_seat", seat: seat.id, to: "open" });
  }
  for (const who of ["b", "c", "d", "e", "f", "g", "h", "i", "j"]) {
    state = changed(state, who, { kind: "join", callsign: who });
  }
  expect(roomReduce(state, host.who, { kind: "add_seat", side: "blue" }, 20)).toEqual({
    refused: expect.any(String),
  });
  expect(roomReduce(state, "last", { kind: "join", callsign: "Last" }, 20)).toEqual({
    refused: expect.any(String),
  });
  const started = { ...state, phase: "battle" as const };
  expect(roomReduce(started, "last", { kind: "join", callsign: "Last" }, 20)).toEqual({
    refused: expect.any(String),
  });
});

test("a player can move to an open seat and the move clears readiness", () => {
  let state = room();
  state = changed(state, guest.who, { kind: "join", callsign: guest.callsign });
  state = changed(state, guest.who, { kind: "ready", ready: true });
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-3", to: "open" });
  state = changed(state, guest.who, { kind: "sit", seat: "seat-3" });
  expect(state.members.find((member) => member.who === guest.who)).toMatchObject({
    seat: "seat-3",
    ready: false,
  });
});

test("only the host changes room settings and seats", () => {
  const state = room();
  expect(
    roomReduce(state, guest.who, { kind: "settings", patch: { lighting: "night" } }, 20),
  ).toEqual({
    refused: expect.any(String),
  });
  expect(roomReduce(state, guest.who, { kind: "start" }, 20)).toEqual({
    refused: expect.any(String),
  });
  const added = changed(state, host.who, { kind: "add_seat", side: "blue" });
  expect(added.lobby.seats.filter((seat) => seat.side === "blue")).toHaveLength(2);
  expect(added.lobby.lighting).toBe(state.lobby.lighting);
  const open = changed(added, host.who, { kind: "set_seat", seat: "seat-3", to: "open" });
  expect(open.lobby.seats.find((seat) => seat.id === "seat-3")?.occupant).toEqual({ kind: "open" });
  const ai = changed(open, host.who, { kind: "set_seat", seat: "seat-3", to: "ai" });
  expect(ai.lobby.seats.find((seat) => seat.id === "seat-3")?.occupant).toEqual({ kind: "ai" });
  const configured = changed(added, host.who, {
    kind: "settings",
    patch: {
      factions: { blue: "europe", red: "eastern" },
      map: { type: "metro", size: "medium", region: "paris", seed: "42" },
      lighting: "golden_hour",
    },
  });
  expect(configured.lobby).toMatchObject({
    factions: { blue: "europe", red: "eastern" },
    map: { type: "metro", region: "paris" },
    lighting: "golden_hour",
  });
});

test("removing a seat removes its human but leaves every other seat id alone", () => {
  let state = room();
  state = changed(state, guest.who, { kind: "join", callsign: guest.callsign });
  state = changed(state, host.who, { kind: "add_seat", side: "red" });
  const ids = state.lobby.seats.map((seat) => seat.id).filter((id) => id !== "seat-2");
  state = changed(state, host.who, { kind: "remove_seat", seat: "seat-2" });
  expect(state.members.map((member) => member.who)).toEqual([host.who]);
  expect(state.lobby.seats.map((seat) => seat.id)).toEqual(ids);
});

test("the longest-present human becomes host when the host leaves", () => {
  let state = room();
  state = changed(state, guest.who, { kind: "join", callsign: guest.callsign }, 2);
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-3", to: "open" });
  state = changed(state, "browser-c", { kind: "join", callsign: "Charlie" }, 3);
  state = changed(state, host.who, { kind: "leave" }, 4);
  expect(state.host).toBe(guest.who);
});

test("starting needs every human ready and a fighter on each side", () => {
  let state = room();
  expect(canStart(state)).toBe(false);
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "ai" });
  state = changed(state, host.who, { kind: "ready", ready: true });
  expect(canStart(state)).toBe(true);
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "open" });
  expect(canStart(state)).toBe(false);
  expect(roomReduce(state, host.who, { kind: "start" }, 20)).toEqual({
    refused: expect.any(String),
  });
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "ai" });
  const started = roomReduce(state, host.who, { kind: "start" }, 20);
  if ("refused" in started) throw new Error(started.refused);
  expect(started.phase).toBe("starting");
  expect(roomView(started, host.who).phase).toEqual({ kind: "starting" });
});

test("the server records battle identity after the room enters starting", () => {
  let state = room();
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "ai" });
  state = changed(state, host.who, { kind: "ready", ready: true });
  const starting = changed(state, host.who, { kind: "start" });
  expect(roomView(starting, host.who).phase).toEqual({ kind: "starting" });
  const battle = recordBattle(starting, {
    address: "http://127.0.0.1:1234",
    id: "battle-42",
    mapIdentity: "map-42",
  });
  expect(roomView(battle, host.who).phase).toMatchObject({
    kind: "battle",
    ticket: { battle: "battle-42", mapIdentity: "map-42" },
  });
});

test("each human gets the battle player for their own seat and side", () => {
  let state = room();
  state = changed(state, guest.who, { kind: "join", callsign: guest.callsign });
  state = changed(state, host.who, { kind: "ready", ready: true });
  state = changed(state, guest.who, { kind: "ready", ready: true });
  state = changed(state, host.who, { kind: "start" });
  state = recordBattle(state, {
    address: "http://127.0.0.1:1234",
    id: "battle-42",
    mapIdentity: "map-42",
  });
  const hostPhase = roomView(state, host.who).phase;
  const guestPhase = roomView(state, guest.who).phase;
  expect(hostPhase).toMatchObject({ kind: "battle", ticket: { side: "blue", player: "blue-1" } });
  expect(guestPhase).toMatchObject({ kind: "battle", ticket: { side: "red", player: "red-1" } });
});

test("leaving after start keeps the member's seat and ticket for their return, and marks them away", () => {
  let state = room();
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "ai" });
  state = changed(state, host.who, { kind: "ready", ready: true });
  state = changed(state, host.who, { kind: "start" });
  state = recordBattle(state, {
    address: "http://127.0.0.1:1234",
    id: "battle-42",
    mapIdentity: "map-42",
  });
  expect(deserted(state)).toBe(false);
  const left = changed(state, host.who, { kind: "leave" }, 30);
  expect(left.members.map((member) => member.away)).toEqual([true]);
  expect(deserted(left)).toBe(true);
  expect(roomView(left, host.who).phase).toMatchObject({
    kind: "battle",
    ticket: { battle: "battle-42", player: "blue-1" },
  });
  // Opening the room again is the way back to the same seat.
  const back = changed(left, host.who, { kind: "join", callsign: host.callsign }, 40);
  expect(back).toEqual(state);
  expect(deserted(back)).toBe(false);
});

test("a battle one human has left is not deserted while another is still in it", () => {
  let state = changed(room(), guest.who, { kind: "join", callsign: guest.callsign });
  state = changed(state, host.who, { kind: "ready", ready: true });
  state = changed(state, guest.who, { kind: "ready", ready: true });
  expect(deserted(state)).toBe(false);
  state = changed(state, host.who, { kind: "start" });
  state = changed(state, host.who, { kind: "leave" });
  expect(deserted(state)).toBe(false);
  expect(deserted(changed(state, guest.who, { kind: "leave" }))).toBe(true);
});

test("a human-held seat cannot be changed with set_seat", () => {
  const state = room();
  expect(roomReduce(state, host.who, { kind: "set_seat", seat: "seat-1", to: "ai" }, 20)).toEqual({
    refused: expect.any(String),
  });
  expect(state.members).toHaveLength(1);
  expect(state.lobby.seats[0]?.occupant).toEqual({ kind: "human", who: host.who });
});

test("settings use known values, the lobby size floor and the seed setter", () => {
  let state = changed(room(), host.who, { kind: "add_seat", side: "blue" });
  expect(state.lobby.map.size).toBe("medium");
  state = changed(state, host.who, { kind: "settings", patch: { map: { size: "small" } } });
  expect(state.lobby.map.size).toBe("medium");
  state = changed(state, host.who, {
    kind: "settings",
    patch: { map: { region: null, seed: "99" }, lighting: "random" },
  });
  expect(state.lobby.map.region).toBeUndefined();
  expect(state.lobby.map.seed).toBe("99");
  expect(state.lobby.lighting).toBe("random");
  expect(
    roomReduce(
      state,
      host.who,
      { kind: "settings", patch: { map: { type: "not-a-map" } } } as never,
      20,
    ),
  ).toEqual({ refused: expect.any(String) });
  expect(
    roomReduce(state, host.who, { kind: "settings", patch: { lighting: "night" } } as never, 20),
  ).toEqual({ refused: expect.any(String) });
});

test("repeating an already-held state succeeds", () => {
  let state = room();
  expect(roomReduce(state, host.who, { kind: "sit", seat: "seat-1" }, 20)).toEqual(state);
  expect(roomReduce(state, "missing", { kind: "leave" }, 20)).toEqual(state);
  state = changed(state, host.who, { kind: "ready", ready: true });
  expect(roomReduce(state, host.who, { kind: "ready", ready: true }, 20)).toEqual(state);
});

test("a room view exposes callsigns but never browser ids", () => {
  const view = roomView(room(), host.who);
  expect(JSON.stringify(view)).not.toContain(host.who);
  expect(view).toMatchObject({ code: "TANGO-4821", visibility: "public", you: "seat-1" });
});

test("room setup and its lobby round trip produce one battle request", () => {
  let state = room();
  state = changed(state, host.who, { kind: "ready", ready: true });
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "ai" });
  const setup = battleSetup(state);
  const roundTrip = fromBattleSetup(setup);
  const generator = {
    template_catalogue_json: () => JSON.stringify({ hash: "templates" }),
    map_generator_version: () => "generator",
  };
  const data: SetupData = {
    generator: generator as unknown as SetupData["generator"],
    presets: JSON.stringify({ revision: "presets" }),
    templates: "templates",
    limits: { max_authored_parts: 1, max_bay_positions: 1, max_ground_points: 1 },
  };
  expect(battleRequest(roundTrip, data)).toEqual(battleRequest(state.lobby, data));
  expect(setup.seats.map((seat) => seat.player)).toEqual(["blue-1", "red-1"]);
});

test("a room code has six unambiguous characters at either end of the draw", () => {
  expect(roomCode(() => 0)).toBe(ROOM_CODE_ALPHABET[0].repeat(6));
  expect(roomCode(() => 0.999999)).toBe(ROOM_CODE_ALPHABET.at(-1)!.repeat(6));
});

test("open seats disappear from a battle setup", () => {
  let state = room();
  state = changed(state, host.who, { kind: "set_seat", seat: "seat-2", to: "ai" });
  state = changed(state, host.who, { kind: "add_seat", side: "blue" });
  const open = setOccupant(state.lobby, "seat-3", { kind: "open" });
  expect(battleSetup({ ...state, lobby: open }).seats).toEqual([
    { player: "blue-1", side: "blue", controller: "human" },
    { player: "red-1", side: "red", controller: "active_ai" },
  ]);
});
