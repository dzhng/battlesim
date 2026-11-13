/** What fielded units draw from, resolved by Rust (`presentation::fielded`):
 *  the page only reads the files it names (`fixtures/presentation.json`, the
 *  runtime catalog, its bundles and skeleton clips). A battle loads the
 *  result into its world; a showcase shows one unit from it. */
import { presentationSettings } from "@web/presentationSettings";
import type { AppearanceLibrary } from "@packages/scene-assets/src/loader";
import type { CatalogView } from "@packages/scene-assets/src/units";
import { Fielding } from "@wasm/renderer_browser";

/** The runtime files a page's appearances were loaded from: the runtime
 *  catalog and its bundle contents by hash (`AppearanceLibrary`). */
export type RuntimeFiles = Pick<AppearanceLibrary, "catalog" | "contents">;

/** The units and effects sections `kinds` draw from, and each clip table
 *  their bodies sample, by name. */
export interface FieldedUnits {
  units: object;
  effects: object;
  tables: ReadonlyMap<string, Float32Array>;
}

/** `kinds` of the session's unit `catalog` under `rules` (a scenario's
 *  rules, JSON), each side wearing `factions`' looks (blue, red; none: one
 *  look each), drawn from `runtime`'s art. The module must be loaded. */
export async function fieldUnits(input: {
  rules: string;
  catalog: CatalogView;
  kinds: readonly string[];
  factions?: readonly [string, string];
  runtime: RuntimeFiles;
}): Promise<FieldedUnits> {
  const catalog = input.runtime.catalog;
  if (!catalog) throw new Error("units were fielded before the runtime catalog was loaded");
  const fielding = new Fielding(
    JSON.stringify(presentationSettings),
    input.rules,
    JSON.stringify(input.catalog),
    JSON.stringify(catalog),
    JSON.stringify(input.kinds),
    input.factions && JSON.stringify(input.factions),
  );
  try {
    const files = await input.runtime.contents(fielding.contents());
    const lengths = Uint32Array.from(files, (f) => f.length);
    const bytes = new Uint8Array(lengths.reduce((n, l) => n + l, 0));
    let at = 0;
    for (const f of files) {
      bytes.set(f, at);
      at += f.length;
    }
    const { units, effects, tables } = JSON.parse(fielding.resolve(bytes, lengths)) as {
      units: object;
      effects: object;
      tables: string[];
    };
    return { units, effects, tables: new Map(tables.map((name, i) => [name, fielding.table(i)])) };
  } finally {
    fielding.free();
  }
}
