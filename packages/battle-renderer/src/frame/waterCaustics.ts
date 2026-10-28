// The light the water's waves focus on its bed (caustics, RedGPU
// `lakeFragment.wgsl:435-501`): the baked normal map (`waterNormals.ts`) at
// two scales, each bending the other's coordinates, bright where the two
// agree. The waves that make them are the surface's own, so they drift with
// its primary and detail layers. Only where the water refracts
// (`WaterDetail`): Low draws none.
import { tgpu, d, std } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import { environmentSun } from "../world/environment";
import { terrainLayout } from "./terrainLayout";
import { WATER_NORMALS_FINEST } from "./waterNormals";
import { waterCarried } from "./waterSurface";

/** The normal map's slope at tile position `uv`. */
const slopeAt = tgpu.fn(
  [d.vec2f],
  d.vec2f,
)((uv) => {
  "use gpu";
  return std.textureSampleLevel(terrainLayout.$.waterNormals, terrainLayout.$.waterTiles, uv, 0).xy;
});

/** How bright a crest is where two waves' heights lie `apart`: whole
 *  where they meet, gone `width` apart. A crest narrower than half its
 *  pixel widens to it and dims as it does, so it keeps its light without
 *  crawling. */
const crest = tgpu.fn(
  [d.f32, d.f32],
  d.f32,
)((apart, width) => {
  "use gpu";
  const blur = 0.5 * std.fwidth(apart);
  const c = (1 - std.smoothstep(0, width + blur, std.abs(apart))) * (width / (width + blur));
  return c * c;
});

/** How bright the crests are where two slopes agree, along one diagonal
 *  each way, sharpened by `sharpness`. */
const crests = tgpu.fn(
  [d.vec2f, d.vec2f, d.f32],
  d.f32,
)((a, b, sharpness) => {
  "use gpu";
  const one = crest((a.x + a.y - b.x + b.y) * sharpness, 0.85);
  const two = crest((a.x - a.y - b.x - b.y) * sharpness, 0.85);
  return (one + two) * 0.9;
});

/** The caustics' light on the bed under the water's surface point `world`,
 *  `far` metres straight down the view (unrefracted, so the bed it rebuilds
 *  is smooth from pixel to pixel), in look `look`, as a share of the bed's
 *  own light. The bed point is the view's, and the waves that light it lie where
 *  the sun's ray came down through the surface, its slant shortened by the
 *  water's bend (about three quarters). They fade with the depth the light
 *  crosses (`caustics.z` a metre), vanish in the first centimetres, keep off
 *  walls too steep to hold them, and fade where a pixel (`footprint`
 *  metres) spans a quarter to a half of the finer layer's finest wave. */
export const waterCaustics = tgpu.fn(
  [d.vec3f, d.f32, d.f32, d.u32],
  d.f32,
)((world, far, footprint, look) => {
  "use gpu";
  if (terrainLayout.$.params.view.z === 1) {
    return 0;
  }
  const w = terrainLayout.$.params.waterLooks[look];
  const c = w.caustics;
  const view = std.normalize(std.sub(typegpuCameraLayout.$.cam.eye, world));
  const bed = std.sub(world, std.mul(view, far));
  const depth = std.max(world.z - bed.z, 0);
  const up = std.abs(std.normalize(std.cross(std.dpdx(bed), std.dpdy(bed))).z);
  const level = std.smoothstep(0.3, 0.7, up);
  const sun = environmentSun();
  const through = std.add(bed.xy, std.mul(sun.xy, (0.75 * depth) / std.max(sun.z, 0.3)));
  const carried = waterCarried();
  const uv1 = std.mul(std.sub(through, std.mul(carried, w.drifts.x)), c.x);
  const uv2 = std.mul(std.sub(through, std.mul(carried, w.drifts.z)), c.x * 1.8);
  const s1 = slopeAt(std.add(uv1, std.mul(slopeAt(uv2), 0.18)));
  const s2 = slopeAt(std.add(uv2, std.mul(slopeAt(uv1), 0.18)));
  const fade = std.exp(-depth * c.z) * std.smoothstep(0.01, 0.08, depth);
  const held = 1 - std.smoothstep(0.25, 0.5, footprint * c.x * 1.8 * WATER_NORMALS_FINEST);
  const high = std.clamp(sun.z * 1.5, 0.35, 1);
  return crests(s1, s2, c.w) * c.y * fade * level * held * high;
});
