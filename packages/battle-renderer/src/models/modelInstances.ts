// What the battle frame draws of the models layer: one record per drawn
// model, naming an installed appearance, where it stands and how it is posed,
// and one per corpse. Poses come from the pose driver (`poseDriver.ts`);
// `poseFrameInstances` and `corpseInstances` turn a pose frame into these.

import type { Side } from "@packages/scene-assets/src/schema";
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

/** A skinned appearance lying at its bundle's `corpse_pose`: a static mesh. */
export interface CorpseModelPose {
  kind: "corpse";
}

export type ModelPose = SkinnedModelPose | ArticulatedModelPose | StaticModelPose | CorpseModelPose;

export interface ModelInstance {
  /** An appearance name in the installed catalog generation. */
  appearance: string;
  x: number;
  y: number;
  z: number;
  /** Heading of the model's +X, radians counter-clockwise from world +X. */
  yaw: number;
  /** Per-axis scale in the model's own frame, applied before `yaw`: a static
   *  prop fitted to its placed box (`propAppearance.ts`). None is 1. */
  scale?: readonly [number, number, number];
  pose: ModelPose;
  /** The side's tint on tint-masked surfaces (`AppearanceCatalog`); none keeps
   *  the authored colours. */
  tint?: readonly [number, number, number];
  /** Mesh tier, 0 finest. Omitted, the frame picks it by projected size and
   *  draws a far model as an impostor. */
  tier?: number;
  highlight?: boolean;
}

/** A fallen soldier at rest: his appearance's static corpse mesh (the end of
 *  its death clip, baked once at install), never skinned. */
export interface CorpseInstance {
  appearance: string;
  x: number;
  y: number;
  z: number;
  yaw: number;
  tint?: readonly [number, number, number];
}

/** An appearance and side tint for a unit kind on a side (`AppearanceCatalog.resolve`). */
export type ResolveAppearance = (
  kind: UnitKindName,
  side: Side,
) => { appearance: string; tint: readonly [number, number, number] } | null;

/** One model per posed soldier and vehicle, by the appearance for its kind and
 *  side, highlighted when its unit is in `selected`. Writes into `out`
 *  (reusing its records, so a frame allocates nothing once warm) and returns it. */
export function poseFrameInstances(
  out: ModelInstance[],
  frame: PoseFrame,
  resolve: ResolveAppearance,
  selected: ReadonlySet<number> = NO_SELECTION,
): ModelInstance[] {
  let n = 0;
  const record = (): ModelInstance => {
    let m = out[n];
    if (!m) {
      m = { appearance: "", x: 0, y: 0, z: 0, yaw: 0, pose: REST_POSE, highlight: false };
      out[n] = m;
    }
    n++;
    return m;
  };
  for (const s of frame.soldiers) {
    const resolved = resolve(s.kind, s.side);
    if (!resolved) continue;
    const m = record();
    m.appearance = resolved.appearance;
    m.tint = resolved.tint;
    m.x = s.position[0];
    m.y = s.position[1];
    m.z = s.position[2];
    m.yaw = s.facing;
    let pose = m.pose;
    if (pose.kind !== "skinned" || pose === REST_POSE)
      pose = m.pose = { kind: "skinned", clip: "", phase: 0, blend: null };
    pose.clip = s.clip;
    pose.phase = s.phase;
    pose.blend = s.blend;
    m.highlight = selected.has(s.unit);
  }
  for (const v of frame.vehicles) {
    const resolved = resolve(v.kind, v.side);
    if (!resolved) continue;
    const m = record();
    m.appearance = resolved.appearance;
    m.tint = resolved.tint;
    m.x = v.position[0];
    m.y = v.position[1];
    m.z = v.position[2];
    m.yaw = v.yaw;
    const pose =
      m.pose.kind === "articulated"
        ? m.pose
        : (m.pose = { kind: "articulated", articulation: v.articulation });
    pose.articulation = v.articulation;
    m.highlight = selected.has(v.unit);
  }
  out.length = n;
  return out;
}

/** The static corpses of a pose frame (call when `corpsesVersion` changes). */
export function corpseInstances(frame: PoseFrame, resolve: ResolveAppearance): CorpseInstance[] {
  const out: CorpseInstance[] = [];
  for (const c of frame.corpses) {
    const resolved = resolve(c.kind, c.side);
    if (!resolved) continue;
    out.push({
      appearance: resolved.appearance,
      x: c.position[0],
      y: c.position[1],
      z: c.position[2],
      yaw: c.yaw,
      tint: resolved.tint,
    });
  }
  return out;
}

const NO_SELECTION: ReadonlySet<number> = new Set();
const REST_POSE: SkinnedModelPose = { kind: "skinned", clip: "", phase: 0, blend: null };
