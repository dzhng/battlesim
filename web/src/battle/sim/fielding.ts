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

type FieldingInput = Parameters<typeof createFielding>[0];

/** One battle's fielding, kept across its kinds changes so that Rust keeps
 *  the facts it derived from art it has already read. Whoever owns the battle
 *  owns this and frees it. */
export class FieldingOwner {
  private made: { fielding: Fielding; input: FieldingInput } | null = null;
  private tail: Promise<void> = Promise.resolve();
  private closed = false;
  private last: { input: FieldingInput; result: FieldedUnits } | null = null;

  field(input: FieldingInput): Promise<FieldedUnits> {
    if (this.closed) return Promise.reject(new Error("fielding owner is closed"));
    const run = this.tail.then(async () => {
      if (this.closed) throw new Error("fielding owner is closed");
      if (this.last && sameSource(this.last.input, input) && sameKinds(this.last.input, input))
        return this.last.result;
      // Rules, catalogs and factions are fixed when Rust's fielding is made:
      // a change to any of them starts over.
      if (this.made && !sameSource(this.made.input, input)) {
        this.made.fielding.free();
        this.made = null;
      }
      const { fielding } = (this.made ??= { fielding: createFielding(input), input });
      fielding.set_kinds(JSON.stringify(input.kinds));
      const result = await resolveFielding(fielding, input);
      this.last = { input, result };
      return result;
    });
    this.tail = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }

  free(): void {
    this.closed = true;
    this.last = null;
    void this.tail.then(() => {
      this.made?.fielding.free();
      this.made = null;
    });
  }
}

function sameSource(a: FieldingInput, b: FieldingInput): boolean {
  return (
    a.rules === b.rules &&
    a.catalog === b.catalog &&
    a.runtime === b.runtime &&
    a.runtime.catalog === b.runtime.catalog &&
    a.factions?.[0] === b.factions?.[0] &&
    a.factions?.[1] === b.factions?.[1]
  );
}

function sameKinds(a: FieldingInput, b: FieldingInput): boolean {
  return a.kinds.length === b.kinds.length && a.kinds.every((kind, i) => kind === b.kinds[i]);
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
  const owner = new FieldingOwner();
  try {
    return await owner.field(input);
  } finally {
    owner.free();
  }
}

function createFielding(input: {
  rules: string;
  catalog: CatalogView;
  kinds: readonly string[];
  factions?: readonly [string, string];
  runtime: RuntimeFiles;
}): Fielding {
  const catalog = input.runtime.catalog;
  if (!catalog) throw new Error("units were fielded before the runtime catalog was loaded");
  return new Fielding(
    JSON.stringify(presentationSettings),
    input.rules,
    JSON.stringify(input.catalog),
    JSON.stringify(catalog),
    JSON.stringify(input.kinds),
    input.factions && JSON.stringify(input.factions),
  );
}

async function resolveFielding(
  fielding: Fielding,
  input: {
    runtime: RuntimeFiles;
  },
): Promise<FieldedUnits> {
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
  return {
    units,
    effects,
    tables: new Map(tables.map((name, i) => [name, fielding.table(i)])),
  };
}
