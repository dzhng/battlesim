// One GLB judged on its own: the CLI's `validate <glb>` and the workbench's
// drop zone. A file that is a catalog source is judged with its catalog entry
// (and a vehicle against every unit type that draws it); any other file gets
// an ad hoc entry from its content and the caller's options, and a vehicle is
// fitted to the unit type the caller names. Skinned files bring their own
// clips unless a clip source is given.

import { parseGlb } from "./glb.ts";
import { SCENERY_KINDS } from "./scenery.ts";
import {
  UNIT_BUNDLE_KIND,
  type AppearanceEntry,
  type Bundle,
  type BundleKind,
  type AppearanceUnit,
  type Catalog,
  type SkeletonClips,
  type SkeletonEntry,
  type PoseRef,
} from "./schema.ts";
import {
  validateAppearance,
  validateSkeleton,
  type Validation,
  type ValidationContext,
} from "./validate.ts";
import type { MountDraws } from "./units.ts";

export interface LooseOptions {
  /** What it draws; inferred from the file when omitted. */
  unit?: AppearanceUnit;
  /** A vehicle's unit type to fit it to; a catalog source is fitted to every
   *  type that names it. */
  type?: string;
  /** For `unit: "scenery"`: the scenery kind (`SCENERY_KINDS`). */
  scenery?: string;
  /** Basis yaw in degrees; the catalog's, else 0. A wrong yaw shows as a
   *  model facing sideways in the workbench's front view. */
  yaw?: number;
  /** Clip names that loop, outside the catalog; every other clip is one-shot. */
  loops?: string[];
  /** A separate clip source for a skinned body. */
  clips?: { path: string; bytes: Uint8Array };
}

export interface LooseResult {
  path: string;
  unit: AppearanceUnit;
  kind: BundleKind;
  yaw: number;
  /** The catalog entry this file is a source of, if any. */
  entryName: string | null;
  /** A skinned body's clips: the skeleton the body is laid out on. */
  clips: (Validation<SkeletonClips> & { path: string; id: string }) | null;
  /** A vehicle's rig per mount id: its catalog entry's, or for another
   *  file fitted to a type, the rigs that type's own model declares. */
  mounts: MountDraws | null;
  /** The file's own judgement; for a catalog skeleton source, its clips. */
  appearance: Validation<Bundle> | null;
}

/** What a GLB most likely draws, from its content alone: a skin is a
 *  soldier, running gear a vehicle, anything else scenery (of the kind the
 *  caller names). */
export function inferUnit(bytes: Uint8Array): AppearanceUnit {
  const { json } = parseGlb(bytes);
  const names: string[] = (json.nodes ?? []).map((n: { name?: string }) => n.name ?? "");
  if ((json.skins ?? []).length) return "soldier";
  if (names.some((n) => n.startsWith("wheel_") || n.startsWith("track_"))) return "vehicle";
  return "scenery";
}

async function looseSkeletonResult(
  path: string,
  bytes: Uint8Array,
  named: [string, SkeletonEntry],
): Promise<LooseResult> {
  const [id, entry] = named;
  const clips = await validateSkeleton(id, entry, bytes);
  return {
    path,
    unit: "soldier",
    kind: "skinned",
    yaw: entry.basis_yaw_deg,
    entryName: id,
    clips: { ...clips, path, id },
    mounts: null,
    appearance: null,
  };
}

function looseEntry(
  path: string,
  named: [string, AppearanceEntry] | undefined,
  unit: AppearanceUnit,
  kind: BundleKind,
  yaw: number,
  options: LooseOptions,
  typeMounts: MountDraws | undefined,
): AppearanceEntry {
  if (named) return named[1];
  if (kind === "static") {
    return {
      unit,
      states: {
        [(options.scenery && SCENERY_KINDS[options.scenery]?.states[0]) || "default"]: path,
      },
      basis_yaw_deg: yaw,
      ...(options.scenery ? { scenery: options.scenery } : {}),
    };
  }
  return { unit, source: path, basis_yaw_deg: yaw, ...(typeMounts ? { mounts: typeMounts } : {}) };
}

async function looseFiles(
  path: string,
  bytes: Uint8Array,
  entry: AppearanceEntry,
  readSource: (path: string) => Promise<Uint8Array>,
): Promise<Record<string, Uint8Array>> {
  const files: Record<string, Uint8Array> = { [path]: bytes };
  for (const state of Object.values(entry.states ?? {}))
    if (!(state in files)) files[state] = await readSource(state);
  return files;
}

async function looseClips(
  path: string,
  bytes: Uint8Array,
  named: [string, AppearanceEntry] | undefined,
  catalog: Pick<Catalog, "skeletons">,
  options: LooseOptions,
  yaw: number,
  readSource: (path: string) => Promise<Uint8Array>,
): Promise<{
  result: LooseResult["clips"];
  skeleton?: { clips: SkeletonClips; aim_reference: PoseRef };
}> {
  const skeletonEntry = named?.[1].skeleton ? catalog.skeletons[named[1].skeleton] : null;
  const clipsPath = options.clips?.path ?? skeletonEntry?.source ?? path;
  const clipBytes =
    options.clips?.bytes ?? (clipsPath === path ? bytes : await readSource(clipsPath));
  let declared: SkeletonEntry | null = skeletonEntry && !options.clips ? skeletonEntry : null;
  if (!declared) {
    const loops = options.loops ?? [];
    const names: string[] = (parseGlb(clipBytes).json.animations ?? []).map(
      (a: { name: string }) => a.name,
    );
    declared = {
      source: clipsPath,
      basis_yaw_deg: yaw,
      sample_hz: 30,
      aim_reference: { clip: "stand_aim", phase: 0 },
      clips: Object.fromEntries(names.map((n) => [n, { loop: loops.includes(n) }])),
    };
  }
  const id = named?.[1].skeleton ?? "adhoc";
  const clips = await validateSkeleton(id, declared, clipBytes);
  return {
    result: { ...clips, path: clipsPath, id },
    ...(clips.preview
      ? { skeleton: { clips: clips.preview, aim_reference: declared.aim_reference } }
      : {}),
  };
}

function namedAppearance(catalog: Pick<Catalog, "appearances">, path: string) {
  return Object.entries(catalog.appearances).find(
    ([, e]) => e.source === path || Object.values(e.states ?? {}).includes(path),
  ) as [string, AppearanceEntry] | undefined;
}

function typeMounts(
  catalog: Pick<Catalog, "appearances">,
  context: ValidationContext,
  options: LooseOptions,
): MountDraws | undefined {
  if (!options.type || !context.authority.units.has(options.type)) return undefined;
  const typeModel = context.authority.units.type(options.type).appearance;
  return typeModel ? catalog.appearances[typeModel]?.mounts : undefined;
}

function looseResult(
  path: string,
  named: [string, AppearanceEntry] | undefined,
  unit: AppearanceUnit,
  kind: BundleKind,
  yaw: number,
  entry: AppearanceEntry,
): LooseResult {
  return {
    path,
    unit,
    kind,
    yaw,
    entryName: named?.[0] ?? null,
    clips: null,
    mounts: entry.mounts ?? null,
    appearance: null,
  };
}

export async function validateLoose(
  path: string,
  bytes: Uint8Array,
  catalog: Pick<Catalog, "skeletons" | "appearances">,
  context: ValidationContext,
  options: LooseOptions = {},
  readSource: (path: string) => Promise<Uint8Array> = async (p) => {
    throw new Error(`no source reader for ${p}`);
  },
): Promise<LooseResult> {
  const named = namedAppearance(catalog, path);
  const skeletonNamed = Object.entries(catalog.skeletons).find(([, s]) => s.source === path);
  if (skeletonNamed && !options.unit) return looseSkeletonResult(path, bytes, skeletonNamed);
  const unit = options.unit ?? named?.[1].unit ?? inferUnit(bytes);
  const kind = UNIT_BUNDLE_KIND[unit];
  if (!kind) throw new Error(`unknown unit ${unit}`);
  const yaw = options.yaw ?? named?.[1].basis_yaw_deg ?? 0;
  const entry = looseEntry(
    path,
    named,
    unit,
    kind,
    yaw,
    options,
    typeMounts(catalog, context, options),
  );
  const result = looseResult(path, named, unit, kind, yaw, entry);
  const files = await looseFiles(path, bytes, entry, readSource);
  let skeleton: { clips: SkeletonClips; aim_reference: PoseRef } | undefined;
  if (kind === "skinned") {
    const clips = await looseClips(path, bytes, named, catalog, options, yaw, readSource);
    result.clips = clips.result;
    // Fit is measured on whatever clips built, so body findings show even
    // when the clips have errors.
    if (!clips.skeleton) return result;
    skeleton = clips.skeleton;
  }
  result.appearance = await validateAppearance(
    {
      name: named?.[0] ?? path,
      entry,
      files,
      skeleton,
      types: options.type ? [options.type] : undefined,
    },
    context,
  );
  return result;
}
