import game from "@fixtures/game.json";
import type { Player, PrepareBattleRequest, ControllerKind } from "@web/battle/prepare/protocol";
import {
  MAP_SIZES,
  MAP_TYPES,
  newSeed,
  type GenerationRequest,
  type MapChoice,
  type MapSize,
  type MapType,
} from "@web/maps/source";
import type { Faction } from "@packages/scene-assets/src/units";

export type LobbySide = "blue" | "red";
export type LobbyPreset = "1v1" | "3v3" | "5v5";

export interface LobbyState {
  players: Player[];
  map: MapChoice;
  battleSeed: number;
  sizeFloor: MapSize;
}

export const MAX_PLAYERS_PER_SIDE = 5;
const SIDES: readonly LobbySide[] = ["blue", "red"];
const DEFAULT_MAP: MapChoice = { type: "mixed", size: "small", profile: "skirmish", seed: "1" };

const countForPreset: Record<LobbyPreset, number> = { "1v1": 1, "3v3": 3, "5v5": 5 };

function factionFor(side: LobbySide): Faction {
  return side === "blue" ? "us" : "eastern";
}

function makePlayer(side: LobbySide, index: number, human = false): Player {
  return {
    id: `${side}-${index}`,
    side,
    faction: factionFor(side),
    controller: human ? "human" : "active_ai",
    human_capable: human,
  };
}

function playerTarget(target: Player | string): string {
  return typeof target === "string" ? target : target.id;
}

function sideCount(players: readonly Player[], side: LobbySide): number {
  return players.filter((player) => player.side === side).length;
}

function enforceHumanInvariant(players: Player[]): Player[] {
  const blue =
    players.find((player) => player.side === "blue" && player.controller === "human") ??
    players.find((player) => player.side === "blue");
  if (!blue) throw new Error("lobby must have a Blue player");
  return players.map((player) =>
    player.id === blue.id
      ? { ...player, controller: "human", human_capable: true }
      : player.controller === "human"
        ? { ...player, controller: "active_ai" }
        : player,
  );
}

function orderedPlayers(players: Player[]): Player[] {
  const sideOrder: Record<Player["side"], number> = { blue: 0, red: 1, neutral: 2 };
  return [...players].sort((a, b) => {
    const side = sideOrder[a.side] - sideOrder[b.side];
    return side || a.id.localeCompare(b.id, undefined, { numeric: true });
  });
}

function minimumSize(players: readonly Player[]): MapSize {
  const largestSide = Math.max(sideCount(players, "blue"), sideCount(players, "red"));
  return largestSide >= 5
    ? "xl"
    : largestSide >= 3
      ? "large"
      : largestSide === 2
        ? "medium"
        : "small";
}

function sizeRank(size: MapSize): number {
  return MAP_SIZES.indexOf(size);
}

function ratchetSize(players: readonly Player[], size: MapSize): MapSize {
  const floor = minimumSize(players);
  return sizeRank(size) >= sizeRank(floor) ? size : floor;
}

function entryCount(players: number): number {
  return players <= 1 ? 1 : players === 2 ? 2 : 3;
}

function withPlayers(state: LobbyState, players: Player[]): LobbyState {
  const floor =
    sizeRank(state.sizeFloor) >= sizeRank(minimumSize(players))
      ? state.sizeFloor
      : minimumSize(players);
  return {
    ...state,
    players: orderedPlayers(enforceHumanInvariant(players)),
    sizeFloor: floor,
    map: {
      ...state.map,
      size: sizeRank(state.map.size) >= sizeRank(floor) ? state.map.size : floor,
    },
  };
}

export function preset(kind: LobbyPreset): LobbyState {
  const count = countForPreset[kind];
  const players = SIDES.flatMap((side) =>
    Array.from({ length: count }, (_, index) =>
      makePlayer(side, index + 1, side === "blue" && index === 0),
    ),
  );
  return {
    players,
    map: { ...DEFAULT_MAP, size: ratchetSize(players, DEFAULT_MAP.size) },
    battleSeed: Number(game.seed),
    sizeFloor: minimumSize(players),
  };
}

export function add(state: LobbyState, side: LobbySide): LobbyState {
  if (sideCount(state.players, side) >= MAX_PLAYERS_PER_SIDE)
    throw new Error("maximum of five players per side");
  const used = new Set(
    state.players
      .filter((player) => player.side === side)
      .map((player) => Number(player.id.split("-").at(-1))),
  );
  let nextIndex = 1;
  while (used.has(nextIndex)) nextIndex++;
  return withPlayers(state, [...state.players, makePlayer(side, nextIndex)]);
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

export function setController(
  state: LobbyState,
  target: Player | string,
  controller: ControllerKind,
): LobbyState {
  const id = playerTarget(target);
  const player = state.players.find((candidate) => candidate.id === id);
  if (!player) return state;
  if (controller === "human") {
    if (player.side !== "blue") throw new Error("the Human player must be on Blue");
    return seatHuman(state, id);
  }
  if (player.controller === "human") throw new Error("lobby must retain a Human player");
  return withPlayers(
    state,
    state.players.map((player) =>
      player.id === id ? { ...player, controller, human_capable: player.human_capable } : player,
    ),
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
        ? { ...player, controller: "human", human_capable: true }
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
  const minimum = minimumSize(state.players);
  const floor = sizeRank(state.sizeFloor) >= sizeRank(minimum) ? state.sizeFloor : minimum;
  return {
    ...state,
    sizeFloor: floor,
    map: { ...state.map, size: sizeRank(size) >= sizeRank(floor) ? size : floor },
  };
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
    entries: entryCount(
      Math.max(sideCount(state.players, "blue"), sideCount(state.players, "red")),
    ),
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

export function toRequest(state: LobbyState): PrepareBattleRequest {
  // Slice 06 will replace this placeholder generation request with its admitted setup seam.
  const request = {
    ...state.map,
    profile: "skirmish" as const,
    generator_version: "",
    preset_revision: "",
    template_catalog_hash: "",
    limits: { max_authored_parts: 0, max_bay_positions: 0, max_ground_points: 0 },
  } satisfies GenerationRequest;
  return {
    map_source: { kind: "generated", request },
    players: structuredClone(state.players),
    battle_seed: state.battleSeed,
  };
}

export function toUrl(state: LobbyState): string {
  // Slice 06 will replace this temporary lobby URL grammar with the battle link grammar.
  const params = new URLSearchParams({
    type: state.map.type,
    size: state.map.size,
    seed: state.map.seed,
    battle: String(state.battleSeed),
    floor: state.sizeFloor,
    players: JSON.stringify(state.players),
  });
  if (state.map.region !== undefined) params.set("region", state.map.region);
  return `?${params}`;
}

export function fromUrl(url: string): LobbyState {
  // Slice 06 will replace this temporary lobby URL parser with the battle link grammar.
  const params = new URL(url, "https://lobby.invalid").searchParams;
  const players = JSON.parse(params.get("players") ?? "null") as Player[] | null;
  const type = params.get("type");
  const size = params.get("size");
  if (!players || !MAP_TYPES.includes(type as MapType) || !MAP_SIZES.includes(size as MapSize))
    throw new Error("invalid lobby URL");
  const battleSeed = Number(params.get("battle"));
  if (!Number.isSafeInteger(battleSeed) || battleSeed < 0)
    throw new Error("invalid lobby battle seed");
  const normalizedPlayers = orderedPlayers(enforceHumanInvariant(players));
  const minimum = minimumSize(normalizedPlayers);
  const floorParam = params.get("floor") as MapSize | null;
  const sizeFloor =
    floorParam && MAP_SIZES.includes(floorParam) && sizeRank(floorParam) >= sizeRank(minimum)
      ? floorParam
      : minimum;
  return {
    players: normalizedPlayers,
    map: {
      type: type as MapType,
      size: sizeRank(size as MapSize) >= sizeRank(sizeFloor) ? (size as MapSize) : sizeFloor,
      seed: params.get("seed") ?? "1",
      profile: "skirmish",
      ...(params.get("region") === null ? {} : { region: params.get("region")! }),
    },
    battleSeed,
    sizeFloor,
  };
}
