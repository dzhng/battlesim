// The browser's adapter over the one map resolver: a saved map's documents
// (`fixtures/maps/<id>/`), fetched over HTTP and handed to the WebAssembly
// resolver. Each document is its own served file, found by a Vite glob, so no
// map is part of a script; a map is fetched when a route first asks for it.
// Node tools read the same documents with `node.ts`; the native reader is
// `sim::maps`.
import { loadWasm } from "../battle/sim/module";
import {
  checkAddress,
  MapResolveError,
  parseEncounter,
  resolveSavedMap,
  type Encounter,
  type ResolvedMap,
} from "./resolve.ts";

/** Each catalogue document's URL, by its path from here. */
const URLS = import.meta.glob<string>(
  [
    "../../../fixtures/maps/*/{map,SOURCES}.json",
    "../../../fixtures/maps/*/encounters/*.json",
    "../../../fixtures/building-templates.json",
  ],
  { query: "?url&no-inline", import: "default", eager: true },
);
const FIXTURES = "../../../fixtures/";

async function document(path: string, location: string): Promise<string> {
  const url = URLS[`${FIXTURES}${path}`];
  if (!url)
    throw new MapResolveError(
      "missing_document",
      location,
      `the catalogue has no fixtures/${path}`,
    );
  const response = await fetch(url);
  if (!response.ok)
    throw new MapResolveError(
      "missing_document",
      location,
      `fetching fixtures/${path}: HTTP ${response.status}`,
    );
  return response.text();
}

const maps = new Map<string, Promise<ResolvedMap>>();
let library: Promise<string> | null = null;

/** Map `id`, resolved, or the refusal naming the document and field at fault.
 *  The value is shared: a caller that changes a map copies it first. */
export function loadMap(id: string): Promise<ResolvedMap> {
  let map = maps.get(id);
  if (!map) {
    map = (async () => {
      checkAddress(id, id);
      library ??= document("building-templates.json", "physical catalogue");
      const [wasm, text, sources, templates] = await Promise.all([
        loadWasm(),
        document(`maps/${id}/map.json`, `${id}/map.json`),
        document(`maps/${id}/SOURCES.json`, `${id}/SOURCES.json`),
        library,
      ]);
      return resolveSavedMap(wasm.resolve_saved_map, id, {
        map: text,
        sources,
        library: templates,
      });
    })();
    maps.set(id, map);
  }
  return map;
}

/** Map `id`'s saved encounter `name`. */
export async function loadEncounter(id: string, name: string): Promise<Encounter> {
  const location = `${id}/encounters/${name}.json`;
  checkAddress(id, id);
  checkAddress(name, location);
  return parseEncounter(location, await document(`maps/${id}/encounters/${name}.json`, location));
}
