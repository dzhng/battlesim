// The wave field on the water (water and wetlands slice 16): the field's
// slope where it covers the water (`Camera.waves`, `wave.rs`), laid over the
// surface's normal with RNM as RedGPU lays its ripples
// (`lakeFragment.wgsl:347-362`). It fades out at the field's edge and as the
// camera draws far back, and is off where the water keeps its depth colour
// alone (the Low tier's `WaterDetail`).
import { tgpu, d, std } from "typegpu";
import { WAVE } from "../records.gen";
import { typegpuCameraLayout } from "../world/camera";
import { fogLayout } from "./fogTerm";
import { terrainLayout } from "./terrainLayout";
import { blendRnm } from "./waterSurface";

/** The field's height at `uv`, filtered. */
const waveHeight = tgpu.fn(
  [d.vec2f],
  d.f32,
)((uv) => {
  "use gpu";
  return std.textureSampleLevel(fogLayout.$.waves, fogLayout.$.sceneTaps, uv, 0).x;
});

/** The water's normal `n` at `xy` with the wave field's ripples on it. */
export const waveNormal = tgpu.fn(
  [d.vec2f, d.vec3f],
  d.vec3f,
)((xy, n) => {
  "use gpu";
  const area = typegpuCameraLayout.$.cam.waves;
  const low = d.f32(terrainLayout.$.params.view.z === 1);
  const shown = area.w * (1 - low);
  const uv = std.div(std.sub(xy, area.xy), area.z);
  const inside = std.min(uv, std.sub(d.vec2f(1), uv));
  if (shown <= 0 || std.min(inside.x, inside.y) <= 0) {
    return d.vec3f(n);
  }
  const texel = 1 / WAVE.texels;
  const dx =
    waveHeight(std.add(uv, d.vec2f(texel, 0))) - waveHeight(std.sub(uv, d.vec2f(texel, 0)));
  const dy =
    waveHeight(std.add(uv, d.vec2f(0, texel))) - waveHeight(std.sub(uv, d.vec2f(0, texel)));
  const edge = std.smoothstep(0, 0.08, std.min(inside.x, inside.y));
  // RedGPU's slope: the height's step over two texels, times 16.
  const ripple = std.mul(d.vec2f(dx, dy), 16 * WAVE.normal * shown * edge);
  const flatten = std.saturate(std.length(ripple) * 1.8) * 0.72;
  const calmed = std.normalize(std.mix(n, d.vec3f(0, 0, 1), flatten));
  return blendRnm(calmed, std.normalize(d.vec3f(std.mul(ripple, -1), 1)));
});
