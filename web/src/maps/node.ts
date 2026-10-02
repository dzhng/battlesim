// Node's adapter over the one map resolver, for tests, scenes and tools: a
// saved map's documents read from `fixtures/maps/<id>/`, with the physical
// template library its sources name, and handed to the built WebAssembly
// resolver (`bun run build:wasm` first). The browser's
// adapter (`browser.ts`) fetches the same documents; the native reader is
// `sim::maps`.
//
// Plain TypeScript with no bundler features, so Node scripts import it too.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { initSync, resolve_saved_map } from "../wasm/game_wasm.js";
import {
  checkAddress,
  libraryOf,
  MapResolveError,
  parseEncounter,
  resolveSavedMap,
  type Encounter,
  type ResolvedMap,
} from "./resolve.ts";

const FIXTURES = join(import.meta.dirname, "../../../fixtures");
const WASM = join(import.meta.dirname, "../wasm/game_wasm_bg.wasm");

let ready = false;
/** The resolver, its module initialised once per process. */
function resolver(): typeof resolve_saved_map {
  if (!ready) {
    if (!existsSync(WASM))
      throw new Error(`the map resolver is not built (${WASM}): run \`bun run build:wasm\``);
    initSync({ module: readFileSync(WASM) });
    ready = true;
  }
  return resolve_saved_map;
}

function document(path: string, location: string): string {
  try {
    return readFileSync(path, "utf8");
  } catch (e) {
    throw new MapResolveError(
      "missing_document",
      location,
      `reading ${path}: ${(e as Error).message}`,
    );
  }
}

/** A catalogue of saved maps (`maps`, the directory of map folders) and the
 *  physical template libraries they pin (`libraries`, the directory a map's
 *  `SOURCES.json` names its library's file in). */
export function openCatalogue(maps: string, libraries: string) {
  const resolved = new Map<string, ResolvedMap>();
  const folder = (id: string) => (checkAddress(id, id), join(maps, id));
  const read = (id: string, name: string) => document(join(folder(id), name), `${id}/${name}`);
  const templates = (id: string) =>
    document(
      join(libraries, libraryOf(id, read(id, "SOURCES.json"))),
      `${id}/SOURCES.json.catalogue.library`,
    );
  return {
    /** Every map's id, in order: the catalogue's folders. */
    ids(): string[] {
      return readdirSync(maps)
        .filter((name) => statSync(join(maps, name)).isDirectory())
        .sort();
    },
    /** Map `id`'s document `name` (`map.json`, `SOURCES.json`), as written. */
    document: read,
    /** The physical template library map `id`'s sources name, as written. */
    library: templates,
    /** Map `id`, resolved, or the refusal naming the document and field at
     *  fault. */
    load(id: string): ResolvedMap {
      let map = resolved.get(id);
      if (!map) {
        map = resolveSavedMap(resolver(), id, {
          map: read(id, "map.json"),
          sources: read(id, "SOURCES.json"),
          library: templates(id),
        });
        resolved.set(id, map);
      }
      return map;
    },
    /** The names of map `id`'s saved encounters, in order. */
    encounters(id: string): string[] {
      const dir = join(folder(id), "encounters");
      if (!existsSync(dir)) return [];
      return readdirSync(dir)
        .filter((name) => name.endsWith(".json"))
        .map((name) => name.slice(0, -".json".length))
        .sort();
    },
    /** Map `id`'s saved encounter `name`. */
    encounter(id: string, name: string): Encounter {
      const location = `${id}/encounters/${name}.json`;
      checkAddress(name, location);
      return parseEncounter(
        location,
        document(join(folder(id), "encounters", `${name}.json`), location),
      );
    },
    /** Map `id`'s `meta.json`, as written (`catalogue.ts` validates it). */
    meta(id: string): unknown {
      return JSON.parse(read(id, "meta.json"));
    },
  };
}

/** The repository's catalogue, `fixtures/maps/`. */
export const shipped = openCatalogue(join(FIXTURES, "maps"), FIXTURES);

/** The shipped catalogue's map `id`, resolved. The value is shared: a caller
 *  that changes a map copies it first. */
export const loadMap = (id: string): ResolvedMap => shipped.load(id);

/** The shipped catalogue's encounter `name` of map `id`. */
export const loadEncounter = (id: string, name: string): Encounter => shipped.encounter(id, name);
