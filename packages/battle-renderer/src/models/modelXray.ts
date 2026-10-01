// Per drawn model, count covered and world-hidden fragments on the GPU.
// Grass and other units never enter the sampled depth. No readback or CPU
// visibility decision is needed; counts are reset and consumed in one frame.
// This is a surface-fragment fraction, not an exact silhouette-pixel ratio:
// overlapping triangles weight the heuristic, so tune it on actual posed models.
import { tgpu, d } from "typegpu";
import { modelVaryings } from "./modelLayer";

export const XRAY_DEPTH_BIAS = 1 << 16;

export function validateXrayFraction(fraction: number): number {
  if (!Number.isFinite(fraction) || fraction <= 0 || fraction > 1)
    throw new Error("presentation.overlay.xray.min_hidden_fragment_fraction must be in (0, 1]");
  return fraction;
}

const Count = d.struct({ total: d.atomic(d.u32), hidden: d.atomic(d.u32) });
export const xrayCountLayout = tgpu.bindGroupLayout({
  counts: {
    storage: (n: number) => d.arrayOf(Count, n),
    access: "mutable",
    visibility: ["fragment"],
  },
  depth: { texture: d.textureDepthMultisampled2d(), visibility: ["fragment"] },
});
export const xrayReadLayout = tgpu.bindGroupLayout({
  counts: {
    storage: (n: number) => d.arrayOf(d.vec2u, n),
    access: "readonly",
    visibility: ["fragment"],
  },
  minimum: { uniform: d.f32, visibility: ["fragment"] },
});

const countCoverage = tgpu
  .fn(
    [d.vec4f, d.u32, d.vec4f],
    d.vec4f,
  )(/* wgsl */ `(clip: vec4f, instance: u32, xray: vec4f) -> vec4f {
    if (xray.w <= 0.0) { discard; }
    atomicAdd(&xrayCountLayout.$.counts[instance].total, 1u);
    // Same float reverse-Z offset as the hardware depth bias in the draw.
    let biased = bitcast<f32>(bitcast<u32>(clip.z) + ${XRAY_DEPTH_BIAS}u);
    let world = textureLoad(xrayCountLayout.$.depth, vec2i(clip.xy), 0);
    if (biased < world) { atomicAdd(&xrayCountLayout.$.counts[instance].hidden, 1u); }
    return vec4f(0.0);
  }`)
  .$uses({ xrayCountLayout });

const drawCoverage = tgpu
  .fn(
    [d.u32, d.vec4f],
    d.vec4f,
  )(/* wgsl */ `(instance: u32, xray: vec4f) -> vec4f {
    let coverage = xrayReadLayout.$.counts[instance];
    if (xray.w <= 0.0 || f32(coverage.y) < f32(coverage.x) * xrayReadLayout.$.minimum) { discard; }
    return vec4f(xray.rgb * xray.a, xray.a);
  }`)
  .$uses({ xrayReadLayout });

export const modelXrayCountFragment = tgpu.fragmentFn({ in: modelVaryings, out: d.vec4f })((v) => {
  "use gpu";
  return countCoverage(v.clip, v.instance, v.xray);
});
export const modelXrayFragment = tgpu.fragmentFn({ in: modelVaryings, out: d.vec4f })((v) => {
  "use gpu";
  return drawCoverage(v.instance, v.xray);
});
