// Cast lights: the light combat throws on what is round it. A muzzle flash, a
// missile's motor, a tracer, a burst or a burning wreck is a short-lived point
// light, and every world material (ground, grass, scenery, models, water) and
// every lit smoke sprite takes the same lights through the environment's one
// `shade` (`world/environment.ts`), so a flash lights the ground, the grass,
// the house wall and the tank beside it alike.
//
// The effects (`crates/presentation/src/effects`) say which lights burn at
// the presentation clock, each with a published cause as every effect has,
// and choose the strongest the view sees when more burn than the world
// holds. This module owns the light model and the uniform the world reads
// (`CastLights`, at most `CAST_LIGHTS_MAX` of the frame's `CastLight` rows).
//
// The light model: irradiance `intensity · (1 − x²)² / (1 + CAST_FALLOFF·x²)`
// at x = d/r of its radius r, none past it (a bright heart and a long soft
// tail, as inverse square gives, windowed to zero at r so no pool has a
// rim), on the surface's albedo (partly greyed, held above a floor) as
// Lambert (over π, so an intensity reads against the sun's radiance), the
// facing wrapped by `wrap` (a pool on flat ground from a light a metre up
// still reaches its edge). No shadow: a light is short and near what it
// lights. Each light costs every lit fragment a loop step, so the uniform
// holds few, the strongest.
import { d } from "typegpu";
import { CAST_LIGHT_WORDS, CAST_LIGHTS_MAX, CastLight } from "../records.gen";

/** How fast a light falls off inside its radius (the model above): 0 is the
 *  plain window, which reads as a spotlight's disc with a shoulder. */
export const CAST_FALLOFF = 6;
/** How much of a surface's own hue a cast light ignores, 0..1: its albedo
 *  is pulled this far toward its own grey before the light multiplies it,
 *  so a warm pool on green grass reads warm, not lime. */
export const CAST_ALBEDO_GREY = 0.35;
/** The least albedo a cast light sees (linear): dark grass takes a pool
 *  near what the pale road beside it does, so no pool stops at a verge. */
export const CAST_ALBEDO_FLOOR = 0.1;
/** The frame's lights: count and wrap, then the lights. */
export const CastLights = d.struct({
  /** x: the lights in use; y: the facing wrap; zw unused. */
  header: d.vec4f,
  lights: d.arrayOf(CastLight, CAST_LIGHTS_MAX),
});
/** `CastLights`' bytes holding its first `count` lights: the 16 B header,
 *  then the lights. */
export const castLightsBytes = (count: number) => 16 + count * CAST_LIGHT_WORDS * 4;
/** `CastLights`' whole bytes. */
export const CAST_LIGHTS_BYTES = castLightsBytes(CAST_LIGHTS_MAX);
