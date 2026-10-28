// How a standing tree moves in the one wind (`cameraWind`), on the camera's
// clock (RedGPU's `foliageInstanced.wgsl` trunk bend and leaf flutter). The
// trunk bends downwind, more the higher up a point stands (its height share
// cubed), swinging slowly about a lean, harder as a gust front rolls through
// (the grass's fronts); the leaves shiver about it, each its own way. Every
// pass that draws a standing tree calls this one function, so the depth
// prepass and the colour pass agree to the bit; a shadow caster passes no
// leaf, so its shadow bends with the trunk and does not shiver. Look only:
// the simulation's trees never move, and a felled tree does not sway.
import { tgpu, d } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import { cameraWind, typegpuCameraLayout } from "../world/camera";

/** The tuning (look only): how far the top bends as a share of the tree's
 *  height at full strength, the steady lean and the swing about it, the
 *  swing's rate (rad/s), the gust speed (m/s) that is full strength, and the
 *  leaves' shiver (m at the crown's top) and rate (rad/s). */
const SWAY = {
  bend: 0.1,
  lean: 0.25,
  swing: 0.75,
  rate: 1.6,
  fullGustMps: 6,
  shiverM: 0.15,
  shiverRate: 7,
};

/** The offset of a point of a standing tree: `foot` the tree's foot, `point`
 *  the point, `up` its height over the tree's (0 at the foot, 1 at the top),
 *  `height` the tree's height (m), `leaf` 1 where leaves shiver and 0 on
 *  bark or in a caster. */
export const sway = tgpu
  .fn(
    [d.vec3f, d.vec3f, d.f32, d.f32, d.f32],
    d.vec3f,
  )(/* wgsl */ `(foot: vec3f, point: vec3f, up: f32, height: f32, leaf: f32) -> vec3f {
  let wind = cameraWind();
  let t = typegpuCameraLayout.$.cam.time;
  let dir = wind.toward.xy;
  let strength = clamp(wind.gustMps / ${SWAY.fullGustMps}, 0.0, 1.0);
  let along = dot(foot.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(0.2, 1.0, front);
  let seed = pcgHash(bitcast<u32>(floor(foot.x * 4.0)) ^ (bitcast<u32>(floor(foot.y * 4.0)) * 2654435761u));
  let phase = f32(seed >> 8u) * (6.2831853 / 16777216.0);
  let swing = sin(t * ${SWAY.rate} + phase) + 0.25 * sin(1.5 * t * ${SWAY.rate} + 1.8 * phase);
  let h = clamp(up, 0.0, 1.0);
  let reach = h * h * h;
  let bend = height * ${SWAY.bend} * strength * (${SWAY.lean} * (1.0 + gust) + ${SWAY.swing} * swing) * reach;
  // Bent, the point keeps its distance from the foot: it drops a little.
  var offset = vec3f(dir * bend, -0.5 * bend * bend / max(height * h, 1.0));
  let q = point - foot;
  let shiver = vec3f(
    sin(t * ${SWAY.shiverRate} + q.x * 2.1 + q.y * 0.7 + phase),
    sin(t * ${SWAY.shiverRate} * 1.3 + q.y * 1.9 + q.z * 0.8 + phase),
    sin(t * ${SWAY.shiverRate} * 0.9 + q.z * 2.3 + q.x * 0.6 + phase),
  );
  offset += shiver * (${SWAY.shiverM} * strength * (0.5 + gust) * leaf * h);
  return offset;
}`)
  .$uses({ cameraWind, typegpuCameraLayout, pcgHash });
