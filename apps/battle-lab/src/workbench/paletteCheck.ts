// The GPU pose path checked against the CPU one: the pose kernel's palette
// for a posed body must equal scene-assets' `sampleClip` → `worldTransforms`
// → `kickWorlds` → `· inverse_bind`, and a vehicle's node palette must equal
// `articulate`'s node worlds under its whole-model motion. The workbench and
// its scene run this on the live frame.

import { mat4 } from "math";
import type { BattleFrame } from "@packages/battle-renderer/src/scene";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import { articulate, articulationRig, restLocals } from "@packages/scene-assets/src/articulation";
import {
  kickWorlds,
  kickJoint,
  sampleClip,
  worldTransforms,
} from "@packages/scene-assets/src/pose";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";

/** Largest absolute difference between the drawn palette of `instance` (the
 *  frame's only model) and the CPU's. */
export async function paletteError(
  frame: BattleFrame,
  installed: InstalledAppearances,
  instance: ModelInstance,
): Promise<number> {
  const bundle = installed.appearances.get(instance.appearance)?.bundle;
  if (!bundle || bundle.kind === "static") return 0;
  const { palette, bases } = await frame.readPalette();
  const base = bases[0];
  if (base === undefined) throw new Error(`${instance.appearance} is not drawn`);
  let expected: Float32Array[] = [];
  const pose = instance.pose;
  if (bundle.kind === "skinned" && pose.kind === "skinned") {
    const skeleton = installed.skeletons.get(bundle.skeleton)!;
    const clip = skeleton.clips.find((c) => c.name === pose.clip) ?? skeleton.clips[0];
    const phase = pose.phase;
    // The kernel wraps a looping clip's phase as `clipFrames` does.
    const parents = bundle.joints.map((j) => j.parent);
    const worlds = worldTransforms(
      parents,
      sampleClip(skeleton, clip, bundle.joints, clip.loop ? phase - Math.floor(phase) : phase),
    );
    if (pose.kick) kickWorlds(worlds, parents, kickJoint(bundle), pose.kick);
    expected = worlds.map((w, j) =>
      Float32Array.from(mat4.multiply(mat4.create(), w, bundle.joints[j].inverse_bind)),
    );
  } else if (bundle.kind === "articulated" && pose.kind === "articulated") {
    const nodes = bundle.nodes;
    const motion = pose.motion;
    expected = worldTransforms(
      nodes.map((n) => n.parent),
      articulate(restLocals(nodes), nodes, articulationRig(nodes), pose.articulation),
    ).map((w) => Float32Array.from(motion ? mat4.multiply(mat4.create(), motion, w) : w));
  }
  let worst = 0;
  expected.forEach((m, j) => {
    for (let c = 0; c < 16; c++)
      worst = Math.max(worst, Math.abs(palette[(base + j) * 16 + c] - m[c]));
  });
  return worst;
}
