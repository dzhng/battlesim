// The pose kernel: skinned palettes on the GPU. One invocation poses one
// soldier: for each joint (parents before children) it samples the clip's
// two neighbouring frames, slerps between them, optionally crossfades a
// second clip, composes the joint's world transform and writes
// `world · inverseBind` into the shared palette the model vertex stage reads.
//
// Rewritten from reading ~/dev/game battle-renderer/src/world/poseKernel.ts,
// posePalette.ts and renderer-core/src/posePaletteWgsl.ts:
// the same per-instance hierarchy walk and shortest-arc slerp,
// over our dense clip table (`clipTable.ts`) instead of the source's
// snapshot banks and upper-body layer.

/** Words per instance control record: four vec4u. */
export const CONTROL_WORDS = 16;

/**
 * One instance's control record.
 * - `a`: clip A's first float / 4 (vec4 index), frame 0, frame 1, weight bits
 * - `b`: the same for clip B (the crossfade), or copies of A
 * - `c`: blend weight bits, joint count, joint table base, palette base
 */
export function writeControl(
  words: Uint32Array,
  floats: Float32Array,
  at: number,
  a: { offset: number; f0: number; f1: number; w: number },
  b: { offset: number; f0: number; f1: number; w: number },
  blend: number,
  joints: number,
  jointBase: number,
  paletteBase: number,
) {
  const o = at * CONTROL_WORDS;
  words[o] = a.offset / 4;
  words[o + 1] = a.f0;
  words[o + 2] = a.f1;
  floats[o + 3] = a.w;
  words[o + 4] = b.offset / 4;
  words[o + 5] = b.f0;
  words[o + 6] = b.f1;
  floats[o + 7] = b.w;
  floats[o + 8] = blend;
  words[o + 9] = joints;
  words[o + 10] = jointBase;
  words[o + 11] = paletteBase;
}

/** The kernel's WGSL for bodies of at most `maxJoints` joints. */
export function poseKernelWgsl(maxJoints: number): string {
  return /* wgsl */ `
struct Control { a: vec4u, b: vec4u, c: vec4u, pad: vec4u };
struct Local { q: vec4f, t: vec3f, s: f32 };

@group(0) @binding(0) var<storage, read> samples: array<vec4f>;
@group(0) @binding(1) var<storage, read> parents: array<i32>;
@group(0) @binding(2) var<storage, read> inverseBinds: array<mat4x4f>;
@group(0) @binding(3) var<storage, read> controls: array<Control>;
@group(0) @binding(4) var<storage, read_write> palette: array<mat4x4f>;
@group(0) @binding(5) var<uniform> dispatch: vec4u;

// Shortest-arc slerp (the source's paletteSlerp, with WGSL's sin).
fn slerpQ(a: vec4f, inputB: vec4f, weight: f32) -> vec4f {
  if (weight <= 0.0) { return a; }
  if (weight >= 1.0) { return inputB; }
  var b = inputB;
  var cosine = dot(a, b);
  if (cosine < 0.0) { b = -b; cosine = -cosine; }
  var result: vec4f;
  if (cosine > 0.9995) {
    result = a * (1.0 - weight) + b * weight;
  } else {
    let theta = atan2(sqrt(max(0.0, 1.0 - cosine * cosine)), cosine);
    result = a * sin((1.0 - weight) * theta) + b * sin(weight * theta);
  }
  let magnitude = length(result);
  return result / select(1.0, magnitude, magnitude > 0.0);
}

fn readLocal(vec4Index: u32) -> Local {
  let q = samples[vec4Index];
  let ts = samples[vec4Index + 1u];
  return Local(q, ts.xyz, ts.w);
}

fn mixLocal(a: Local, b: Local, weight: f32) -> Local {
  return Local(slerpQ(a.q, b.q, weight), mix(a.t, b.t, weight), mix(a.s, b.s, weight));
}

fn sampleClip(clip: vec4u, joint: u32, joints: u32) -> Local {
  let a = readLocal(clip.x + (clip.y * joints + joint) * 2u);
  let b = readLocal(clip.x + (clip.z * joints + joint) * 2u);
  return mixLocal(a, b, bitcast<f32>(clip.w));
}

fn trs(pose: Local) -> mat4x4f {
  let q = pose.q;
  let x2 = q.x + q.x; let y2 = q.y + q.y; let z2 = q.z + q.z;
  let xx = q.x * x2; let xy = q.x * y2; let xz = q.x * z2;
  let yy = q.y * y2; let yz = q.y * z2; let zz = q.z * z2;
  let wx = q.w * x2; let wy = q.w * y2; let wz = q.w * z2;
  return mat4x4f(
    vec4f(vec3f(1.0 - (yy + zz), xy + wz, xz - wy) * pose.s, 0.0),
    vec4f(vec3f(xy - wz, 1.0 - (xx + zz), yz + wx) * pose.s, 0.0),
    vec4f(vec3f(xz + wy, yz - wx, 1.0 - (xx + yy)) * pose.s, 0.0),
    vec4f(pose.t, 1.0)
  );
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let instance = id.x;
  if (instance >= dispatch.x) { return; }
  let control = controls[instance];
  let blend = bitcast<f32>(control.c.x);
  let joints = min(control.c.y, ${maxJoints}u);
  let jointBase = control.c.z;
  let paletteBase = control.c.w;
  var world: array<mat4x4f, ${maxJoints}>;
  for (var joint = 0u; joint < joints; joint++) {
    var local = sampleClip(control.a, joint, joints);
    if (blend > 0.0) {
      local = mixLocal(local, sampleClip(control.b, joint, joints), blend);
    }
    var m = trs(local);
    let parent = parents[jointBase + joint];
    if (parent >= 0) { m = world[u32(parent)] * m; }
    world[joint] = m;
    palette[paletteBase + joint] = m * inverseBinds[jointBase + joint];
  }
}
`;
}
