// The address of a prepared battle, `/battle?…`: what the main menu links to
// and what the battle route reads back. The address is the battle's share
// identity: an exact address prepares the same battle on the same build.
// Ordinary Play publishes that exact address only after admission.
//
//   /battle?play=1&type=mixed&size=small      ordinary unpinned Play
//   /battle?type=mixed&size=small&seed=<u64>   an exact generated map
//       [&recipe=<name>] [&encounter=<u64>] [&battle=<n>]
//   /battle?replay=saved                       the saved replay of either
// A generated map's address may add `&region=<family>`; without it the seed
// draws the region.
import type { PrepareBattleRequest } from "@web/battle/prepare/protocol";
import game from "@fixtures/game.json";
import config from "@fixtures/generated-battle.json";
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
      map: MapChoice;
      recipe: string;
      encounterSeed: string;
      battleSeed: number;
    }
  | {
      kind: "play";
      map: Omit<MapChoice, "seed">;
      recipe: string;
      encounterSeed: string;
      battleSeed: number;
    }
  | { kind: "replay" };

/** The regions a player may ask for: the presets' regional families. */
export const REGIONS: readonly string[] = presets.parcels.regional_families;
/** A data name as the player reads it: `new_york` is "new york". */
export const spoken = (name: string) => name.replaceAll("_", " ");

const one = <T extends string>(value: string | null, of: readonly T[]): value is T =>
  value !== null && (of as readonly string[]).includes(value);

/** The map a menu address names (`/?type=&size=&seed=&region=`), field by
 *  field: what the menu opens on after a battle is cancelled or refused. */
export function askedChoice(search: string): Partial<MapChoice> {
  const params = new URLSearchParams(search);
  const [type, size, region] = [params.get("type"), params.get("size"), params.get("region")];
  return {
    ...(one(type, MAP_TYPES) && { type }),
    ...(one(size, MAP_SIZES) && { size }),
    seed: canonicalSeed(params.get("seed") ?? "") ?? undefined,
    ...(one(region, REGIONS) && { region }),
  };
}

/** A generated map's fields as address text, the region only when chosen. */
const query = (choice: Partial<MapChoice> & Pick<MapChoice, "type" | "size">) =>
  [
    `type=${choice.type}&size=${choice.size}`,
    choice.seed !== undefined && `seed=${choice.seed}`,
    choice.region !== undefined && `region=${choice.region}`,
  ]
    .filter(Boolean)
    .join("&");
/** The battle on the generated map `choice`. */
export const battleHref = (choice: MapChoice) => `/battle?${query(choice)}`;
/** Ordinary Play chooses an admitted battle after navigation. */
export const playHref = (choice: Omit<MapChoice, "seed">) => `/battle?play=1&${query(choice)}`;

/** The exact address of the battle preparation actually admitted. */
export function preparedBattleHref(request: PrepareBattleRequest): string {
  const source = request.map_source;
  const params = new URLSearchParams(
    source.kind === "generated" ? query(source.request) : { map: source.id },
  );
  params.set("recipe", request.recipe_id);
  params.set("encounter", request.encounter_seed);
  params.set("battle", String(request.battle_seed));
  return `/battle?${params}`;
}

/** The main menu, opened on `choice`. */
export const menuHref = (choice: MapChoice | Omit<MapChoice, "seed"> | null) =>
  choice ? `/?${query(choice)}` : "/";

/** The battle `search` asks for, or which parameter it gets wrong. Nothing
 *  is guessed for a parameter that is present and wrong. */
export function askedBattle(search: string): AskedBattle | { error: string } {
  const params = new URLSearchParams(search);
  if (params.has("replay")) return { kind: "replay" };
  if (params.has("play") && (params.get("play") !== "1" || params.has("seed") || params.has("map")))
    return { error: "play must be 1 and cannot name an exact seed or saved map" };
  const seed = (name: string, fallback: string) => canonicalSeed(params.get(name) ?? fallback);
  const encounterSeed = seed("encounter", config.encounter.seed);
  if (encounterSeed === null)
    return { error: "encounter must be a whole number from 0 to 18446744073709551615" };
  const battleSeed = Number(params.get("battle") ?? game.seed);
  if (!Number.isSafeInteger(battleSeed) || battleSeed < 0)
    return { error: `battle must be a whole number from 0 to ${Number.MAX_SAFE_INTEGER}` };
  const recipe = params.get("recipe");
  if (params.has("map")) return { error: "map is not supported; start a generated skirmish" };
  const type = params.get("type") ?? ("mixed" satisfies MapType);
  const size = params.get("size") ?? ("small" satisfies MapSize);
  const mapSeed = seed("seed", "1");
  if (!one(type, MAP_TYPES)) return { error: `type must be one of ${MAP_TYPES.join(", ")}` };
  if (!one(size, MAP_SIZES)) return { error: `size must be one of ${MAP_SIZES.join(", ")}` };
  const region = params.get("region");
  if (region !== null && !one(region, REGIONS))
    return { error: `region must be one of ${REGIONS.join(", ")}` };
  const chosen = region === null ? {} : { region };
  if (params.has("play"))
    return {
      kind: "play",
      map: { type, size, ...chosen },
      recipe: recipe ?? config.encounter.recipe,
      encounterSeed,
      battleSeed,
    };
  if (mapSeed === null)
    return { error: "seed must be a whole number from 0 to 18446744073709551615" };
  return {
    kind: "generated",
    map: { type, size, seed: mapSeed, ...chosen },
    recipe: recipe ?? config.encounter.recipe,
    encounterSeed,
    battleSeed,
  };
}
