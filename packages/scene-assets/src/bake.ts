// Catalog → runtime bundles. Pure: the caller supplies source bytes and writes
// the outputs. Skeleton clips bake first, since skinned bodies are laid out
// on their skeleton's joint order and bounded by its clips.

import { bundleHash, encodeBundle } from "./codec.ts";
import {
  KIT_BUNDLE_MAX_BYTES,
  UNIT_BUNDLE_KIND,
  bundlePath,
  templateLibraryPath,
  type Bundle,
  type Catalog,
  type Finding,
  type RuntimeCatalog,
  type SideTints,
  type AppearanceUnit,
  type SkeletonClips,
  type StaticBundle,
} from "./schema.ts";
import { bindInteriors, interiorTexture, type InteriorSheets } from "./interior.ts";
import { encodeTemplateLibrary, sealTemplateLibrary } from "./templateLibrary.ts";
import {
  packTemplateSets,
  type PhysicalTemplates,
  type TemplateCatalogue,
  type TemplateLibraryStats,
  type TemplateSetInput,
} from "./templateSource.ts";
import {
  hasErrors,
  typeAppearanceFindings,
  validateAppearance,
  validateSkeleton,
  type Stats,
  type ValidationContext,
} from "./validate.ts";
import type { MountDraws } from "./units.ts";

export interface BakeReport {
  name: string;
  /** A skeleton, an appearance, the unit types' appearance names, or the
   *  template art library packed from the city sets. */
  what: "skeleton" | "appearance" | "types" | "library";
  findings: Finding[];
  stats: Stats | null;
  /** The template art library: what each template draws. */
  templates?: TemplateLibraryStats;
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

/** What the bake judges art against: the simulation's bodies and, for a
 *  catalog with city sets, the physical template catalogues they dress. */
export interface BakeContext extends Omit<ValidationContext, "tolerances"> {
  templates?: {
    catalogues: readonly TemplateCatalogue[];
    physical: PhysicalTemplates;
  };
}

/** A kit's bundle against its byte budget. */
export function kitBytesFindings(name: string, bytes: number): Finding[] {
  return bytes > KIT_BUNDLE_MAX_BYTES
    ? [
        {
          code: "kit.bytes",
          severity: "error",
          message: `${name}: the kit's bundle is ${(bytes / 2 ** 20).toFixed(1)} MiB, over its budget of ${KIT_BUNDLE_MAX_BYTES / 2 ** 20} MiB`,
          fix: "thin the modules' tiers, shrink or share textures, or split the set",
        },
      ]
    : [];
}

/** The template art library's name in a bake report. */
export const TEMPLATE_LIBRARY_REPORT = "template art";

export async function bakeCatalog(
  catalog: Catalog,
  readSource: (path: string) => Promise<Uint8Array>,
  context: BakeContext,
  only?: string[],
): Promise<BakeResult> {
  const runtime: RuntimeCatalog = { sides: catalog.sides, skeletons: {}, appearances: {} };
  const files = new Map<string, Uint8Array>();
  const reports: BakeReport[] = [];
  const clipsById = new Map<string, SkeletonClips>();
  const kits = new Map<string, { bundle: StaticBundle; hash: string }>();
  const wanted = (name: string) => !only?.length || only.includes(name);
  const neededSkeletons = new Set(
    Object.entries(catalog.appearances)
      .filter(([name]) => wanted(name))
      .map(([, a]) => a.skeleton)
      .filter((s): s is string => !!s),
  );
  // The interior sheets, each read and laid out once, when a room first asks.
  const sheetTextures = new Map<string, ReturnType<InteriorSheets>>();
  const sheets: InteriorSheets = (sheet) => {
    let texture = sheetTextures.get(sheet);
    if (!texture) {
      const path = catalog.interiors?.[sheet];
      texture = path
        ? readSource(path)
            .then(interiorTexture)
            .catch((error: unknown) => `${path} ${error instanceof Error ? error.message : error}`)
        : Promise.resolve("the catalog has no source for");
      sheetTextures.set(sheet, texture);
    }
    return texture;
  };
  const emit = async (bundle: Parameters<typeof encodeBundle>[0]) => {
    const bytes = encodeBundle(bundle);
    const hash = await bundleHash(bytes);
    files.set(bundlePath(hash), bytes);
    return { hash, bytes: bytes.byteLength };
  };

  for (const id of Object.keys(catalog.skeletons).sort()) {
    if (!wanted(id) && !neededSkeletons.has(id)) continue;
    const entry = catalog.skeletons[id];
    const result = await validateSkeleton(id, entry, await readSource(entry.source));
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
    if (result.bundle && result.bundle.kind !== "clips")
      result.findings.push(...(await bindInteriors(name, result.bundle, sheets)));
    const out = result.bundle && !hasErrors(result.findings) ? await emit(result.bundle) : null;
    if (entry.unit === "kit" && out) result.findings.push(...kitBytesFindings(name, out.bytes));
    if (entry.unit === "kit" && result.bundle?.kind === "static" && out)
      kits.set(name, { bundle: result.bundle, hash: out.hash });
    if (result.bundle && out)
      runtime.appearances[name] = {
        unit: entry.unit,
        kind: result.bundle.kind as RuntimeCatalog["appearances"][string]["kind"],
        bundle: out.hash,
        ...(entry.skeleton ? { skeleton: entry.skeleton } : {}),
        ...(entry.scenery ? { scenery: entry.scenery } : {}),
        ...(entry.footprint_half_m ? { footprint_half_m: entry.footprint_half_m } : {}),
        ...(entry.mounts ? { mounts: entry.mounts } : {}),
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
  // The city sets, packed into the one template art library. A bake of named
  // entries leaves it out: it is whole or absent.
  const sets = Object.entries(catalog.city_sets ?? {});
  if (sets.length && !only?.length) {
    if (!context.templates)
      throw new Error("the catalog has city sets: the bake needs the physical template catalogues");
    const inputs: TemplateSetInput[] = [];
    for (const [name, entry] of sets)
      inputs.push({
        name,
        entry,
        bytes: await readSource(entry.templates),
        kit: catalog.appearances[entry.kit]?.unit === "kit" ? (kits.get(entry.kit) ?? null) : null,
      });
    const packed = packTemplateSets(
      inputs,
      context.templates.catalogues,
      context.templates.physical,
      catalog.tolerances.ground_m,
    );
    let out: { hash: string; bytes: number } | null = null;
    if (packed.library) {
      const library = await sealTemplateLibrary(packed.library);
      const bytes = encodeTemplateLibrary(library);
      const hash = await bundleHash(bytes);
      files.set(templateLibraryPath(hash), bytes);
      runtime.templates = { library: hash, art_hash: library.art_hash, covers: library.covers };
      out = { hash, bytes: bytes.byteLength };
    }
    reports.push({
      name: TEMPLATE_LIBRARY_REPORT,
      what: "library",
      findings: packed.findings,
      stats: null,
      ...(packed.stats ? { templates: packed.stats } : {}),
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
  /** A vehicle's rig per mount name, as its catalog entry declares. */
  mounts?: MountDraws;
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
      ...(entry.mounts ? { mounts: entry.mounts } : {}),
    };
  }
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(runtime)));
  return files;
}
