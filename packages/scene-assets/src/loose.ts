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
  /** A vehicle's rig per mount name: its catalog entry's, or for another
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
  const named = Object.entries(catalog.appearances).find(
    ([, e]) => e.source === path || Object.values(e.states ?? {}).includes(path),
  );
  const skeletonNamed = Object.entries(catalog.skeletons).find(([, s]) => s.source === path);
  if (skeletonNamed && !options.unit) {
    const [id, entry] = skeletonNamed;
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
  const unit = options.unit ?? named?.[1].unit ?? inferUnit(bytes);
  const kind = UNIT_BUNDLE_KIND[unit];
  if (!kind) throw new Error(`unknown unit ${unit}`);
  const yaw = options.yaw ?? named?.[1].basis_yaw_deg ?? 0;
  // A new model for a type is rigged like the type's own model.
  const units = context.authority.units;
  const typeModel =
    options.type && units.has(options.type) ? units.type(options.type).appearance : undefined;
  const typeMounts = typeModel ? catalog.appearances[typeModel]?.mounts : undefined;
  const entry: AppearanceEntry =
    named?.[1] ??
    (kind === "static"
      ? {
          unit,
          states: {
            [(options.scenery && SCENERY_KINDS[options.scenery]?.states[0]) || "default"]: path,
          },
          basis_yaw_deg: yaw,
          ...(options.scenery ? { scenery: options.scenery } : {}),
        }
      : { unit, source: path, basis_yaw_deg: yaw, ...(typeMounts ? { mounts: typeMounts } : {}) });
  const result: LooseResult = {
    path,
    unit,
    kind,
    yaw,
    entryName: named?.[0] ?? null,
    clips: null,
    mounts: entry.mounts ?? null,
    appearance: null,
  };
  const files: Record<string, Uint8Array> = { [path]: bytes };
  // A catalog entry of several states (a wreck and its pieces) is judged whole.
  for (const state of Object.values(entry.states ?? {}))
    if (!(state in files)) files[state] = await readSource(state);
  let skeleton;
  if (kind === "skinned") {
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
    result.clips = { ...clips, path: clipsPath, id };
    // Fit is measured on whatever clips built, so body findings show even
    // when the clips have errors.
    if (!clips.preview) return result;
    skeleton = { clips: clips.preview, aim_reference: declared.aim_reference };
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
