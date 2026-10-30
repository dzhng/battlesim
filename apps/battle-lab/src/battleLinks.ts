// The address of a prepared battle, `/battle?…`: what the main menu links to
// and what the battle route reads back. The address is the battle's share
// identity: an exact address prepares the same battle on the same build.
// The lobby links only to the map its preview admitted.
//
//   /battle?type=mixed&size=small&seed=<u64>&blue=h&red=a&blue_faction=us&red_faction=eastern
//   /battle?replay=saved                                 the saved replay of either
// A battle must name its players: each side's ordered controller codes and
// its faction. The map type, size, map seed and battle seed may be left out,
// and then default (the size to the smallest that fits the players); `&region=<family>` is optional, and without it the seed
// draws the region. A wrong value, or a parameter the battle does not read
// (an old link's `faction` or `enemy`), is refused by name, never ignored.
import { DEFAULT_BATTLE_SEED } from "@apps/battle-lab/src/lobby/defaults";
import { FACTIONS, type Faction } from "@packages/scene-assets/src/units";
import type { Player } from "@web/battle/prepare/protocol";
import {
  MAX_PLAYERS_PER_SIDE,
  sizeTier,
  skirmishPlayers,
  type SkirmishBattle,
  type SkirmishSide,
} from "./skirmishSetup";
import presets from "@fixtures/map-presets.json";
import {
  canonicalSeed,
  MAP_SIZES,
  MAP_TYPES,
  type MapChoice,
  type MapType,
} from "@web/maps/source";

/** What a battle address asks for, before it is pinned to this build. */
export type AskedBattle = ({ kind: "generated" } & SkirmishBattle) | { kind: "replay" };

/** Every parameter a battle address may hold. */
const PARAMETERS = new Set([
  "type",
  "size",
  "seed",
  "region",
  "blue",
  "red",
  "blue_faction",
  "red_faction",
  "battle",
  "replay",
]);

/** The regions a player may ask for: the presets' regional families. */
export const REGIONS: readonly string[] = presets.parcels.regional_families;
/** Data names the player reads as another name. */
const SAID: Readonly<Record<string, string>> = {
  small: "S",
  medium: "M",
  large: "L",
  xl: "XL",
  china: "taiwan",
  us: "U.S.",
  europe: "European",
  eastern: "Eastern",
};
/** What a menu address asks the lobby to open on. */
export type AskedLobby = Partial<MapChoice & { faction: Faction }>;

/** A data name as the player reads it: `new_york` is "new york", `china` "taiwan". */
export const spoken = (name: string) => SAID[name] ?? name.replaceAll("_", " ");

const one = <T extends string>(value: string | null, of: readonly T[]): value is T =>
  value !== null && (of as readonly string[]).includes(value);

/** The map a menu address names (`/?type=&size=&seed=&region=`), field by
 *  field: what the menu opens on after a battle is cancelled or refused. */
export function askedChoice(search: string): AskedLobby {
  const params = new URLSearchParams(search);
  const [type, size, region] = [params.get("type"), params.get("size"), params.get("region")];
  const faction = params.get("blue_faction");
  return {
    ...(one(faction, FACTIONS) && { faction: faction as Faction }),
    ...(one(type, MAP_TYPES) && { type }),
    ...(one(size, MAP_SIZES) && { size }),
    seed: canonicalSeed(params.get("seed") ?? "") ?? undefined,
    ...(one(region, REGIONS) && { region }),
  };
}

/** A generated map's fields as address text, the region only when chosen. */
const query = (choice: MapChoice) =>
  [
    `type=${choice.type}&size=${choice.size}`,
    choice.seed !== undefined && `seed=${choice.seed}`,
    choice.region !== undefined && `region=${choice.region}`,
  ]
    .filter(Boolean)
    .join("&");
const CONTROLLERS: Record<string, Player["controller"]> = {
  h: "human",
  a: "active_ai",
  p: "passive_ai",
};
const controllerCode: Record<Player["controller"], string> = {
  human: "h",
  active_ai: "a",
  passive_ai: "p",
};

function playerParams(players: readonly Player[]): string[] {
  return (["blue", "red"] as const).flatMap((side) => {
    const sidePlayers = players.filter((player) => player.side === side);
    return [
      `${side}=${sidePlayers.map((player) => controllerCode[player.controller]).join(",")}`,
      `${side}_faction=${sidePlayers[0]?.faction ?? "us"}`,
    ];
  });
}

/** The battle `players` fight on the generated map `choice`. */
export const battleHref = (choice: MapChoice, players: readonly Player[], battleSeed?: number) =>
  `/battle?${[
    query(choice),
    ...(battleSeed === undefined ? [] : [`battle=${battleSeed}`]),
    ...playerParams(players),
  ]
    .filter(Boolean)
    .join("&")}`;
/** The main menu, opened on `choice`. */
export const menuHref = (choice: MapChoice | null) => (choice ? `/?${query(choice)}` : "/");

/** The battle `search` asks for, or which parameter it gets wrong. Missing
 *  players, a wrong value and a parameter the battle does not read are
 *  refused, so a stale link cannot pass as another battle. */
export function askedBattle(search: string): AskedBattle | { error: string } {
  const params = new URLSearchParams(search);
  const unknown = [...params.keys()].find((name) => !PARAMETERS.has(name));
  if (unknown !== undefined) return { error: `${unknown} is not a battle parameter` };
  if (params.has("replay")) return { kind: "replay" };
  const seed = (name: string, fallback: string) => canonicalSeed(params.get(name) ?? fallback);
  const battleSeed = Number(params.get("battle") ?? DEFAULT_BATTLE_SEED);
  if (!Number.isSafeInteger(battleSeed) || battleSeed < 0)
    return { error: `battle must be a whole number from 0 to ${Number.MAX_SAFE_INTEGER}` };
  const blueCodes = params.get("blue");
  const redCodes = params.get("red");
  if (blueCodes === null || redCodes === null)
    return { error: "missing players: blue and red are required" };
  const blueFaction = params.get("blue_faction");
  const redFaction = params.get("red_faction");
  if (!one(blueFaction, FACTIONS)) return { error: "blue_faction must be us, europe or eastern" };
  if (!one(redFaction, FACTIONS)) return { error: "red_faction must be us, europe or eastern" };
  const sides = [
    ["blue", blueCodes.split(","), blueFaction],
    ["red", redCodes.split(","), redFaction],
  ] as const satisfies readonly (readonly [SkirmishSide, string[], Faction])[];
  for (const [side, codes] of sides) {
    if (codes.some((code) => CONTROLLERS[code] === undefined))
      return { error: `${side} has an unknown controller` };
    if (codes.length > MAX_PLAYERS_PER_SIDE)
      return { error: `a side cannot have more than ${MAX_PLAYERS_PER_SIDE} players` };
  }
  const players = skirmishPlayers(
    sides.flatMap(([side, codes, faction]) =>
      codes.map((code) => ({ side, faction, controller: CONTROLLERS[code] })),
    ),
  );
  // A battle without a human is watched (D34); one with humans is played by
  // exactly one, on Blue.
  const humans = players.filter((player) => player.controller === "human");
  if (humans.length > 0 && (humans.length > 1 || humans[0].side !== "blue"))
    return { error: "a link with a Human has exactly one Human on Blue" };
  const type = params.get("type") ?? ("mixed" satisfies MapType);
  // Without a size, the smallest map that fits the players, as in the lobby.
  const size = params.get("size") ?? sizeTier(players).size;
  const mapSeed = seed("seed", "1");
  if (!one(type, MAP_TYPES)) return { error: `type must be one of ${MAP_TYPES.join(", ")}` };
  if (!one(size, MAP_SIZES)) return { error: `size must be one of ${MAP_SIZES.join(", ")}` };
  const region = params.get("region");
  if (region !== null && !one(region, REGIONS))
    return { error: `region must be one of ${REGIONS.join(", ")}` };
  const chosen = region === null ? {} : { region };
  if (mapSeed === null)
    return { error: "seed must be a whole number from 0 to 18446744073709551615" };
  return { kind: "generated", players, map: { type, size, seed: mapSeed, ...chosen }, battleSeed };
}
