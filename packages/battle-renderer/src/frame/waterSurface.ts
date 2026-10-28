// The water surface's look (presentation, not rules; a `WaterLook` row):
// clear at its edge, where the bed shows through, taking on the channel's
// colour and opacity over the shallows; smooth enough that sky and sun
// reflect in it, broken by two octaves of ripples. The surface ends on the
// simulation's water edge, feathered over a pixel: the ground meets the
// surface there, so nothing hides it and nothing steps. Its numbers are its
// row's, which the water's pass reads too.
import { tgpu, d, std } from "typegpu";
import { groundCell, groundWater, terrainLayout, valueNoise } from "./terrainMaterial";
import { WaterLook } from "./waterLook";

/** Ripple height at `xy` under look `w`: two octaves of value noise, in
 *  about [0, 1.5]. */
const rippleHeight = tgpu.fn(
  [d.vec2f, WaterLook],
  d.f32,
)((xy, w) => {
  "use gpu";
  return (
    valueNoise(std.mul(xy, w.ripple.x)) +
    w.ripple.z * valueNoise(std.add(std.mul(xy, w.ripple.y), d.vec2f(17.3, 5.1)))
  );
});

/** The water surface at `xy` in look `look` (a row of
 *  `TerrainParams.waterLooks`): linear colour and opacity (clear at the
 *  shore, murky out in the channel, `groundWater` giving how far in it
 *  lies), none outside the water's edge. Streaks of light lie on it in lanes
 *  along the stream: a reflection shows the ripples only where the sun or
 *  the sky's bright edge happens to lie behind the water, and without the
 *  streaks a reach seen from above is a flat band. A pixel wider than a
 *  ripple takes their mean, so the water keeps its value as the camera pulls
 *  out. `footprint` is the metres one pixel spans. */
export const waterSurface = tgpu.fn(
  [d.vec2f, d.f32, d.u32],
  d.vec4f,
)((xy, footprint, look) => {
  "use gpu";
  const w = terrainLayout.$.params.waterLooks[look];
  const water = groundWater(xy, groundCell(xy, footprint));
  const deep = 1 - std.exp(-std.max(water, 0) * w.surface.x);
  const shown = 1 - std.smoothstep(w.rippleSlope.y, w.rippleSlope.z, footprint);
  const drift = w.lanes.y * valueNoise(std.mul(xy, w.lanes.z));
  const lane = valueNoise(d.vec2f(water / w.lanes.x + drift, 3.7));
  const streak = valueNoise(std.add(std.mul(xy, w.lanes.w), d.vec2f(63.1, 29.3)));
  const lit = std.smoothstep(
    w.lanesLit.z,
    w.lanesLit.w,
    w.lanesLit.x * lane + w.lanesLit.y * streak,
  );
  const colour = std.mul(
    std.mix(w.edge.xyz, w.channel.xyz, deep),
    1 + w.surface.y * std.mix(w.lanesMean.x, lit, shown),
  );
  const edge = std.smoothstep(-footprint * 0.5, footprint * 0.5, water);
  return d.vec4f(colour, std.mix(w.edge.w, w.channel.w, deep) * edge);
});

/** The water's normal at `xy` in look `look`: small ripples, fading out
 *  where a pixel spans more than a ripple (`footprint` metres), so far
 *  water lies flat and calm. */
export const waterNormal = tgpu.fn(
  [d.vec2f, d.f32, d.u32],
  d.vec3f,
)((xy, footprint, look) => {
  "use gpu";
  const w = terrainLayout.$.params.waterLooks[look];
  const ex = d.vec2f(w.ripple.w, 0);
  const ey = d.vec2f(0, w.ripple.w);
  const dx = rippleHeight(std.add(xy, ex), w) - rippleHeight(std.sub(xy, ex), w);
  const dy = rippleHeight(std.add(xy, ey), w) - rippleHeight(std.sub(xy, ey), w);
  const k = w.rippleSlope.x * (1 - std.smoothstep(w.rippleSlope.y, w.rippleSlope.z, footprint));
  return std.normalize(d.vec3f(-dx * k, -dy * k, 1));
});
