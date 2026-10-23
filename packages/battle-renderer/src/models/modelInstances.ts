// What the models layer draws besides the presentation frame: one record per
// static model (a prop, a wreck, a building's part) or lab model, naming an
// installed appearance, where it stands and how it is posed. The battle's
// units, corpses and ghosts come posed from the presentation crate
// (`crates/presentation`) as frame batches; a lab or a bake poses its own
// body here with `posedPalette`, from the same bundles.

import { mat4, type Mat4 } from "math";
import type { Bundle, SkeletonClips } from "@packages/scene-assets/src/schema";
import {
  REST_ARTICULATION,
  articulate,
  articulationRig,
  restLocals,
  trackScroll,
  type Articulation,
} from "@packages/scene-assets/src/articulation";
import { poseWorlds, worldTransforms } from "@packages/scene-assets/src/pose";
import { isRgba, type Rgba } from "../mesh";
import { INSTANCE_WORDS, JOINT_WORDS } from "../records.gen";

/** A body or vehicle posed by its joint palette: one `Joint` (3×4 rows,
 *  `records.gen.ts`) per joint or node. */
export interface PosedModelPose {
  kind: "posed";
  palette: Float32Array;
  /** Left and right track scrolls, in links. */
  scroll?: readonly [number, number];
}

export interface StaticModelPose {
  kind: "static";
  state: string;
  /** A rigid motion of the state in its own frame, before its scale and
   *  yaw; none: as authored. A moving piece is never drawn as a card. */
  motion?: Mat4;
  /** A surface movers stand on (its body stops no mover: a bridge deck,
   *  rubble): it takes the ground paint, as the ground does (`view::model_fog`). */
  ground?: boolean;
}

/** A skinned appearance lying dead at one of its bundle's `corpse_poses`
 *  (`lies`, its index): a static mesh. */
export interface CorpseModelPose {
  kind: "corpse";
  lies: number;
}

export type ModelPose = PosedModelPose | StaticModelPose | CorpseModelPose;

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
  /** The side's tint on tint-masked surfaces; none keeps the authored colours. */
  tint?: readonly [number, number, number];
  /** Mesh tier, 0 finest. Omitted, the frame picks it by projected size and
   *  draws a far model as an impostor. */
  tier?: number;
  /** The colour (rgba) its hidden parts are drawn in through whatever world
   *  stands in front of it; none: not x-rayed. */
  xray?: readonly [number, number, number, number] | null;
  /** Placement preview colour and opacity. Draws the named mesh translucent,
   *  reading world depth, with no depth writes, shadow, impostor or x-ray. */
  ghost?: Rgba;
}

/** `presentation.overlay.xray`: the colours hidden parts are drawn in
 *  (`ModelInstance.xray`), the observing side's units in `own`, the
 *  selection in `selected`, an ally's units in `ally`. */
export interface XrayStyle {
  own: Rgba;
  selected: Rgba;
  ally: Rgba;
}

/** `presentation.overlay.xray`, checked: each rgba, none invisible. */
export function validateXray(x: XrayStyle): XrayStyle {
  const seen = (c: unknown) => isRgba(c) && c[3] > 0;
  if (!seen(x.own) || !seen(x.selected) || !seen(x.ally))
    throw new Error(
      "presentation.overlay.xray: own, selected and ally, rgba in [0, 1] with alpha > 0",
    );
  return x;
}

/** Column-major model matrices as palette joints. */
function joints(worlds: readonly ArrayLike<number>[]): Float32Array {
  const out = new Float32Array(worlds.length * JOINT_WORDS);
  worlds.forEach((m, j) => {
    for (let r = 0; r < 3; r++)
      for (let c = 0; c < 4; c++) out[j * JOINT_WORDS + r * 4 + c] = m[c * 4 + r];
  });
  return out;
}

/** A lab's or a bake's pose of `bundle`: a body in a clip at a phase, or a
 *  vehicle articulated as given (at rest without). The battle's poses are
 *  the presentation crate's. */
export function posedPalette(
  bundle: Exclude<Bundle, SkeletonClips>,
  skeleton: SkeletonClips | null,
  pose: { clip: string; phase: number } | Articulation | null = null,
): ModelPose {
  if (bundle.kind === "skinned") {
    const clip = pose && "clip" in pose ? pose : bundle.far_pose;
    const worlds = poseWorlds(bundle, skeleton, clip);
    return {
      kind: "posed",
      palette: joints(
        worlds.map((w, j) => mat4.multiply(mat4.create(), w, bundle.joints[j].inverse_bind)),
      ),
    };
  }
  if (bundle.kind === "articulated") {
    const input = pose && !("clip" in pose) ? pose : REST_ARTICULATION;
    const rig = articulationRig(bundle.nodes);
    const locals = articulate(restLocals(bundle.nodes), bundle.nodes, rig, input);
    const scroll = trackScroll(rig, input);
    return {
      kind: "posed",
      palette: joints(
        worldTransforms(
          bundle.nodes.map((n) => n.parent),
          locals,
        ),
      ),
      scroll: [scroll.left, scroll.right],
    };
  }
  return { kind: "static", state: bundle.states[0].name };
}

/** The authored resting model: shared by placement previews and far-model bakes. */
export function restingModelPose(
  bundle: Exclude<Bundle, SkeletonClips>,
  skeleton: SkeletonClips | null,
): ModelPose {
  return posedPalette(bundle, skeleton);
}

/** A column-major model matrix into palette joint `at`, as its 3×4 rows. */
export function writeJoint(into: Float32Array, at: number, m: ArrayLike<number>) {
  const o = at * JOINT_WORDS;
  for (let r = 0; r < 3; r++) for (let c = 0; c < 4; c++) into[o + r * 4 + c] = m[c * 4 + r];
}
const NO_XRAY = [0, 0, 0, 0] as const;
/** An x-ray colour's rgb as one integer under 2^24, which f32 holds exactly
 *  (the vertex stage unpacks it). */
export function packXray(c: readonly number[]): number {
  const byte = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 255);
  return byte(c[0]) * 65536 + byte(c[1]) * 256 + byte(c[2]);
}
/** A model drawn without a side keeps its authored colours. */
const NO_TINT = [1, 1, 1] as const;
const UNIT_SCALE = [1, 1, 1] as const;
export function writeRecord(
  into: Float32Array,
  at: number,
  inst: ModelInstance,
  base: number,
  scrollL: number,
  scrollR: number,
  layer: number,
) {
  const r = at * INSTANCE_WORDS;
  into[r] = inst.x;
  into[r + 1] = inst.y;
  into[r + 2] = inst.z;
  into[r + 3] = inst.yaw;
  into[r + 4] = base;
  into[r + 5] = scrollL - Math.floor(scrollL);
  into[r + 6] = scrollR - Math.floor(scrollR);
  const presentation = inst.ghost ?? inst.xray ?? NO_XRAY;
  into[r + 7] = packXray(presentation);
  into.set(inst.tint ?? NO_TINT, r + 8);
  into[r + 11] = layer;
  into.set(inst.scale ?? UNIT_SCALE, r + 12);
  into[r + 15] = presentation[3];
}
