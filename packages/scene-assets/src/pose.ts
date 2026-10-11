// CPU posing of baked bundles: clip sampling, joint and node world transforms,
// and linear-blend skinning. Validation measures fit in a pose with these, and
// the bake derives animated and posed bounds from them.

import { quat, vec3, type Mat4, type Quat, type Vec3 } from "math";
import { mul, trsMatrix, type Trs } from "./trs.ts";
import {
  SWEEP_PAD,
  articulate,
  articulationRig,
  restLocals,
  sweepArticulations,
} from "./articulation.ts";
import {
  CHANNEL_ABSENT,
  CHANNEL_ANIMATED,
  type ArticulatedNode,
  type Bounds,
  type Bundle,
  type Clip,
  type Joint,
  type MeshData,
  type PoseRef,
  type SkeletonClips,
  type SkinnedBundle,
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

/**
 * The finest tier's bounds in the pose an impostor shows from afar: a body's
 * `far_pose`, a vehicle at rest, a static appearance's first state.
 */
export function farPoseBounds(
  bundle: Exclude<Bundle, SkeletonClips>,
  skeleton: SkeletonClips | null,
): Bounds {
  if (bundle.kind === "static") return bundle.states[0].bounds;
  if (bundle.kind === "articulated")
    return positionsBounds(articulatedPositions(bundle.nodes, articulatedWorlds(bundle.nodes), 0));
  return positionsBounds(
    skinPositions(bundle.tiers[0], bundle.joints, poseWorlds(bundle, skeleton, bundle.far_pose)),
  );
}

/** A body's joint worlds in `pose` (its bind where the clip is missing). */
export function poseWorlds(
  bundle: Pick<SkinnedBundle, "joints">,
  skeleton: SkeletonClips | null,
  pose: PoseRef,
): Mat4[] {
  const clip = skeleton?.clips.find((c) => c.name === pose.clip);
  const locals =
    skeleton && clip
      ? sampleClip(skeleton, clip, bundle.joints, pose.phase)
      : bundle.joints.map((j) => j.bind);
  return worldTransforms(
    bundle.joints.map((j) => j.parent),
    locals,
  );
}

/**
 * `mesh` skinned once under joint `worlds` into a static mesh: positions,
 * normals and tangents posed, joints and weights dropped. A corpse is its
 * body posed at `corpse_pose` this way, then drawn like any static mesh.
 */
export function posedMesh(mesh: MeshData, joints: Joint[], worlds: Mat4[]): MeshData {
  const skinning = worlds.map((w, j) => mul(w, joints[j].inverse_bind));
  const count = mesh.positions.length / 3;
  const positions = new Float32Array(mesh.positions.length);
  const normals = new Int16Array(mesh.normals.length);
  const tangents = mesh.tangents ? new Int16Array(mesh.tangents.length) : undefined;
  const p = vec3.create();
  const n = vec3.create();
  const t = vec3.create();
  const q = vec3.create();
  const sumP = vec3.create();
  const sumN = vec3.create();
  const sumT = vec3.create();
  // Rotation only: the joints carry no shear, so the upper 3×3 turns directions.
  const turn = (m: Mat4, a: Vec3) =>
    vec3.set(
      q,
      m[0] * a[0] + m[4] * a[1] + m[8] * a[2],
      m[1] * a[0] + m[5] * a[1] + m[9] * a[2],
      m[2] * a[0] + m[6] * a[1] + m[10] * a[2],
    );
  const unit = (out: Vec3, from: Int16Array, v: number) =>
    vec3.set(out, from[v * 4] / 32767, from[v * 4 + 1] / 32767, from[v * 4 + 2] / 32767);
  for (let v = 0; v < count; v++) {
    vec3.fromBuffer(p, mesh.positions, v * 3);
    unit(n, mesh.normals, v);
    if (tangents) unit(t, mesh.tangents!, v);
    vec3.zero(sumP);
    vec3.zero(sumN);
    vec3.zero(sumT);
    for (let k = 0; k < 4; k++) {
      const weight = mesh.weights![v * 4 + k] / 65535;
      if (!weight) continue;
      const m = skinning[mesh.joints![v * 4 + k]];
      vec3.scaleAndAdd(sumP, sumP, vec3.transformMat4(q, p, m), weight);
      vec3.scaleAndAdd(sumN, sumN, turn(m, n), weight);
      if (tangents) vec3.scaleAndAdd(sumT, sumT, turn(m, t), weight);
    }
    vec3.toBuffer(positions, sumP, v * 3);
    vec3.normalize(sumN, sumN);
    for (let c = 0; c < 3; c++) normals[v * 4 + c] = Math.round(sumN[c] * 32767);
    if (tangents && vec3.length(sumT) > 1e-8) {
      vec3.normalize(sumT, sumT);
      for (let c = 0; c < 3; c++) tangents[v * 4 + c] = Math.round(sumT[c] * 32767);
      tangents[v * 4 + 3] = mesh.tangents![v * 4 + 3];
    }
  }
  return {
    positions,
    normals,
    ...(tangents ? { tangents } : {}),
    uvs: mesh.uvs,
    colors: mesh.colors,
    indices: mesh.indices,
    draws: mesh.draws,
  };
}

/** Each node's box over every tier, in the node's own space; null for an empty node. */
export function nodeBoxes(nodes: readonly ArticulatedNode[]): (Bounds | null)[] {
  return nodes.map((node) => {
    let box: Bounds | null = null;
    for (const mesh of node.tiers)
      if (mesh.positions.length) box = positionsBounds(mesh.positions, box ?? undefined);
    return box;
  });
}

/**
 * Culling bounds of an articulated bundle over every pose its rig can reach
 * (`sweepArticulations`, at the rig's own pitch limits): each node's box corners placed by the posed
 * node worlds, padded horizontally for the turns between sampled bearings.
 * Conservative by construction — a box's corners bound anything inside it —
 * and cheap, since only eight points per node move.
 */
export function posedBounds(nodes: readonly ArticulatedNode[]): Bounds {
  const rig = articulationRig(nodes);
  const boxes = nodeBoxes(nodes);
  // A wheel rolls about its axle (local +Y): its box becomes the disc's square.
  for (const wheel of rig.wheels) {
    const box = boxes[wheel.node];
    if (!box) continue;
    for (const c of [0, 2]) {
      box.min[c] = Math.min(box.min[c], -wheel.radius);
      box.max[c] = Math.max(box.max[c], wheel.radius);
    }
  }
  // A rotor turns about its mast (local +Z): its box becomes its disc's square.
  for (const rotor of rig.rotors) {
    const box = boxes[rotor.node];
    if (!box) continue;
    for (const c of [0, 1]) {
      box.min[c] = Math.min(box.min[c], -rotor.radius);
      box.max[c] = Math.max(box.max[c], rotor.radius);
    }
  }
  const locals = restLocals(nodes);
  const parents = nodes.map((n) => n.parent);
  const bounds: Bounds = {
    min: [Infinity, Infinity, Infinity],
    max: [-Infinity, -Infinity, -Infinity],
  };
  const corner = vec3.create();
  let reach = 0;
  for (const input of sweepArticulations(rig.pitch)) {
    const worlds = worldTransforms(parents, articulate(locals, nodes, rig, input));
    boxes.forEach((box, i) => {
      if (!box) return;
      for (let k = 0; k < 8; k++) {
        vec3.set(
          corner,
          k & 1 ? box.max[0] : box.min[0],
          k & 2 ? box.max[1] : box.min[1],
          k & 4 ? box.max[2] : box.min[2],
        );
        vec3.transformMat4(corner, corner, worlds[i]);
        vec3.min(bounds.min, bounds.min, corner);
        vec3.max(bounds.max, bounds.max, corner);
        reach = Math.max(reach, Math.hypot(corner[0], corner[1]));
      }
    });
  }
  const pad = rig.turret >= 0 || rig.hmg >= 0 ? reach * SWEEP_PAD : 0;
  for (const c of [0, 1]) {
    bounds.min[c] -= pad;
    bounds.max[c] += pad;
  }
  return bounds;
}
