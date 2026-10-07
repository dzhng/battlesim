// The address of a prepared battle, `/battle?…`: what the main menu links to
// and what the battle route reads back. The address is the battle's share
// identity: an exact address prepares the same battle on the same build.
// Ordinary Play publishes that exact address only after admission.
//
//   /battle?play=1&type=mixed&size=small&faction=us     ordinary unpinned Play
//   /battle?type=mixed&size=small&seed=<u64>&faction=us  an exact generated map
//       [&enemy=<faction>] [&battle=<n>]
//   /battle?replay=saved                                 the saved replay of either
// A battle names the player's faction; without `enemy` the enemy is Eastern,
// or U.S. against Eastern. A generated map's address may add
// `&region=<family>`; without it the seed draws the region. A missing faction
// or any other parameter is refused by name, never defaulted or ignored.
import type { Faction } from "@packages/scene-assets/src/units";
import type { PrepareBattleRequest } from "@web/battle/prepare/protocol";
import game from "@fixtures/game.json";
import presets from "@fixtures/map-presets.json";
import {
  canonicalSeed,
  MAP_SIZES,
  MAP_TYPES,
  type MapChoice,
  type MapSize,
  type MapType,
} from "@web/maps/source";

/** What a battle address asks for, before it is pinned to this build. */
export type AskedBattle =
  | {
      kind: "generated";
      /** The player's faction, then the enemy's. */
      factions: [Faction, Faction];
      map: MapChoice;
      battleSeed: number;
    }
  | {
      kind: "play";
      factions: [Faction, Faction];
      map: Omit<MapChoice, "seed">;
      battleSeed: number;
    }
  | { kind: "replay" };

/** Every parameter a battle address may hold. */
const PARAMETERS = new Set([
  "play",
  "type",
  "size",
  "seed",
  "region",
  "profile",
  "faction",
  "enemy",
  "battle",
  "replay",
]);

/** The regions a player may ask for: the presets' regional families. */
export const REGIONS: readonly string[] = presets.parcels.regional_families;
/** Data names the player reads as another name. */
const SAID: Readonly<Record<string, string>> = {
  china: "taiwan",
  us: "U.S.",
  europe: "European",
  eastern: "Eastern",
};
export const FACTIONS = ["us", "europe", "eastern"] as const;
type BattleChoice = MapChoice & { faction?: Faction };

/** A data name as the player reads it: `new_york` is "new york", `china` "taiwan". */
export const spoken = (name: string) => SAID[name] ?? name.replaceAll("_", " ");

const one = <T extends string>(value: string | null, of: readonly T[]): value is T =>
  value !== null && (of as readonly string[]).includes(value);

/** The map a menu address names (`/?type=&size=&seed=&region=`), field by
 *  field: what the menu opens on after a battle is cancelled or refused. */
export function askedChoice(search: string): Partial<BattleChoice> {
  const params = new URLSearchParams(search);
  const [type, size, region] = [params.get("type"), params.get("size"), params.get("region")];
  return {
    ...(one(params.get("faction"), FACTIONS) && { faction: params.get("faction") as Faction }),
    ...(one(type, MAP_TYPES) && { type }),
    ...(one(size, MAP_SIZES) && { size }),
    ...(params.get("profile") === "skirmish" && { profile: "skirmish" as const }),
    seed: canonicalSeed(params.get("seed") ?? "") ?? undefined,
    ...(one(region, REGIONS) && { region }),
  };
}

/** A generated map's fields as address text, the region only when chosen. */
const query = (choice: Partial<BattleChoice> & Pick<MapChoice, "type" | "size">) =>
  [
    `type=${choice.type}&size=${choice.size}`,
    choice.seed !== undefined && `seed=${choice.seed}`,
    choice.region !== undefined && `region=${choice.region}`,
    choice.profile !== undefined && `profile=${choice.profile}`,
    choice.faction !== undefined && `faction=${choice.faction}`,
  ]
    .filter(Boolean)
    .join("&");
/** The battle on the generated map `choice`. */
export const battleHref = (choice: BattleChoice) => `/battle?${query(choice)}`;
/** Ordinary Play chooses an admitted battle after navigation. */
export const playHref = (choice: Omit<BattleChoice, "seed">) => `/battle?play=1&${query(choice)}`;

/** The exact address of the battle preparation actually admitted. */
export function preparedBattleHref(request: PrepareBattleRequest): string {
  const source = request.map_source;
  // Only generated battles are admitted; a saved map has no battle address.
  if (source.kind !== "generated") throw new Error("a saved map has no battle address");
  const params = new URLSearchParams(query(source.request));
  params.set("faction", request.factions[0]);
  params.set("enemy", request.factions[1]);
  params.set("battle", String(request.battle_seed));
  return `/battle?${params}`;
}

/** The main menu, opened on `choice`. */
export const menuHref = (choice: MapChoice | Omit<MapChoice, "seed"> | null) =>
  choice ? `/?${query(choice)}` : "/";

/** The battle `search` asks for, or which parameter it gets wrong. Nothing
 *  is guessed for a parameter that is missing or wrong, and a parameter the
 *  battle does not read is refused, so a stale link cannot pass as another
 *  battle. */
export function askedBattle(search: string): AskedBattle | { error: string } {
  const params = new URLSearchParams(search);
  const unknown = [...params.keys()].find((name) => !PARAMETERS.has(name));
  if (unknown !== undefined) return { error: `${unknown} is not a battle parameter` };
  if (params.has("replay")) return { kind: "replay" };
  if (params.has("play") && (params.get("play") !== "1" || params.has("seed")))
    return { error: "play must be 1 and cannot name an exact seed" };
  const seed = (name: string, fallback: string) => canonicalSeed(params.get(name) ?? fallback);
  const battleSeed = Number(params.get("battle") ?? game.seed);
  if (!Number.isSafeInteger(battleSeed) || battleSeed < 0)
    return { error: `battle must be a whole number from 0 to ${Number.MAX_SAFE_INTEGER}` };
  const faction = params.get("faction");
  const enemy = params.get("enemy");
  if (!one(faction, FACTIONS)) return { error: "faction must be us, europe or eastern" };
  if (enemy !== null && !one(enemy, FACTIONS))
    return { error: "enemy must be us, europe or eastern" };
  const factions: [Faction, Faction] = [
    faction,
    enemy ?? (faction === "eastern" ? "us" : "eastern"),
  ];
  const type = params.get("type") ?? ("mixed" satisfies MapType);
  const size = params.get("size") ?? ("small" satisfies MapSize);
  const mapSeed = seed("seed", "1");
  if (!one(type, MAP_TYPES)) return { error: `type must be one of ${MAP_TYPES.join(", ")}` };
  if (!one(size, MAP_SIZES)) return { error: `size must be one of ${MAP_SIZES.join(", ")}` };
  const region = params.get("region");
  if (region !== null && !one(region, REGIONS))
    return { error: `region must be one of ${REGIONS.join(", ")}` };
  const profile = params.get("profile");
  // Both factions are fielded on skirmish geography: no other profile plays.
  if (profile !== null && profile !== "skirmish") return { error: "profile must be skirmish" };
  const chosen = {
    ...(region === null ? {} : { region }),
    ...(profile !== null && { profile: "skirmish" }),
  } as Pick<MapChoice, "region" | "profile">;
  if (params.has("play"))
    return { kind: "play", factions, map: { type, size, ...chosen }, battleSeed };
  if (mapSeed === null)
    return { error: "seed must be a whole number from 0 to 18446744073709551615" };
  return { kind: "generated", factions, map: { type, size, seed: mapSeed, ...chosen }, battleSeed };
}
