// What the battle frame draws of the models layer: one record per drawn
// model, naming an installed appearance, where it stands and how it is posed,
// and one per corpse. Poses come from the pose driver (`poseDriver.ts`);
// `poseFrameInstances` and `corpseInstances` turn a pose frame into these.

import type { Mat4 } from "math";
import type { Bundle, Side, SkeletonClips } from "@packages/scene-assets/src/schema";
import { REST_ARTICULATION, type Articulation } from "@packages/scene-assets/src/articulation";
import { isRgba, type Rgba } from "../mesh";
import type { PoseFrame } from "./poseDriver";

/** Floats in a model's GPU record (`modelLayer.ts` `ModelRecord`): placement
 *  (x, y, z, yaw), data (palette base, track scrolls, x-ray rgb), tint (rgb,
 *  card layer), scale (xyz, x-ray alpha). */
export const MODEL_RECORD_FLOATS = 16;

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
  /** A rigid motion of the state in its own frame, before its scale and
   *  yaw (a cook-off's wreck pieces, `effects/cookOff.ts`); none: as authored.
   *  A moving piece is never drawn as a card. */
  motion?: Mat4;
  /** A surface movers stand on (its body stops no mover: a bridge deck,
   *  rubble): it takes the ground paint, as the ground does (`modelFog`). */
  ground?: boolean;
}

/** A skinned appearance lying at its bundle's `corpse_pose`: a static mesh. */
export interface CorpseModelPose {
  kind: "corpse";
}

/** `presentation.overlay.xray`: the colours hidden parts are drawn in
 *  (`ModelInstance.xray`), the observing side's units in `own`, the
 *  selection in `selected`. */
export interface XrayStyle {
  own: Rgba;
  selected: Rgba;
}

/** `presentation.overlay.xray`, checked: both rgba, neither invisible. */
export function validateXray(x: XrayStyle): XrayStyle {
  const seen = (c: unknown) => isRgba(c) && c[3] > 0;
  if (!seen(x.own) || !seen(x.selected))
    throw new Error("presentation.overlay.xray: own and selected, rgba in [0, 1] with alpha > 0");
  return x;
}

export type ModelPose = SkinnedModelPose | ArticulatedModelPose | StaticModelPose | CorpseModelPose;

/** The authored resting model: shared by placement previews and far-model bakes. */
export function restingModelPose(bundle: Exclude<Bundle, SkeletonClips>): ModelPose {
  if (bundle.kind === "skinned") return { kind: "skinned", ...bundle.far_pose, blend: null };
  if (bundle.kind === "articulated")
    return { kind: "articulated", articulation: { ...REST_ARTICULATION } };
  return { kind: "static", state: bundle.states[0].name };
}

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
  /** The colour (rgba) its hidden parts are drawn in through whatever world
   *  stands in front of it (terrain, props, buildings, trees), as a flat
   *  silhouette over the frame; none: not x-rayed. The one highlight a model
   *  takes: its visible parts are never tinted. Incidental hidden fragments
   *  are suppressed by the frame's per-model coverage gate. Presentation
   *  chooses it (`XrayOf`). */
  xray?: readonly [number, number, number, number] | null;
  /** Placement preview colour and opacity. Draws the named mesh translucent,
   *  reading world depth, with no depth writes, shadow, impostor or x-ray. */
  ghost?: Rgba;
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

/** An appearance and side tint for a unit type on a side: a vehicle's model,
 *  or the variant soldier `id` in squad slot `slot` wears
 *  (`AppearanceCatalog.resolve`). */
export type ResolveAppearance = (
  kind: string,
  side: Side,
  id: number,
  slot: number,
  operatorMount?: number | null,
  activeMount?: number | null,
) => { appearance: string; tint: readonly [number, number, number] } | null;

/** The x-ray colour of a unit's models, by its side and id (null: none). */
export type XrayOf = (side: Side, unit: number) => ModelInstance["xray"];

/** One model per posed soldier and vehicle, by the appearance for its kind and
 *  side (a soldier's own variant), x-rayed in `xrayOf`'s colour for its unit,
 *  and one per fading corpse, lying, sunk by its fade and never x-rayed.
 *  Writes into `out` (reusing its records, so a frame allocates nothing once
 *  warm) and returns it. */
export function poseFrameInstances(
  out: ModelInstance[],
  frame: PoseFrame,
  resolve: ResolveAppearance,
  xrayOf: XrayOf = NO_XRAY,
): ModelInstance[] {
  let n = 0;
  const record = (): ModelInstance => {
    let m = out[n];
    if (!m) {
      m = { appearance: "", x: 0, y: 0, z: 0, yaw: 0, pose: REST_POSE, xray: null };
      out[n] = m;
    }
    n++;
    return m;
  };
  for (const s of frame.soldiers) {
    const resolved = resolve(s.kind, s.side, s.soldier, s.slot, s.operatorMount, s.activeMount);
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
    m.xray = xrayOf(s.side, s.unit);
  }
  for (const v of frame.vehicles) {
    const resolved = resolve(v.kind, v.side, v.unit, 0);
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
    m.xray = xrayOf(v.side, v.unit);
  }
  // Corpses the cap has pushed out, sinking: posed per frame until gone.
  for (const f of frame.fading) {
    const c = f.corpse;
    const resolved = resolve(c.kind, c.side, c.soldier, c.slot, c.operatorMount ?? null, null);
    if (!resolved) continue;
    const m = record();
    m.appearance = resolved.appearance;
    m.tint = resolved.tint;
    m.x = c.position[0];
    m.y = c.position[1];
    m.z = c.position[2] - f.sink;
    m.yaw = c.yaw;
    m.pose = LYING_POSE;
    m.xray = null;
  }
  out.length = n;
  return out;
}

/** The static corpses of a pose frame (call when `corpsesVersion` changes). */
export function corpseInstances(frame: PoseFrame, resolve: ResolveAppearance): CorpseInstance[] {
  const out: CorpseInstance[] = [];
  for (const c of frame.corpses) {
    const resolved = resolve(c.kind, c.side, c.soldier, c.slot, c.operatorMount ?? null, null);
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

const NO_XRAY: XrayOf = () => null;
const REST_POSE: SkinnedModelPose = { kind: "skinned", clip: "", phase: 0, blend: null };
const LYING_POSE: CorpseModelPose = { kind: "corpse" };
