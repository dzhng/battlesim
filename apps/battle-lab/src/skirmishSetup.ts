// A generated skirmish's setup as the browser builds it: its players, the
// size tier they need, and the one preparation request the lobby previews and
// its Deploy link prepares, so the battle is the map and players the lobby
// showed.
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import skirmishSizes from "@fixtures/skirmish-sizes.json";
import type { Player, PrepareBattleRequest } from "@web/battle/prepare/protocol";
import { generationRequest, type MapChoice, type MapGenerator } from "@web/maps/source";
import type { Faction } from "@packages/scene-assets/src/units";

/** The most players a side fields in the lobby or its link: lobby data,
 *  apart from admission's own `max_players_per_side` (D35). */
export const MAX_PLAYERS_PER_SIDE = 5;

export type SkirmishSide = "blue" | "red";

/** The faction a side fields until the player picks another. */
export const DEFAULT_FACTION: Readonly<Record<SkirmishSide, Faction>> = {
  blue: "us",
  red: "eastern",
};

/** A skirmish player before it is numbered: who plays which side, as what. */
export type SkirmishSeat = Pick<Player, "side" | "faction" | "controller">;

/** `seats` as the battle's players: Blue's then Red's, each side in its own
 *  order, named `side-index` from 1, and human-capable exactly when a human
 *  plays them. A link names players this way, so every setup does. */
export function skirmishPlayers(seats: readonly SkirmishSeat[]): Player[] {
  return (["blue", "red"] as const).flatMap((side) =>
    seats
      .filter((seat) => seat.side === side)
      .map((seat, index) => ({
        id: `${side}-${index + 1}`,
        side,
        faction: seat.faction,
        controller: seat.controller,
        human_capable: seat.controller === "human",
      })),
  );
}

/** `blueCount` against `redCount` with the default factions, Blue's
 *  `humanBlueIndex`th player human and the rest active AI. */
export function playersFor(
  blueCount: number,
  redCount: number,
  humanBlueIndex: number | null = 0,
): Player[] {
  return skirmishPlayers(
    (["blue", "red"] as const).flatMap((side) =>
      Array.from({ length: side === "blue" ? blueCount : redCount }, (_, index) => ({
        side,
        faction: DEFAULT_FACTION[side],
        controller:
          side === "blue" && index === humanBlueIndex ? ("human" as const) : ("active_ai" as const),
      })),
    ),
  );
}

/** The shared fixture's size tier for the larger side's player count: the
 *  smallest map that fits it and the entry arms each side gets. */
export function sizeTier(players: readonly Pick<Player, "side">[]) {
  const largest = Math.max(
    players.filter((player) => player.side === "blue").length,
    players.filter((player) => player.side === "red").length,
  );
  const tier = skirmishSizes.tiers.find(
    (candidate) =>
      largest >= candidate.min_players &&
      (candidate.max_players === undefined || largest <= candidate.max_players),
  );
  if (!tier) throw new Error(`no skirmish size tier for ${largest} players`);
  return tier;
}

/** The shared fixture's entry-arm count for the larger combatant side. */
export function entryCountForPlayers(players: readonly Pick<Player, "side">[]): number {
  return players.some((player) => player.side === "blue" || player.side === "red")
    ? sizeTier(players).entry_count
    : 0;
}

/** What a generated skirmish asks for, before it is pinned to this build. */
export interface SkirmishBattle {
  players: Player[];
  map: MapChoice;
  battleSeed: number;
}

/** `battle`'s preparation request, pinned to this build's generator, presets
 *  and template catalogue. */
export function skirmishRequest(
  generator: MapGenerator,
  { players, map, battleSeed }: SkirmishBattle,
): PrepareBattleRequest {
  return {
    map_source: {
      kind: "generated",
      request: generationRequest(
        generator,
        { ...map, profile: "skirmish", entries_per_side: entryCountForPlayers(players) },
        { presets, templates },
        config.limits,
      ),
    },
    players,
    battle_seed: battleSeed,
  };
}
