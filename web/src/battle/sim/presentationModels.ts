/** External art and poses admitted to the canonical Rust frame. */
import type { ModelInstance, ModelPose } from "@packages/battle-renderer/src/models/modelInstances";
import type { Articulation } from "@packages/scene-assets/src/articulation";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { splitTextures } from "@packages/scene-assets/src/codec";

export interface LabAsset {
  name: string;
  /** Inflated scene-assets container; texture bytes belong to rendering. */
  bundle: Uint8Array;
  skeleton?: Uint8Array;
}

export interface ModelDetail {
  lod_px: readonly [number, number, number];
  impostor_px: number;
  shadow_caster_px?: number;
}

/** Existing loaded art uses the same binary container as fetched runtime art.
 * Textures remain with the renderer; presentation takes geometry and poses. */
export function labAssetsOf(installed: InstalledAppearances, names: Iterable<string>): LabAsset[] {
  const skeletons = new Map<string, Uint8Array>();
  return [...new Set(names)].map((name) => {
    const entry = installed.appearances.get(name);
    if (!entry) throw new Error(`lab appearance ${name} is not installed`);
    const { bundle } = entry;
    let skeleton: Uint8Array | undefined;
    if (bundle.kind === "skinned") {
      skeleton = skeletons.get(bundle.skeleton);
      if (!skeleton) {
        const clips = installed.skeletons.get(bundle.skeleton);
        if (!clips) throw new Error(`lab skeleton ${bundle.skeleton} is not installed`);
        skeleton = splitTextures(clips).content;
        skeletons.set(bundle.skeleton, skeleton);
      }
    }
    return { name, bundle: splitTextures(bundle).content, skeleton };
  });
}

export type LabModel = Omit<ModelInstance, "pose"> & {
  pose:
    | ModelPose
    | { kind: "rest" }
    | { kind: "clip"; clip: string; phase: number }
    | { kind: "articulated"; articulation: Articulation };
};

/** JSON only at request admission. Typed Joint data is a flat numeric array. */
export function labModelsJson(models: readonly LabModel[]): string {
  return JSON.stringify(
    models.map((m) => ({
      ...m,
      pose: m.pose.kind === "posed" ? { ...m.pose, palette: Array.from(m.pose.palette) } : m.pose,
    })),
  );
}
