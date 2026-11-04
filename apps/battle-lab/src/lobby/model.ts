import { DEFAULT_BATTLE_SEED } from "@apps/battle-lab/src/lobby/defaults";
import type { Player, PrepareBattleRequest } from "@web/battle/prepare/protocol";
import {
  MAP_SIZES,
  MAP_TYPES,
  newSeed,
  type MapChoice,
  type MapGenerator,
  type MapSize,
  type MapType,
} from "@web/maps/source";
import type { Faction } from "@packages/scene-assets/src/units";
import {
  DEFAULT_FACTION,
  entryCountForPlayers,
  MAX_PLAYERS_PER_SIDE,
  playersFor,
  sizeTier,
  skirmishPlayers,
  skirmishRequest,
  type SkirmishSeat,
  type SkirmishSide,
} from "../skirmishSetup";
import { battleHref } from "../battleLinks";

export type LobbySide = SkirmishSide;
export type LobbyPreset = "1v1" | "3v3" | "5v5";

/** The lobby's match. Its players are always the battle's players
 *  (`skirmishPlayers`), so what it shows, previews and links agree. */
export interface LobbyState {
  players: Player[];
  map: MapChoice;
  battleSeed: number;
  sizeFloor: MapSize;
}

const DEFAULT_MAP: MapChoice = { type: "mixed", size: "small", profile: "skirmish", seed: "1" };

const countForPreset: Record<LobbyPreset, number> = { "1v1": 1, "3v3": 3, "5v5": 5 };

function playerTarget(target: Player | string): string {
  return typeof target === "string" ? target : target.id;
}

function sideCount(players: readonly Player[], side: LobbySide): number {
  return players.filter((player) => player.side === side).length;
}

/** `seats` with exactly one human, on Blue: the first Blue human, else the
 *  first Blue player. */
function oneBlueHuman<T extends SkirmishSeat>(seats: readonly T[]): T[] {
  const human =
    seats.find((seat) => seat.side === "blue" && seat.controller === "human") ??
    seats.find((seat) => seat.side === "blue");
  if (!human) throw new Error("lobby must have a Blue player");
  return seats.map((seat) =>
    seat === human
      ? { ...seat, controller: "human" }
      : seat.controller === "human"
        ? { ...seat, controller: "active_ai" }
        : seat,
  );
}

const minimumSize = (players: readonly Player[]) => sizeTier(players).size as MapSize;

function sizeRank(size: MapSize): number {
  return MAP_SIZES.indexOf(size);
}

function ratchetSize(players: readonly Player[], size: MapSize): MapSize {
  const floor = minimumSize(players);
  return sizeRank(size) >= sizeRank(floor) ? size : floor;
}

/** The lobby with `seats` as its players: the floor follows them both ways
 *  (D18), and the size rises to it but never falls with it. */
function withPlayers(state: LobbyState, seats: readonly SkirmishSeat[]): LobbyState {
  const players = skirmishPlayers(oneBlueHuman(seats));
  return {
    ...state,
    players,
    sizeFloor: minimumSize(players),
    map: { ...state.map, size: ratchetSize(players, state.map.size) },
  };
}

/** A fresh `kind` match; applied to `from`, it reseats that lobby, keeping its
 *  map type, region and seed and each side's faction, with the size at the
 *  preset's floor. */
export function preset(kind: LobbyPreset, from?: LobbyState): LobbyState {
  const count = countForPreset[kind];
  const players = playersFor(count, count).map((player) =>
    from ? { ...player, faction: sideFaction(from, player.side as LobbySide) } : player,
  );
  const floor = minimumSize(players);
  return {
    players,
    map: { ...(from?.map ?? DEFAULT_MAP), size: floor },
    battleSeed: from?.battleSeed ?? Number(DEFAULT_BATTLE_SEED),
    sizeFloor: floor,
  };
}

/** The faction `side` fields in `state` (one per side). */
export function sideFaction(state: LobbyState, side: LobbySide): Faction {
  return state.players.find((player) => player.side === side)?.faction ?? DEFAULT_FACTION[side];
}

export function add(state: LobbyState, side: LobbySide): LobbyState {
  if (sideCount(state.players, side) >= MAX_PLAYERS_PER_SIDE)
    throw new Error(`maximum of ${MAX_PLAYERS_PER_SIDE} players per side`);
  return withPlayers(state, [
    ...state.players,
    { side, faction: sideFaction(state, side), controller: "active_ai" },
  ]);
}

export function remove(state: LobbyState, target: Player | string): LobbyState {
  const id = playerTarget(target);
  const player = state.players.find((candidate) => candidate.id === id);
  if (!player) return state;
  if (player.controller === "human") throw new Error("cannot remove the Human player");
  if (sideCount(state.players, player.side as LobbySide) <= 1)
    throw new Error("each side must keep at least one player");
  return withPlayers(
    state,
    state.players.filter((candidate) => candidate.id !== id),
  );
}

export function seatHuman(state: LobbyState, target: Player | string): LobbyState {
  const id = playerTarget(target);
  const selected = state.players.find((player) => player.id === id);
  if (selected?.side === "red") throw new Error("the Human player must be on Blue");
  const humanId = selected?.id;
  if (!humanId) return state;
  return withPlayers(
    state,
    state.players.map((player) =>
      player.id === humanId
        ? { ...player, controller: "human" }
        : player.controller === "human"
          ? { ...player, controller: "active_ai" }
          : player,
    ),
  );
}

export function setFaction(state: LobbyState, side: LobbySide, faction: Faction): LobbyState {
  return {
    ...state,
    players: state.players.map((player) =>
      player.side === side ? { ...player, faction } : player,
    ),
  };
}

export function setSize(state: LobbyState, size: MapSize): LobbyState {
  if (!MAP_SIZES.includes(size)) throw new Error(`unknown map size: ${size}`);
  const floor = minimumSize(state.players);
  return {
    ...state,
    sizeFloor: floor,
    map: { ...state.map, size: ratchetSize(state.players, size) },
  };
}

export function setType(state: LobbyState, type: MapType): LobbyState {
  if (!MAP_TYPES.includes(type)) throw new Error(`unknown map type: ${type}`);
  return { ...state, map: { ...state.map, type } };
}

/** Build the map in `region`'s style, or leave the region to the seed. */
export function setRegion(state: LobbyState, region: string | undefined): LobbyState {
  const { region: _, ...map } = state.map;
  return { ...state, map: region === undefined ? map : { ...map, region } };
}

export function reroll(state: LobbyState, seed?: string): LobbyState {
  const nextSeed = seed ?? newSeed();
  return { ...state, map: { ...state.map, seed: nextSeed } };
}

function mapInputs(state: LobbyState): {
  type: MapType;
  size: MapSize;
  region?: string;
  seed: string;
  entries: number;
} {
  return {
    type: state.map.type,
    size: state.map.size,
    ...(state.map.region === undefined ? {} : { region: state.map.region }),
    seed: state.map.seed,
    entries: entryCountForPlayers(state.players),
  };
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

/** The request the lobby previews and Deploy prepares, at each candidate
 *  map seed the preview's admission draws, pinned to `generator`'s build. */
export function lobbyCandidate(generator: MapGenerator, state: LobbyState) {
  return (seed: string): PrepareBattleRequest =>
    skirmishRequest(generator, {
      players: state.players,
      map: reroll(state, seed).map,
      battleSeed: state.battleSeed,
    });
}

/** The address of the battle the lobby sets up, without its `/battle` path. */
export function toUrl(state: LobbyState): string {
  return battleHref(state.map, state.players, state.battleSeed).replace(/^\/battle/, "");
}
