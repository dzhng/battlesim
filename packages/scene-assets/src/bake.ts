// Catalog → runtime bundles. Pure: the caller supplies source bytes and writes
// the outputs. Skeleton clips bake first, since skinned bodies are laid out
// on their skeleton's joint order and bounded by its clips.

import { bundleHash, encodeBundle } from "./codec.ts";
import {
  UNIT_BUNDLE_KIND,
  bundlePath,
  type Bundle,
  type Catalog,
  type Finding,
  type RuntimeCatalog,
  type SideTints,
  type AppearanceUnit,
  type SkeletonClips,
} from "./schema.ts";
import {
  hasErrors,
  typeAppearanceFindings,
  validateAppearance,
  validateSkeleton,
  type Stats,
  type ValidationContext,
} from "./validate.ts";

export interface BakeReport {
  name: string;
  /** A skeleton, an appearance, or the unit types' appearance names. */
  what: "skeleton" | "appearance" | "types";
  findings: Finding[];
  stats: Stats | null;
  hash: string | null;
  bytes: number;
}

export interface BakeResult {
  runtime: RuntimeCatalog;
  /** Runtime files by path under the runtime directory. */
  files: Map<string, Uint8Array>;
  reports: BakeReport[];
  ok: boolean;
}

export async function bakeCatalog(
  catalog: Catalog,
  readSource: (path: string) => Promise<Uint8Array>,
  context: Omit<ValidationContext, "tolerances">,
  only?: string[],
): Promise<BakeResult> {
  const runtime: RuntimeCatalog = { sides: catalog.sides, skeletons: {}, appearances: {} };
  const files = new Map<string, Uint8Array>();
  const reports: BakeReport[] = [];
  const clipsById = new Map<string, SkeletonClips>();
  const wanted = (name: string) => !only?.length || only.includes(name);
  const neededSkeletons = new Set(
    Object.entries(catalog.appearances)
      .filter(([name]) => wanted(name))
      .map(([, a]) => a.skeleton)
      .filter((s): s is string => !!s),
  );
  const emit = async (bundle: Parameters<typeof encodeBundle>[0]) => {
    const bytes = encodeBundle(bundle);
    const hash = await bundleHash(bytes);
    files.set(bundlePath(hash), bytes);
    return { hash, bytes: bytes.byteLength };
  };

  for (const id of Object.keys(catalog.skeletons).sort()) {
    if (!wanted(id) && !neededSkeletons.has(id)) continue;
    const entry = catalog.skeletons[id];
    const result = await validateSkeleton(id, entry, await readSource(entry.source), context);
    const out = result.bundle ? await emit(result.bundle) : null;
    if (result.bundle && out) {
      clipsById.set(id, result.bundle);
      runtime.skeletons[id] = out.hash;
    }
    reports.push({
      name: id,
      what: "skeleton",
      findings: result.findings,
      stats: result.stats,
      hash: out?.hash ?? null,
      bytes: out?.bytes ?? 0,
    });
  }

  for (const name of Object.keys(catalog.appearances).sort()) {
    if (!wanted(name)) continue;
    const entry = catalog.appearances[name];
    const paths = entry.source ? [entry.source] : Object.values(entry.states ?? {});
    const sources: Record<string, Uint8Array> = {};
    for (const path of paths) sources[path] = await readSource(path);
    const clips = entry.skeleton ? clipsById.get(entry.skeleton) : undefined;
    if (UNIT_BUNDLE_KIND[entry.unit] === "skinned" && !clips) {
      reports.push({
        name,
        what: "appearance",
        findings: [
          {
            code: "structure.skeleton",
            severity: "error",
            message: `${name}: skeleton "${entry.skeleton ?? "(none)"}" did not bake`,
            fix: "name a catalog skeleton, and fix its findings first",
          },
        ],
        stats: null,
        hash: null,
        bytes: 0,
      });
      continue;
    }
    const result = await validateAppearance(
      {
        name,
        entry,
        files: sources,
        skeleton:
          clips && entry.skeleton
            ? { clips, aim_reference: catalog.skeletons[entry.skeleton].aim_reference }
            : undefined,
      },
      { ...context, tolerances: catalog.tolerances },
    );
    const out = result.bundle ? await emit(result.bundle) : null;
    if (result.bundle && out)
      runtime.appearances[name] = {
        unit: entry.unit,
        kind: result.bundle.kind as RuntimeCatalog["appearances"][string]["kind"],
        bundle: out.hash,
        ...(entry.skeleton ? { skeleton: entry.skeleton } : {}),
        ...(entry.scenery ? { scenery: entry.scenery } : {}),
        ...(entry.footprint_half_m ? { footprint_half_m: entry.footprint_half_m } : {}),
      };
    reports.push({
      name,
      what: "appearance",
      findings: result.findings,
      stats: result.stats,
      hash: out?.hash ?? null,
      bytes: out?.bytes ?? 0,
    });
  }
  // Every unit type draws appearances the catalog has.
  const types = typeAppearanceFindings(catalog.appearances, context.authority.units);
  if (types.length)
    reports.push({
      name: "units",
      what: "types",
      findings: types,
      stats: null,
      hash: null,
      bytes: 0,
    });
  return {
    runtime,
    files,
    reports,
    ok: reports.every((r) => !hasErrors(r.findings) && (r.hash || r.what === "types")),
  };
}

/** The runtime catalog's canonical text: sorted keys, two-space indent, trailing newline. */
export function runtimeCatalogText(runtime: RuntimeCatalog): string {
  const sort = (value: unknown): unknown =>
    value && typeof value === "object" && !Array.isArray(value)
      ? Object.fromEntries(
          Object.keys(value)
            .sort()
            .map((k) => [k, sort((value as Record<string, unknown>)[k])]),
        )
      : value;
  return `${JSON.stringify(sort(runtime), null, 2)}\n`;
}

/** One appearance to show without baking: a workbench preview. */
export interface PreviewEntry {
  name: string;
  unit: AppearanceUnit;
  scenery?: string;
  bundle: Exclude<Bundle, SkeletonClips>;
  /** A skinned body's clips, installed under their own id. */
  clips?: SkeletonClips;
}

/**
 * Runtime files (catalog plus content-addressed bundles) for previews, so the
 * workbench installs a dropped file through the one loader exactly as the
 * battle installs a baked catalog. Nothing is written to disk.
 */
export async function previewRuntime(
  entries: PreviewEntry[],
  sides: SideTints,
): Promise<Map<string, Uint8Array>> {
  const runtime: RuntimeCatalog = { sides, skeletons: {}, appearances: {} };
  const files = new Map<string, Uint8Array>();
  const emit = async (bundle: Bundle) => {
    const bytes = encodeBundle(bundle);
    const hash = await bundleHash(bytes);
    files.set(bundlePath(hash), bytes);
    return hash;
  };
  for (const entry of entries) {
    if (entry.clips) runtime.skeletons[entry.clips.id] = await emit(entry.clips);
    runtime.appearances[entry.name] = {
      unit: entry.unit,
      kind: entry.bundle.kind,
      bundle: await emit(entry.bundle),
      ...(entry.bundle.kind === "skinned" ? { skeleton: entry.bundle.skeleton } : {}),
      ...(entry.scenery ? { scenery: entry.scenery } : {}),
    };
  }
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(runtime)));
  return files;
}
