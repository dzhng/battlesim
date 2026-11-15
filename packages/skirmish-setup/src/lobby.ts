import type {
  CompileLimits,
  MapChoice,
  MapGenerator,
  MapSize,
  MapType,
} from "../../../web/src/maps/source";
import { MAP_SIZES, MAP_TYPES, newSeed, generationRequest } from "../../../web/src/maps/source";
import type { Player, PrepareBattleRequest } from "../../../web/src/battle/prepare/protocol";
import type { Faction } from "../../scene-assets/src/units";
import presentation from "../../../fixtures/presentation.json";
import {
  battlePlayers,
  DEFAULT_FACTION,
  entryCountForPlayers,
  MAX_PLAYERS_PER_SIDE,
  sizeTier,
  type Occupant,
  type Seat,
  type SeatId,
  type SkirmishSide,
} from "./seats";

export type LobbySide = SkirmishSide;
export type LobbyPreset = "1v1" | "3v3" | "5v5";
export interface LobbyState {
  seats: Seat[];
  nextSeat: number;
  factions: Record<SkirmishSide, Faction>;
  map: MapChoice;
  battleSeed: number;
  lighting: string;
  rules?: PrepareBattleRequest["rules"];
}
export interface SetupData {
  generator: MapGenerator;
  presets: string;
  templates: string;
  limits: CompileLimits;
}

const DEFAULT_MAP: MapChoice = { type: "mixed", size: "small", profile: "skirmish", seed: "1" };
export const DEFAULT_BATTLE_SEED = 20260925;
export const DEFAULT_LIGHTING = presentation.light_preset;
const countForPreset: Record<LobbyPreset, number> = { "1v1": 1, "3v3": 3, "5v5": 5 };

function filled(state: LobbyState): Seat[] {
  return state.seats.filter((seat) => seat.occupant.kind !== "open");
}
function seatId(number: number): SeatId {
  return `seat-${number}`;
}
function floorFor(state: LobbyState): MapSize {
  return sizeTier(filled(state).map((seat) => ({ side: seat.side }))).size;
}
function sizeRank(size: MapSize): number {
  return MAP_SIZES.indexOf(size);
}
function ratchetSize(state: LobbyState, size: MapSize): MapSize {
  const floor = floorFor(state);
  return sizeRank(size) >= sizeRank(floor) ? size : floor;
}
function withSeats(
  state: LobbyState,
  seats: readonly Seat[],
  nextSeat = state.nextSeat,
): LobbyState {
  const next = { ...state, seats: [...seats] };
  return { ...next, nextSeat, map: { ...next.map, size: ratchetSize(next, next.map.size) } };
}
function targetId(target: Seat | SeatId): SeatId {
  return typeof target === "string" ? target : target.id;
}

export function preset(kind: LobbyPreset, from?: LobbyState): LobbyState {
  const count = countForPreset[kind];
  const factions = from?.factions ?? DEFAULT_FACTION;
  let nextSeat = from?.nextSeat ?? 1;
  const seats = (["blue", "red"] as const).flatMap((side) =>
    Array.from({ length: count }, (_, index) => {
      const occupant: Occupant =
        side === "blue" && index === 0 ? { kind: "human", who: "human" } : { kind: "ai" };
      return { id: seatId(nextSeat++), side, occupant };
    }),
  );
  const state: LobbyState = {
    seats,
    nextSeat,
    factions: { blue: factions.blue, red: factions.red },
    map: { ...(from?.map ?? DEFAULT_MAP), size: "small" },
    battleSeed: from?.battleSeed ?? DEFAULT_BATTLE_SEED,
    lighting: from?.lighting ?? DEFAULT_LIGHTING,
  };
  const floor = floorFor(state);
  return { ...state, map: { ...state.map, size: floor } };
}

export function sideFaction(state: LobbyState, side: LobbySide): Faction {
  return state.factions[side];
}

export function add(state: LobbyState, side: LobbySide): LobbyState {
  const count = state.seats.filter((seat) => seat.side === side).length;
  if (count >= MAX_PLAYERS_PER_SIDE)
    throw new Error(`maximum of ${MAX_PLAYERS_PER_SIDE} players per side`);
  return withSeats(
    state,
    [...state.seats, { id: seatId(state.nextSeat), side, occupant: { kind: "ai" } }],
    state.nextSeat + 1,
  );
}

export function remove(state: LobbyState, target: Seat | SeatId): LobbyState {
  const id = targetId(target);
  const seat = state.seats.find((candidate) => candidate.id === id);
  if (!seat) return state;
  if (state.seats.filter((candidate) => candidate.side === seat.side).length <= 1)
    throw new Error("each side must keep at least one seat");
  return withSeats(
    state,
    state.seats.filter((candidate) => candidate.id !== id),
  );
}

export function seatHuman(state: LobbyState, target: Seat | SeatId, who = "human"): LobbyState {
  const id = targetId(target);
  return setOccupant(state, id, { kind: "human", who });
}

/** Who fills a seat decides how many will fight, so the size follows. */
export function setOccupant(state: LobbyState, id: SeatId, occupant: Occupant): LobbyState {
  return withSeats(
    state,
    state.seats.map((seat) => (seat.id === id ? { ...seat, occupant } : seat)),
  );
}

export function setFaction(state: LobbyState, side: LobbySide, faction: Faction): LobbyState {
  return { ...state, factions: { ...state.factions, [side]: faction } };
}

export function setSize(state: LobbyState, size: MapSize): LobbyState {
  if (!MAP_SIZES.includes(size)) throw new Error(`unknown map size: ${size}`);
  return {
    ...state,
    map: { ...state.map, size: ratchetSize(state, size) },
  };
}

export function setType(state: LobbyState, type: MapType): LobbyState {
  if (!MAP_TYPES.includes(type)) throw new Error(`unknown map type: ${type}`);
  return { ...state, map: { ...state.map, type } };
}

export function setRegion(state: LobbyState, region: string | undefined): LobbyState {
  const { region: _, ...map } = state.map;
  return { ...state, map: region === undefined ? map : { ...map, region } };
}

export function setLighting(state: LobbyState, lighting: string): LobbyState {
  return { ...state, lighting };
}

export function reroll(state: LobbyState, seed?: string): LobbyState {
  return { ...state, map: { ...state.map, seed: seed ?? newSeed() } };
}

export function isMapInputChange(before: LobbyState, after: LobbyState): boolean {
  const a = mapInputs(before);
  const b = mapInputs(after);
  return (
    a.type !== b.type ||
    a.size !== b.size ||
    a.region !== b.region ||
    a.seed !== b.seed ||
    a.entries !== b.entries
  );
}
function mapInputs(state: LobbyState) {
  return {
    type: state.map.type,
    size: state.map.size,
    region: state.map.region,
    seed: state.map.seed,
    entries: entryCountForPlayers(filled(state).map((seat) => ({ side: seat.side }))),
  };
}

export function battleRequest(lobby: LobbyState, data: SetupData): PrepareBattleRequest {
  const { players } = battlePlayers(lobby.seats);
  const factioned = players.map((player) => ({
    ...player,
    faction: lobby.factions[player.side as LobbySide],
  }));
  return {
    map_source: {
      kind: "generated",
      request: generationRequest(
        data.generator,
        {
          ...lobby.map,
          profile: "skirmish",
          entries_per_side: entryCountForPlayers(players),
        },
        { presets: data.presets, templates: data.templates },
        data.limits,
      ),
    },
    players: factioned,
    battle_seed: lobby.battleSeed,
    ...(lobby.rules !== undefined && { rules: lobby.rules }),
  };
}

export function sizeFloor(state: LobbyState): MapSize {
  return floorFor(state);
}

export function lobbyPlayers(state: LobbyState): Player[] {
  const { players } = battlePlayers(state.seats);
  return players.map((player) => ({
    ...player,
    faction: state.factions[player.side as LobbySide],
  }));
}
