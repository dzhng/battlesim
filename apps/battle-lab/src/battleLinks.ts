// The address of a prepared battle, `/battle?…`: what the main menu links to
// and what the battle route reads back. The address is the battle's share
// identity: the same address prepares the same battle on the same build.
//
//   /battle?type=mixed&size=small&seed=<u64>   a generated map (the menu's)
//       [&recipe=<name>] [&encounter=<u64>] [&battle=<n>]
//   /battle?map=<id>&recipe=<name>             a saved map and its encounter
//   /battle?replay=saved                       the saved replay of either
import game from "@fixtures/game.json";
import config from "@fixtures/generated-battle.json";
import {
  canonicalSeed,
  MAP_SIZES,
  MAP_TYPES,
  type MapChoice,
  type MapSize,
  type MapType,
} from "@web/maps/source";
import { MAP_ID } from "@web/maps/resolve";

/** What a battle address asks for, before it is pinned to this build. */
export type AskedBattle =
  | {
      kind: "generated";
      map: MapChoice;
      recipe: string;
      encounterSeed: string;
      battleSeed: number;
    }
  | { kind: "catalogue"; id: string; recipe: string; encounterSeed: string; battleSeed: number }
  | { kind: "replay" };

const one = <T extends string>(value: string | null, of: readonly T[]): value is T =>
  value !== null && (of as readonly string[]).includes(value);

/** The map a menu address names (`/?type=&size=&seed=`), field by field:
 *  what the menu opens on after a battle is cancelled or refused. */
export function askedChoice(search: string): Partial<MapChoice> {
  const params = new URLSearchParams(search);
  const [type, size] = [params.get("type"), params.get("size")];
  return {
    ...(one(type, MAP_TYPES) && { type }),
    ...(one(size, MAP_SIZES) && { size }),
    seed: canonicalSeed(params.get("seed") ?? "") ?? undefined,
  };
}

const query = (choice: MapChoice) =>
  `type=${choice.type}&size=${choice.size}&seed=${choice.seed}` as const;
/** The battle on the generated map `choice`. */
export const battleHref = (choice: MapChoice) => `/battle?${query(choice)}`;
/** The main menu, opened on `choice`. */
export const menuHref = (choice: MapChoice | null) => (choice ? `/?${query(choice)}` : "/");

/** The battle `search` asks for, or which parameter it gets wrong. Nothing
 *  is guessed for a parameter that is present and wrong. */
export function askedBattle(search: string): AskedBattle | { error: string } {
  const params = new URLSearchParams(search);
  if (params.has("replay")) return { kind: "replay" };
  const seed = (name: string, fallback: string) => canonicalSeed(params.get(name) ?? fallback);
  const encounterSeed = seed("encounter", config.encounter.seed);
  if (encounterSeed === null)
    return { error: "encounter must be a whole number from 0 to 18446744073709551615" };
  const battleSeed = Number(params.get("battle") ?? game.seed);
  if (!Number.isSafeInteger(battleSeed) || battleSeed < 0)
    return { error: `battle must be a whole number from 0 to ${Number.MAX_SAFE_INTEGER}` };
  const recipe = params.get("recipe");
  const id = params.get("map");
  if (id !== null) {
    if (!MAP_ID.test(id)) return { error: "map must be a saved map's id" };
    if (recipe === null) return { error: "recipe must name one of the map's saved encounters" };
    return { kind: "catalogue", id, recipe, encounterSeed, battleSeed };
  }
  const type = params.get("type") ?? ("mixed" satisfies MapType);
  const size = params.get("size") ?? ("small" satisfies MapSize);
  const mapSeed = seed("seed", "1");
  if (!one(type, MAP_TYPES)) return { error: `type must be one of ${MAP_TYPES.join(", ")}` };
  if (!one(size, MAP_SIZES)) return { error: `size must be one of ${MAP_SIZES.join(", ")}` };
  if (mapSeed === null)
    return { error: "seed must be a whole number from 0 to 18446744073709551615" };
  return {
    kind: "generated",
    map: { type, size, seed: mapSeed },
    recipe: recipe ?? config.encounter.recipe,
    encounterSeed,
    battleSeed,
  };
}
