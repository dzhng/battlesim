/** What fielded units draw from, resolved by Rust (`presentation::fielded`):
 *  the page only reads the files it names (`fixtures/presentation.json`, the
 *  runtime catalog, its bundles and skeleton clips). A battle loads the
 *  result into its world; a showcase shows one unit from it. */
import { presentationSettings } from "@web/presentationSettings";
import type { AppearanceLibrary } from "@packages/scene-assets/src/loader";
import type { CatalogView } from "@packages/scene-assets/src/units";
import { Fielding, type Fielded } from "@wasm/renderer_browser";
import { diagnosticsEvent } from "@web/diagnostics";
import { releaseTransport } from "../transport";
import { artFrame, artWork } from "../artWork";

/** The runtime files a page's appearances were loaded from: the runtime
 *  catalog and its bundle contents by hash (`AppearanceLibrary`). */
export type RuntimeFiles = Pick<AppearanceLibrary, "catalog" | "contents">;

/** A Rust-owned immutable result. Only the hull query and work report cross
 *  the boundary; presentation borrows its descriptions directly. */
export type FieldedUnits = Fielded;

type FieldingInput = Parameters<typeof createFielding>[0];

/** One battle's fielding, kept across its kinds changes so that Rust keeps
 *  the facts it derived from art it has already read. Whoever owns the battle
 *  owns this and frees it. */
export class FieldingOwner {
  private made: { fielding: Fielding; source: FieldingSource } | null = null;
  private tail: Promise<void> = Promise.resolve();
  private closed = false;
  private last: { source: FieldingSource; kinds: readonly string[]; result: FieldedUnits } | null =
    null;

  field(input: FieldingInput): Promise<FieldedUnits> {
    if (this.closed) return Promise.reject(new Error("fielding owner is closed"));
    const run = this.tail.then(async () => {
      if (this.closed) throw new Error("fielding owner is closed");
      const source = sourceOf(input);
      if (
        this.last &&
        sameSource(this.last.source, source) &&
        sameKinds(this.last.kinds, input.kinds)
      )
        return this.last.result;
      // Rules, catalogs and factions are fixed when Rust's fielding is made:
      // a change to any of them starts over.
      if (this.made && !sameSource(this.made.source, source)) {
        this.made.fielding.free();
        this.made = null;
      }
      const { fielding } = (this.made ??= { fielding: createFielding(input), source });
      fielding.set_kinds(JSON.stringify(input.kinds));
      const result = await resolveFielding(fielding, input);
      // Publishing and battle staging share the art-work budget. Yield when
      // resolving spent it; small growths can finish in this same frame.
      if (artWork.exhausted()) await artFrame();
      if (this.closed) throw new Error("fielding owner is closed");
      this.last = { source, kinds: input.kinds, result };
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

/** What Rust's fielding is made from. The runtime catalog is held as the
 *  object it was then: the library that serves it is one object for the page's
 *  life, and loads a new catalog into itself. */
interface FieldingSource {
  rules: string;
  catalog: CatalogView;
  runtime: RuntimeFiles;
  runtimeCatalog: RuntimeFiles["catalog"];
  factions?: readonly [string, string];
}

function sourceOf(input: FieldingInput): FieldingSource {
  return {
    rules: input.rules,
    catalog: input.catalog,
    runtime: input.runtime,
    runtimeCatalog: input.runtime.catalog,
    factions: input.factions,
  };
}

function sameSource(a: FieldingSource, b: FieldingSource): boolean {
  return (
    a.rules === b.rules &&
    a.catalog === b.catalog &&
    a.runtime === b.runtime &&
    a.runtimeCatalog === b.runtimeCatalog &&
    a.factions?.[0] === b.factions?.[0] &&
    a.factions?.[1] === b.factions?.[1]
  );
}

function sameKinds(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((kind, i) => kind === b[i]);
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
  const pending = JSON.parse(fielding.pending_contents()) as {
    bundle: string;
    skeleton?: string | null;
  }[];
  const hashes = (content: (typeof pending)[number]) => [
    content.bundle,
    ...(content.skeleton ? [content.skeleton] : []),
  ];
  const uses = new Map<string, number>();
  for (const content of pending)
    for (const hash of hashes(content)) uses.set(hash, (uses.get(hash) ?? 0) + 1);
  const sources = new Map<string, Uint8Array<ArrayBuffer>>();
  try {
    // Inflate independent inputs together through the existing reader. The
    // browser streams gzip asynchronously; only absorption spends frame work.
    const reads = await Promise.allSettled(
      [...uses.keys()].map(async (hash) => {
        const [file] = await input.runtime.contents([hash]);
        sources.set(hash, file);
      }),
    );
    const refused = reads.find((read) => read.status === "rejected");
    if (refused?.status === "rejected") throw refused.reason;
    for (const content of pending) {
      const needed = hashes(content);
      if (artWork.exhausted()) await artFrame();
      artWork.run(() => {
        const files = needed.map((hash) => sources.get(hash)!);
        const lengths = Uint32Array.from(files, (file) => file.length);
        const bytes = new Uint8Array(files.reduce((sum, file) => sum + file.length, 0));
        let at = 0;
        for (const file of files) {
          bytes.set(file, at);
          at += file.length;
        }
        try {
          fielding.absorb(JSON.stringify(content), bytes, lengths);
        } finally {
          releaseTransport(bytes);
        }
      });
      for (const hash of needed) {
        const left = uses.get(hash)! - 1;
        uses.set(hash, left);
        if (!left) {
          releaseTransport(sources.get(hash)!);
          sources.delete(hash);
        }
      }
    }
  } finally {
    for (const file of sources.values()) releaseTransport(file);
  }
  if (artWork.exhausted()) await artFrame();
  return artWork.run(() => {
    const result = fielding.resolve(new Uint8Array(), new Uint32Array());
    diagnosticsEvent("battle fielded", JSON.parse(result.work()));
    return result;
  });
}
