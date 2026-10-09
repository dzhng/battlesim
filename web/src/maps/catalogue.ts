// The saved-map catalogue's listing: every `fixtures/maps/<id>/meta.json`,
// found by a Vite glob (there is no generated index to keep in step).
// `meta.json` says what a listing shows about a map without loading it. Only
// JavaScript reads it; the simulation reads `map.json` by id through the
// resolver. Every field a listing shows that the map itself also states (its
// size, its features, where it came from) is checked against the map by
// `checkMapFolder`, so a listing cannot drift from what loads.
import { MAP_ID, type MapDefinition, type MapIdentity } from "./resolve.ts";

/** What a saved map is for (`contract::maps::MapCategory`): a test's own
 *  ground (tests, labs, benchmarks) or a battlefield the menu backdrop films.
 *  Players never fight on a saved map: their battles are generated. */
const MAP_CATEGORIES = ["test", "menu"] as const;
const MAP_STATUSES = ["draft", "released", "retired"] as const;
const MAP_SOURCES = ["imported", "generated", "authored"] as const;
/** What kind of ground a map is: a generated map's type (`open`, `mixed`,
 *  `metro`), or an authored test's `arena`. */
const MAP_CHARACTERS = ["open", "mixed", "metro", "arena"] as const;
/** The physical features a map can have, each a tag when the map has any. */
const MAP_TAGS = [
  "relief",
  "river",
  "road",
  "bridge",
  "forest",
  "prop",
  "building",
] as const;

export type MapCategory = (typeof MAP_CATEGORIES)[number];
export type MapStatus = (typeof MAP_STATUSES)[number];
export type MapSource = (typeof MAP_SOURCES)[number];
export type MapCharacter = (typeof MAP_CHARACTERS)[number];
export type MapTag = (typeof MAP_TAGS)[number];

export interface MapMeta {
  /** What the map is for: a test's, or the menu's. */
  category: MapCategory;
  /** Drafts are listed only for developers; retired maps are not listed. */
  status: MapStatus;
  /** Its name in a listing. */
  label: string;
  character: MapCharacter;
  /** The palette it is drawn in: a `fixtures/biomes/<biome>.json`. */
  biome: string;
  /** Its ground's width and height, in metres. */
  size_m: [number, number];
  /** The physical features it has, in `MAP_TAGS` order. */
  tags: MapTag[];
  source: MapSource;
  /** A generated map's seed, as its decimal text; null otherwise. */
  seed: string | null;
  /** Its saved encounters: the names under `encounters/`, in order. */
  encounters: string[];
  /** The benchmarks that run on it, by name. */
  benchmarks: string[];
}

/** A catalogue entry: the folder's id with its metadata. */
export interface MapEntry extends MapMeta {
  id: string;
}

const FIELDS = [
  "category",
  "status",
  "label",
  "character",
  "biome",
  "size_m",
  "tags",
  "source",
  "seed",
  "encounters",
  "benchmarks",
] as const satisfies readonly (keyof MapMeta)[];

/** `value` as map `id`'s metadata, or an error naming the file and field. */
export function validateMapMeta(id: string, value: unknown): MapMeta {
  const fail = (field: string, message: string): never => {
    throw new Error(`fixtures/maps/${id}/meta.json${field && `.${field}`}: ${message}`);
  };
  if (!MAP_ID.test(id)) fail("", "the folder is not a catalogue id");
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return fail("", "metadata is an object");
  const meta = value as Record<string, unknown>;
  for (const key of Object.keys(meta))
    if (!(FIELDS as readonly string[]).includes(key)) fail(key, "unknown field");
  for (const field of FIELDS) if (!(field in meta)) fail(field, "missing");

  const oneOf = (field: string, options: readonly string[]) => {
    if (!options.includes(meta[field] as string))
      fail(field, `${JSON.stringify(meta[field])} is not one of ${options.join(", ")}`);
  };
  oneOf("category", MAP_CATEGORIES);
  oneOf("status", MAP_STATUSES);
  oneOf("character", MAP_CHARACTERS);
  oneOf("source", MAP_SOURCES);

  if (typeof meta.label !== "string" || meta.label.trim() === "") fail("label", "a nonempty name");
  if (typeof meta.biome !== "string" || !MAP_ID.test(meta.biome)) fail("biome", "a biome's name");
  const size = meta.size_m;
  if (
    !Array.isArray(size) ||
    size.length !== 2 ||
    !size.every((v) => typeof v === "number" && Number.isFinite(v) && v > 0)
  )
    fail("size_m", "[width, height] in positive metres");

  const names = (field: "tags" | "encounters" | "benchmarks", admit: (name: string) => boolean) => {
    const list = meta[field];
    if (!Array.isArray(list)) return fail(field, "a list of names");
    for (const name of list) {
      if (typeof name !== "string" || !admit(name))
        fail(field, `${JSON.stringify(name)} is not a name this list takes`);
      if (list.indexOf(name) !== list.lastIndexOf(name))
        fail(field, `${JSON.stringify(name)} is listed twice`);
    }
  };
  names("tags", (name) => (MAP_TAGS as readonly string[]).includes(name));
  names("encounters", (name) => MAP_ID.test(name));
  names("benchmarks", (name) => MAP_ID.test(name));

  // A seed is a generated map's, and crosses JSON as decimal text.
  const seed = meta.seed;
  if (meta.source === "generated") {
    if (typeof seed !== "string" || !/^(0|[1-9][0-9]*)$/.test(seed))
      fail("seed", "a generated map's seed, as decimal text");
  } else if (seed !== null) fail("seed", "null unless the map is generated");
  return meta as unknown as MapMeta;
}

/** The feature tags `map` has, in `MAP_TAGS` order. */
export function mapTags(map: MapDefinition): MapTag[] {
  const has: Record<MapTag, boolean> = {
    relief: map.relief.length > 0,
    river: (map.rivers ?? []).length > 0,
    road: map.surfaces.length > 0,
    bridge: map.bridges.length > 0,
    forest: map.forests.length > 0,
    prop: map.props.length > 0,
    building: (map.buildings ?? []).length > 0,
  };
  return MAP_TAGS.filter((tag) => has[tag]);
}

/** What map `id`'s folder holds besides its metadata. */
export interface MapFolder {
  definition: MapDefinition;
  identity: MapIdentity;
  /** The names under `encounters/`, in order. */
  encounters: string[];
}

/** Every way `meta` disagrees with the map it describes; empty when the
 *  listing says what loads. */
export function checkMapFolder(id: string, meta: MapMeta, folder: MapFolder): string[] {
  const problems: string[] = [];
  const differ = (field: string, listed: unknown, actual: unknown, of: string) => {
    if (JSON.stringify(listed) !== JSON.stringify(actual))
      problems.push(
        `fixtures/maps/${id}/meta.json.${field}: ${JSON.stringify(listed)}, but ${of} ${JSON.stringify(actual)}`,
      );
  };
  differ("size_m", meta.size_m, folder.definition.size, "the map's size is");
  differ("tags", meta.tags, mapTags(folder.definition), "the map's features are");
  differ("encounters", meta.encounters, folder.encounters, "the folder's encounters are");
  const { identity } = folder;
  if (identity.kind === "generated") {
    differ("source", meta.source, "generated", "SOURCES.json's identity is");
    differ("seed", meta.seed, identity.generation.seed, "the generation's seed is");
  } else if (meta.source === "generated")
    differ("source", meta.source, identity.kind, "SOURCES.json's identity is");
  return problems;
}

/** The entry of map `id` among `entries`, which must be a `category` map:
 *  the menu backdrop films only `menu` maps, and labs and tests stand only
 *  on `test` maps. A map the catalogue lacks, or one of another category, is
 *  refused by name. */
export function categoryMap(
  entries: readonly MapEntry[],
  id: string,
  category: MapCategory,
): MapEntry {
  const entry = entries.find((m) => m.id === id);
  if (!entry) throw new Error(`no saved map ${JSON.stringify(id)}`);
  if (entry.category !== category)
    throw new Error(
      `the saved map ${JSON.stringify(id)} is a ${entry.category} map, not a ${category} map`,
    );
  return entry;
}

/** The id a catalogue document's module path names: its folder. */
const mapIdOf = (path: string): string => path.split("/").at(-2)!;

let entries: MapEntry[] | null = null;

/** The catalogue's maps, by id. A map whose metadata is invalid fails the
 *  listing, naming its file and field. */
export function listMaps(): MapEntry[] {
  entries ??= Object.entries(
    import.meta.glob<unknown>("../../../fixtures/maps/*/meta.json", {
      eager: true,
      import: "default",
    }),
  )
    .map(([path, value]) => {
      const id = mapIdOf(path);
      return { id, ...validateMapMeta(id, value) };
    })
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return entries;
}
