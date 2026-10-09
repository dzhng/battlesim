// A test's own map in its saved form, resolved as the browser resolves a
// saved map (`resolve_saved_map`): each building is its template, from
// `templates`, materialized at its frame by the simulation's own code. The
// built WebAssembly must be initialised first.
import { resolve_saved_map, template_catalogue_json } from "@wasm/game_wasm.js";
import { MapResolveError, resolveSavedMap, type ResolvedMap } from "../src/maps/resolve";

/** `saved` (a `contract::map::SavedMap` without its catalogue hash) resolved
 *  against the physical library `templates`. As for an authored map, the
 *  first answer states the content hash it computed, which the second pins. */
export function resolveAuthored(saved: object, templates: readonly unknown[]): ResolvedMap {
  const library = JSON.stringify(templates);
  const catalogue = (JSON.parse(template_catalogue_json(library)) as { hash: string }).hash;
  const documents = (map_hash: string) => ({
    map: JSON.stringify({ ...saved, template_catalog_hash: catalogue }),
    sources: JSON.stringify({
      identity: { kind: "authored", map_hash, template_catalog_hash: catalogue },
      catalogue: { library: "test.json", template_ids: null },
      inputs: [{ kind: "supplied", label: "a test's own map", sha256: "0".repeat(64) }],
    }),
    library,
  });
  try {
    return resolveSavedMap(resolve_saved_map, "test", documents("0".repeat(64)));
  } catch (e) {
    const stated = /content hash is ([0-9a-f]{64})/.exec((e as MapResolveError).message)?.[1];
    if (!stated) throw e;
    return resolveSavedMap(resolve_saved_map, "test", documents(stated));
  }
}
