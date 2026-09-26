// CPU posing of baked bundles: clip sampling, joint and node world transforms,
// and linear-blend skinning. Validation measures fit in a pose with these, and
// the bake derives animated bounds from them.

import { quat, vec3, type Mat4, type Quat, type Vec3 } from "math";
import { mul, trsMatrix, type Trs } from "./trs.ts";
import {
  CHANNEL_ABSENT,
  CHANNEL_ANIMATED,
  type ArticulatedNode,
  type Bounds,
  type Clip,
  type Joint,
  type MeshData,
  type SkeletonClips,
} from "./schema.ts";

/** Local joint transforms of `clip` at `phase` (0..1), falling back to the body's bind. */
export function sampleClip(
  clips: SkeletonClips,
  clip: Clip,
  joints: Joint[],
  phase: number,
): Trs[] {
  const frame = Math.min(clip.frames - 1, Math.max(0, phase * clip.duration * clips.sample_hz));
  const lo = Math.floor(frame);
  const hi = Math.min(clip.frames - 1, lo + 1);
  const w = frame - lo;
  let r = 0;
  let t = 0;
  return joints.map((joint, j) => {
    const local: Trs = { t: [...joint.bind.t], r: [...joint.bind.r], s: [...joint.bind.s] };
    const rMode = clip.rotation_modes[j];
    if (rMode !== CHANNEL_ABSENT) {
      const at = (f: number): Quat => {
        const o = (r + (rMode === CHANNEL_ANIMATED ? f : 0)) * 4;
        return [0, 1, 2, 3].map((c) => clip.rotations[o + c] / 32767) as Quat;
      };
      local.r = quat.slerp(quat.create(), at(lo), at(hi), w);
      r += rMode === CHANNEL_ANIMATED ? clip.frames : 1;
    }
    const tMode = clip.translation_modes[j];
    if (tMode !== CHANNEL_ABSENT) {
      const at = (f: number) => (t + (tMode === CHANNEL_ANIMATED ? f : 0)) * 3;
      const a = at(lo);
      const b = at(hi);
      local.t = [0, 1, 2].map(
        (c) => clip.translations[a + c] + (clip.translations[b + c] - clip.translations[a + c]) * w,
      ) as Vec3;
      t += tMode === CHANNEL_ANIMATED ? clip.frames : 1;
    }
    return local;
  });
}

/** World transforms from locals; parents precede children. */
export function worldTransforms(parents: number[], locals: Trs[]): Mat4[] {
  const worlds: Mat4[] = [];
  parents.forEach((parent, i) => {
    const m = trsMatrix(locals[i]);
    worlds.push(parent < 0 ? m : mul(worlds[parent], m));
  });
  return worlds;
}

/** Linear-blend skinned positions of `mesh` under joint `worlds`. */
export function skinPositions(mesh: MeshData, joints: Joint[], worlds: Mat4[]): Float32Array {
  const skinning = worlds.map((w, j) => mul(w, joints[j].inverse_bind));
  const out = new Float32Array(mesh.positions.length);
  const p = vec3.create();
  const q = vec3.create();
  const sum = vec3.create();
  for (let v = 0; v < mesh.positions.length / 3; v++) {
    vec3.fromBuffer(p, mesh.positions, v * 3);
    vec3.zero(sum);
    for (let k = 0; k < 4; k++) {
      const weight = mesh.weights![v * 4 + k] / 65535;
      if (!weight) continue;
      vec3.scaleAndAdd(
        sum,
        sum,
        vec3.transformMat4(q, p, skinning[mesh.joints![v * 4 + k]]),
        weight,
      );
    }
    vec3.toBuffer(out, sum, v * 3);
  }
  return out;
}

export function positionsBounds(positions: Float32Array, into?: Bounds): Bounds {
  const b: Bounds = into ?? {
    min: [Infinity, Infinity, Infinity],
    max: [-Infinity, -Infinity, -Infinity],
  };
  for (let v = 0; v < positions.length; v += 3)
    for (let c = 0; c < 3; c++) {
      b.min[c] = Math.min(b.min[c], positions[v + c]);
      b.max[c] = Math.max(b.max[c], positions[v + c]);
    }
  return b;
}

/** Articulated node worlds at rest, with optional local overrides by node name. */
export function articulatedWorlds(
  nodes: ArticulatedNode[],
  overrides: Record<string, Trs> = {},
): Mat4[] {
  return worldTransforms(
    nodes.map((n) => n.parent),
    nodes.map((n) => overrides[n.name] ?? n.bind),
  );
}

/** Every vertex of one tier of an articulated bundle, placed by node `worlds`. */
export function articulatedPositions(
  nodes: ArticulatedNode[],
  worlds: Mat4[],
  tier: number,
  include?: (node: number) => boolean,
): Float32Array {
  const out: number[] = [];
  nodes.forEach((node, i) => {
    if (include && !include(i)) return;
    const mesh = node.tiers[tier];
    const p = vec3.create();
    for (let v = 0; v < mesh.positions.length; v += 3)
      out.push(...vec3.transformMat4(p, vec3.fromBuffer(p, mesh.positions, v), worlds[i]));
  });
  return Float32Array.from(out);
}
