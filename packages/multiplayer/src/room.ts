import {
  add,
  preset,
  remove,
  reroll,
  setFaction,
  setLighting,
  setOccupant,
  setRegion,
  setSize,
  setType,
  type LobbyState,
} from "../../skirmish-setup/src/lobby";
import { FACTIONS, type Faction } from "../../scene-assets/src/units";
import {
  canonicalSeed,
  MAP_REGIONS,
  MAP_SIZES,
  MAP_TYPES,
  type MapSize,
  type MapType,
} from "../../../web/src/maps/source";
import presentation from "../../../fixtures/presentation.json";
import {
  battlePlayers,
  type Occupant,
  type Seat,
  type SeatId,
} from "../../skirmish-setup/src/seats";
import { toBattleSetup } from "./setup";
import type {
  BattleSetup,
  BattleTicket,
  RoomAction,
  RoomCode,
  RoomSettingsPatch,
  RoomView,
} from "./wire";

export interface Member {
  who: string;
  callsign: string;
  seat: SeatId;
  ready: boolean;
  since: number;
}

export type RoomPhase = "lobby" | "starting" | "battle";
export interface RoomBattle {
  address?: string;
  id: string;
  mapIdentity: string;
}
export interface Room {
  code: RoomCode;
  visibility: "public" | "private";
  lobby: LobbyState;
  members: Member[];
  host: string;
  phase: RoomPhase;
  battle: RoomBattle | null;
}

export type RoomRefusal = { refused: string };
export type RoomRequest = RoomAction | { kind: "join"; callsign: string };
const SIDES = ["blue", "red"] as const;
/** Omit ambiguous letters/digits even when a code is read in lower case. */
export const ROOM_CODE_ALPHABET = "ACDEFHJKMNPRTUVWXY347";
export function normalizeRoomCode(text: string): RoomCode {
  return text.replace(/[\s-]/g, "").toUpperCase();
}
function isKnownLighting(value: unknown): value is string {
  return ["random", presentation.light_preset, ...Object.keys(presentation.light_presets)].includes(
    value as string,
  );
}

/** A short shareable room id; knowing it is enough to join. */
export function roomCode(random: () => number): RoomCode {
  const draw = () =>
    Math.min(
      ROOM_CODE_ALPHABET.length - 1,
      Math.max(0, Math.floor(random() * ROOM_CODE_ALPHABET.length)),
    );
  return Array.from({ length: 6 }, () => ROOM_CODE_ALPHABET[draw()]).join("");
}

/** A new room: a 1v1 on the map `seed` makes, its host on Blue and Red's
 *  seat open. The seed is the caller's, so each room opens on its own map. */
export function createRoom(
  host: { who: string; callsign: string },
  visibility: Room["visibility"],
  code: RoomCode,
  seed: string,
  now: number,
): Room {
  const initial = reroll(preset("1v1"), seed);
  const hostSeat = initial.seats[0]!.id;
  const otherSeat = initial.seats.find((seat) => seat.id !== hostSeat);
  let lobby = setOccupant(initial, hostSeat, { kind: "human", who: host.who });
  if (otherSeat) lobby = setOccupant(lobby, otherSeat.id, { kind: "open" });
  return {
    code,
    visibility,
    lobby,
    members: [{ ...host, seat: hostSeat, ready: false, since: now }],
    host: host.who,
    phase: "lobby",
    battle: null,
  };
}

function refusal(reason: string): RoomRefusal {
  return { refused: reason };
}

function memberOf(room: Room, who: string): Member | undefined {
  return room.members.find((member) => member.who === who);
}

function seatOf(room: Room, who: string): Seat | undefined {
  const member = memberOf(room, who);
  return member && room.lobby.seats.find((seat) => seat.id === member.seat);
}

function humanCount(room: Room, side: (typeof SIDES)[number]): number {
  return room.members.reduce(
    (count, member) => count + (seatOf(room, member.who)?.side === side ? 1 : 0),
    0,
  );
}

function nextHost(members: readonly Member[]): string {
  return [...members].sort((a, b) => a.since - b.since)[0]?.who ?? "";
}

function withoutMember(room: Room, who: string, to: Occupant = { kind: "open" }): Room {
  const member = memberOf(room, who);
  const lobby = member ? setOccupant(room.lobby, member.seat, to) : room.lobby;
  const members = room.members.filter((candidate) => candidate.who !== who);
  return { ...room, lobby, members, host: room.host === who ? nextHost(members) : room.host };
}

function isKnownMapType(value: unknown): value is MapType {
  return typeof value === "string" && MAP_TYPES.includes(value as MapType);
}

function isKnownMapSize(value: unknown): value is MapSize {
  return typeof value === "string" && MAP_SIZES.includes(value as MapSize);
}

function isKnownFaction(value: unknown): value is Faction {
  return typeof value === "string" && FACTIONS.includes(value as Faction);
}

function validKeys(value: object, keys: readonly string[]): boolean {
  return Object.keys(value).every((key) => keys.includes(key));
}

function validateMap(value: unknown): RoomRefusal | null {
  if (value === undefined) return null;
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return refusal("unknown map setting");
  const map = value as NonNullable<RoomSettingsPatch["map"]>;
  if (!validKeys(map, ["type", "size", "region", "seed"])) return refusal("unknown map setting");
  if (map.type !== undefined && !isKnownMapType(map.type)) return refusal("unknown map type");
  if (map.size !== undefined && !isKnownMapSize(map.size)) return refusal("unknown map size");
  if (map.region !== undefined && map.region !== null && !MAP_REGIONS.includes(map.region))
    return refusal("unknown map region");
  if (map.seed !== undefined && (typeof map.seed !== "string" || canonicalSeed(map.seed) === null))
    return refusal("unknown map seed");
  return null;
}

function validateFactions(value: unknown): RoomRefusal | null {
  if (value === undefined) return null;
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !validKeys(value, ["blue", "red"]) ||
    Object.values(value).some((faction) => !isKnownFaction(faction))
  )
    return refusal("unknown faction");
  return null;
}

function validateSettings(patch: unknown): RoomRefusal | null {
  if (typeof patch !== "object" || patch === null || Array.isArray(patch))
    return refusal("unknown setting");
  if (!validKeys(patch, ["map", "factions", "lighting"])) return refusal("unknown setting");
  const invalidMap = validateMap((patch as RoomSettingsPatch).map);
  if (invalidMap) return invalidMap;
  const invalidFactions = validateFactions((patch as RoomSettingsPatch).factions);
  if (invalidFactions) return invalidFactions;
  const lighting = (patch as RoomSettingsPatch).lighting;
  if (lighting !== undefined && !isKnownLighting(lighting)) return refusal("unknown lighting");
  return null;
}

function applySettings(room: Room, patch: RoomSettingsPatch): Room | RoomRefusal {
  const invalid = validateSettings(patch);
  if (invalid) return invalid;
  const map = patch.map;

  let lobby = room.lobby;
  if (patch.factions?.blue !== undefined) lobby = setFaction(lobby, "blue", patch.factions.blue);
  if (patch.factions?.red !== undefined) lobby = setFaction(lobby, "red", patch.factions.red);
  if (map?.type !== undefined) lobby = setType(lobby, map.type);
  if (map?.size !== undefined) lobby = setSize(lobby, map.size);
  if (map && Object.hasOwn(map, "region"))
    lobby = setRegion(lobby, map.region === null ? undefined : map.region);
  if (map?.seed !== undefined) lobby = reroll(lobby, canonicalSeed(map.seed)!);
  if (patch.lighting !== undefined) lobby = setLighting(lobby, patch.lighting);
  return { ...room, lobby };
}

function move(room: Room, who: string, target: SeatId): Room | RoomRefusal {
  const member = memberOf(room, who);
  if (!member) return refusal("you are not in this room");
  if (member.seat === target) return room;
  const destination = room.lobby.seats.find((seat) => seat.id === target);
  if (!destination) return refusal("that seat does not exist");
  if (destination.occupant.kind !== "open") return refusal("that seat is occupied");
  const lobby = setOccupant(setOccupant(room.lobby, member.seat, { kind: "open" }), target, {
    kind: "human",
    who,
  });
  const members = room.members.map((candidate) =>
    candidate.who === who ? { ...candidate, seat: target, ready: false } : candidate,
  );
  return { ...room, lobby, members };
}

/** Every room request is a value transition; refusal never throws. */
export function roomReduce(
  room: Room,
  by: string,
  action: RoomRequest,
  now: number,
): Room | RoomRefusal {
  if (action.kind === "join") {
    if (memberOf(room, by)) return room;
    if (room.phase !== "lobby") return refusal("this room has started");
    const sideOrder = [...SIDES].sort((a, b) => humanCount(room, a) - humanCount(room, b));
    const destination = sideOrder
      .flatMap((side) => room.lobby.seats.filter((seat) => seat.side === side))
      .find((seat) => seat.occupant.kind === "open");
    if (!destination) return refusal("this room is full");
    const lobby = setOccupant(room.lobby, destination.id, { kind: "human", who: by });
    return {
      ...room,
      lobby,
      members: [
        ...room.members,
        { who: by, callsign: action.callsign, seat: destination.id, ready: false, since: now },
      ],
    };
  }

  if (action.kind === "leave") {
    if (!memberOf(room, by) || room.phase !== "lobby") return room;
    return withoutMember(room, by);
  }

  if (room.phase !== "lobby") return refusal("this room has started");
  const member = memberOf(room, by);
  if (!member) return refusal("you are not in this room");

  if (action.kind === "sit") return move(room, by, action.seat);
  if (action.kind === "ready") {
    if (member.ready === action.ready) return room;
    return {
      ...room,
      members: room.members.map((candidate) =>
        candidate.who === by ? { ...candidate, ready: action.ready } : candidate,
      ),
    };
  }

  if (by !== room.host) return refusal("only the host can change the room");
  if (action.kind === "settings") return applySettings(room, action.patch);
  if (action.kind === "add_seat") {
    try {
      return { ...room, lobby: add(room.lobby, action.side) };
    } catch {
      return refusal("that side has no more seats");
    }
  }
  if (action.kind === "remove_seat") {
    if (!room.lobby.seats.some((seat) => seat.id === action.seat))
      return refusal("that seat does not exist");
    const removed = room.lobby.seats.find((seat) => seat.id === action.seat)!;
    try {
      const next = { ...room, lobby: remove(room.lobby, action.seat) };
      return removed.occupant.kind === "human" ? withoutMember(next, removed.occupant.who) : next;
    } catch {
      return refusal("each side needs a seat");
    }
  }
  if (action.kind === "set_seat") {
    const seat = room.lobby.seats.find((candidate) => candidate.id === action.seat);
    if (!seat) return refusal("that seat does not exist");
    if (seat.occupant.kind === "human") return refusal("someone is sitting there");
    const occupant: Occupant = action.to === "open" ? { kind: "open" } : { kind: "ai" };
    return { ...room, lobby: setOccupant(room.lobby, action.seat, occupant) };
  }
  if (action.kind === "start") {
    if (!canStart(room)) return refusal("every human must be ready and both sides need a fighter");
    return { ...room, phase: "starting" };
  }
  return refusal("unknown room action");
}

export function canStart(room: Room): boolean {
  return (
    room.phase === "lobby" &&
    room.members.every((member) => member.ready) &&
    SIDES.every((side) =>
      room.lobby.seats.some((seat) => seat.side === side && seat.occupant.kind !== "open"),
    )
  );
}

/** Records the server-created battle once its id and map identity are known. */
export function recordBattle(room: Room, battle: RoomBattle): Room {
  if (room.phase === "battle") return room;
  if (room.phase !== "starting") return room;
  return { ...room, phase: "battle", battle: { ...battle } };
}

export function battleSetup(room: Room): BattleSetup {
  return toBattleSetup(room.lobby, "skirmish");
}

function ticketFor(room: Room, who: string): BattleTicket | null {
  if (!room.battle) return null;
  const member = memberOf(room, who);
  if (!member) return null;
  const setup = battleSetup(room);
  const { playerOf } = battlePlayers(room.lobby.seats);
  const playerId = playerOf[member.seat];
  const player = setup.seats.find((seat) => seat.player === playerId);
  if (!player) return null;
  return {
    callsigns: Object.fromEntries(
      room.members.map((member) => [playerOf[member.seat]!, member.callsign]),
    ),
    battle: room.battle.id,
    ...(room.battle.address && { address: room.battle.address }),
    setup,
    mapIdentity: room.battle.mapIdentity,
    side: player.side,
    player: player.player,
  };
}

export function roomView(room: Room, who: string): RoomView {
  const you = memberOf(room, who)?.seat ?? "";
  const seats = room.lobby.seats.map((seat) => {
    const occupant = seat.occupant;
    if (occupant.kind !== "human") return { id: seat.id, side: seat.side, occupant };
    const member = room.members.find((candidate) => candidate.who === occupant.who);
    return {
      id: seat.id,
      side: seat.side,
      occupant: {
        kind: "human" as const,
        callsign: member?.callsign ?? "",
        ready: member?.ready ?? false,
        host: occupant.who === room.host,
      },
    };
  });
  const ticket = room.phase === "battle" ? ticketFor(room, who) : null;
  const phase =
    room.phase === "lobby"
      ? ({ kind: "lobby" } as const)
      : room.phase === "starting" || ticket === null
        ? ({ kind: "starting" } as const)
        : ({ kind: "battle", ticket } as const);
  return {
    code: room.code,
    visibility: room.visibility,
    seats,
    settings: { factions: room.lobby.factions, map: room.lobby.map, lighting: room.lobby.lighting },
    you,
    canStart: canStart(room),
    phase,
  };
}
