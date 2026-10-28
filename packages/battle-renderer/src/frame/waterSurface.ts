// The water surface's look (presentation, not rules; the biome's `water`
// row): clear at its edge, where the bed shows through, taking on the
// channel's colour and opacity over the shallows; smooth enough that sky and
// sun reflect in it, broken by two octaves of ripples. The surface ends on the
// simulation's water edge, feathered over a pixel: the ground meets the
// surface there, so nothing hides it and nothing steps. Its numbers are
// `presentation.terrain.water`, which the world pass reads too.
import { tgpu, d, std } from "typegpu";
import game from "@fixtures/game.json";
import { groundCell, groundWater, terrainLayout, valueNoise } from "./terrainMaterial";
const tuning = game.presentation.terrain;

/** Ripple height at `xy`: two octaves of value noise, in about [0, 1.5]. */
const rippleHeight = tgpu.fn(
  [d.vec2f],
  d.f32,
)((xy) => {
  "use gpu";
  return (
    valueNoise(std.mul(xy, 1 / tuning.water.ripple.long_m)) +
    tuning.water.ripple.short_weight *
      valueNoise(std.add(std.mul(xy, 1 / tuning.water.ripple.short_m), d.vec2f(17.3, 5.1)))
  );
});

/** The water surface at `xy`: linear colour and opacity (clear at the shore,
 *  murky out in the channel, `groundWater` giving how far in it lies), none
 *  outside the water's edge. Streaks of light lie on it in lanes along the
 *  stream: a reflection shows the ripples only where the sun or the sky's
 *  bright edge happens to lie behind the water, and without the streaks a
 *  reach seen from above is a flat band. A pixel wider than a ripple takes
 *  their mean, so the water keeps its value as the camera pulls out.
 *  `footprint` is the metres one pixel spans. */
export const waterSurface = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec4f,
)((xy, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const water = groundWater(xy, groundCell(xy, footprint));
  const deep = 1 - std.exp(-std.max(water, 0) * params.waterLook.x);
  const shown =
    1 - std.smoothstep(tuning.water.ripple.fade_m[0], tuning.water.ripple.fade_m[1], footprint);
  const drift = tuning.water.lanes.drift * valueNoise(std.mul(xy, 1 / tuning.water.lanes.drift_m));
  const lane = valueNoise(d.vec2f(water / tuning.water.lanes.m + drift, 3.7));
  const streak = valueNoise(
    std.add(std.mul(xy, 1 / tuning.water.lanes.streak_m), d.vec2f(63.1, 29.3)),
  );
  const lit = std.smoothstep(
    tuning.water.lanes.lit[0],
    tuning.water.lanes.lit[1],
    tuning.water.lanes.weights[0] * lane + tuning.water.lanes.weights[1] * streak,
  );
  const colour = std.mul(
    std.mix(params.waterEdge.xyz, params.water.xyz, deep),
    1 + params.waterLook.y * std.mix(tuning.water.lanes.lit_mean, lit, shown),
  );
  const edge = std.smoothstep(-footprint * 0.5, footprint * 0.5, water);
  return d.vec4f(colour, std.mix(params.waterEdge.w, params.water.w, deep) * edge);
});

/** The water's normal at `xy`: small ripples, fading out where a pixel spans
 *  more than a ripple (`footprint` metres), so far water lies flat and calm. */
export const waterNormal = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec3f,
)((xy, footprint) => {
  "use gpu";
  const ex = d.vec2f(tuning.water.ripple.step_m, 0);
  const ey = d.vec2f(0, tuning.water.ripple.step_m);
  const dx = rippleHeight(std.add(xy, ex)) - rippleHeight(std.sub(xy, ex));
  const dy = rippleHeight(std.add(xy, ey)) - rippleHeight(std.sub(xy, ey));
  const k =
    (tuning.water.ripple.slope / (2 * tuning.water.ripple.step_m)) *
    (1 - std.smoothstep(tuning.water.ripple.fade_m[0], tuning.water.ripple.fade_m[1], footprint));
  return std.normalize(d.vec3f(-dx * k, -dy * k, 1));
});
