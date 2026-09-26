// What the battle frame draws of the models layer: one record per drawn
// model, naming an installed appearance, where it stands and how it is posed.
// Poses come from the pose driver (`poseDriver.ts`); `poseFrameInstances`
// turns a pose frame into these records.

import type { Articulation } from "@packages/scene-assets/src/articulation";
import type { PoseFrame, UnitKindName } from "./poseDriver";

export interface SkinnedModelPose {
  kind: "skinned";
  clip: string;
  phase: number;
  /** A second clip mixed over the first by `weight` (a crossfade). */
  blend: { clip: string; phase: number; weight: number } | null;
}

export interface ArticulatedModelPose {
  kind: "articulated";
  articulation: Articulation;
}

export interface StaticModelPose {
  kind: "static";
  state: string;
}

export type ModelPose = SkinnedModelPose | ArticulatedModelPose | StaticModelPose;

export interface ModelInstance {
  /** An appearance name in the installed catalog generation. */
  appearance: string;
  x: number;
  y: number;
  z: number;
  /** Heading of the model's +X, radians counter-clockwise from world +X. */
  yaw: number;
  pose: ModelPose;
  /** The side's tint on tint-masked surfaces (`AppearanceCatalog`); none keeps
   *  the authored colours. */
  tint?: readonly [number, number, number];
  /** Mesh tier, 0 finest; defaults to 0. */
  tier?: number;
  highlight?: boolean;
}

/** One model per posed soldier and vehicle, by the appearance for its kind. */
export function poseFrameInstances(
  frame: PoseFrame,
  appearanceOf: (kind: UnitKindName) => string | null,
): ModelInstance[] {
  const out: ModelInstance[] = [];
  for (const s of frame.soldiers) {
    const appearance = appearanceOf(s.kind);
    if (!appearance) continue;
    out.push({
      appearance,
      x: s.position[0],
      y: s.position[1],
      z: s.position[2],
      yaw: s.facing,
      pose: { kind: "skinned", clip: s.clip, phase: s.phase, blend: s.blend },
    });
  }
  for (const v of frame.vehicles) {
    const appearance = appearanceOf(v.kind);
    if (!appearance) continue;
    out.push({
      appearance,
      x: v.position[0],
      y: v.position[1],
      z: v.position[2],
      yaw: v.yaw,
      pose: { kind: "articulated", articulation: v.articulation },
    });
  }
  return out;
}
