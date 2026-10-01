// The one map resolver as JavaScript reaches it. A saved map is a folder of
// the catalogue, `fixtures/maps/<id>/`: its physical `map.json`, and the
// `SOURCES.json` that pins what it is and where it came from. An adapter (the
// browser's fetch, a tool's file read) obtains those documents and the
// physical template library, and the simulation's WebAssembly resolver admits
// them or refuses them, exactly as the native reader (`sim::maps`) does. The
// battle only ever receives the resolved definition.
//
// Plain TypeScript with no bundler features, so Node tools import it too.

/** A box the simulation places: half extents along heading, across, vertical. */
export interface MapBox {
  center: [number, number];
  yaw: number;
  half_extents: [number, number, number];
  base_z?: number | null;
}

/** A ground area: a closed ring, or a rounded line with a width. */
export type GroundShape =
  | { kind: "polygon"; ring: [number, number][] }
  | { kind: "stroke"; points: [number, number][]; width_m: number };

export interface MapProp extends MapBox {
  /** Its reserved prop id, when the map authors one. */
  id?: number;
  /** A prop type of the catalog. */
  kind: string;
}

export interface MapBuilding {
  /** The prop id that owns the building. */
  owner: number;
  /** A prop type of the catalog. */
  kind: string;
  category: string;
  regional_family: string;
  /** Each named part's prop id. */
  parts: { part: string; prop: number }[];
  /** The template's geometry, materialized where the building stands. */
  geometry: { template_id: string; parts: (MapBox & { id: string })[] } & Record<string, unknown>;
}

/** Water: a line of points, each with the water's width and its depth at the
 *  middle there, and one surface height. */
export interface MapRiver {
  points: { xy: [number, number]; width_m: number; depth_m: number }[];
  surface_z: number;
}

/** A resolved physical map (`contract::map::MapDefinition`), in metres. The
 *  resolver leaves out `rivers`, `buildings` and `template_catalog_hash` when
 *  a map has none. */
export interface MapDefinition {
  size: [number, number];
  fog_cell_m: number;
  height_grid_m: number;
  slope_cutoff_deg: number;
  relief: Record<string, unknown>[];
  rivers?: MapRiver[];
  surfaces: { kind: string; shape: GroundShape }[];
  bridges: Record<string, unknown>[];
  forests: { shape: GroundShape }[];
  props: MapProp[];
  buildings?: MapBuilding[];
  template_catalog_hash?: string;
}

/** What a map is, apart from where it is stored (`contract::maps::MapIdentity`). */
export type MapIdentity =
  | { kind: "authored"; map_hash: string; template_catalog_hash: string | null }
  | {
      kind: "generated";
      generation: {
        generator_version: string;
        preset_revision: string;
        seed: string;
        config_hash: string;
        template_catalog_hash: string;
        map_hash: string;
      };
    };

export interface ResolvedMap {
  definition: MapDefinition;
  identity: MapIdentity;
}

/** The documents an adapter obtained for one saved map, as text. */
export interface MapDocuments {
  /** `fixtures/maps/<id>/map.json`. */
  map: string;
  /** `fixtures/maps/<id>/SOURCES.json`. */
  sources: string;
  /** The physical template library the catalogue's maps pin. */
  library: string;
}

/** A catalogue address: one lowercase directory name, never a path
 *  (`contract::maps::MapId`). */
export const MAP_ID = /^[a-z0-9][a-z0-9_-]*$/;

/** A map or encounter that did not resolve: which document and field, and
 *  why. Nothing stands in for it. */
export class MapResolveError extends Error {
  readonly code: string;
  readonly location: string;

  constructor(code: string, location: string, message: string) {
    super(`${location}: ${message}`);
    this.name = "MapResolveError";
    this.code = code;
    this.location = location;
  }
}

/** Refuse an address that is not a catalogue id, before anything is read. */
export function checkAddress(name: string, location: string): void {
  if (!MAP_ID.test(name))
    throw new MapResolveError(
      "invalid_id",
      location,
      "catalogue id must be a lowercase ASCII directory name, starting with a letter or digit",
    );
}

/** The WebAssembly resolver (`resolve_saved_map`). */
export type SavedMapResolver = (map: string, sources: string, library: string) => string;

type ResolveOutcome =
  | { status: "ok"; result: ResolvedMap }
  | { status: "error"; error: { code: string; location: string; message: string } };

/** Map `id`'s documents through the resolver: the definition and identity, or
 *  the refusal, located under the map's folder. */
export function resolveSavedMap(
  resolver: SavedMapResolver,
  id: string,
  documents: MapDocuments,
): ResolvedMap {
  const outcome = JSON.parse(
    resolver(documents.map, documents.sources, documents.library),
  ) as ResolveOutcome;
  if (outcome.status === "ok") return outcome.result;
  const { code, location, message } = outcome.error;
  throw new MapResolveError(code, `${id}/${location}`, message);
}

/** A saved encounter (`contract::scenario::EncounterDefinition`): the forces,
 *  events and scripted orders a scenario lays on its map. */
export interface Encounter {
  units: unknown[];
  events?: unknown[];
  scripts?: unknown[];
  opponent?: unknown;
  encounter?: unknown;
}

/** A saved encounter's document, parsed. */
export function parseEncounter(location: string, text: string): Encounter {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (e) {
    throw new MapResolveError("invalid_encounter", location, (e as Error).message);
  }
  if (typeof value !== "object" || value === null || !Array.isArray((value as Encounter).units))
    throw new MapResolveError("invalid_encounter", location, "an encounter lists its `units`");
  return value as Encounter;
}
