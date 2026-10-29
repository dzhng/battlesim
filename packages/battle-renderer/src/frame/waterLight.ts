// How the water's surface takes light, through the one light owner
// (`environment.surfaceLight`, the shared `standardPbr`): no second lighting
// model (RedGPU `lakeFragment.wgsl:503-609`). The sky and the sun reflect in
// three lobes of rising roughness, sharp glints over a broader sheen, and
// what comes up from under the surface loses what the surface reflects.
import { tgpu, d, std } from "typegpu";
import type { EnvironmentFrame } from "./environmentFrame";
import { terrainLayout } from "./terrainLayout";

/** Water reflects 2% of the light that meets it head on. */
const WATER_F0 = 0.02;
/** The shared light's dielectric reflects 4% head on; on a black base, half
 *  metal reflects half that, in the sun's lobe and the sky's alike, and
 *  carries no diffuse light. */
const WATER_METAL = 1 - WATER_F0 / 0.04;

/** The water's light functions, lit by `environment`. */
export function createWaterLight(environment: EnvironmentFrame) {
  /** One lobe of the reflection, of roughness `roughness`. */
  const lobe = tgpu.fn(
    [d.f32, d.vec3f, d.vec3f, d.f32, d.vec3f],
    d.vec3f,
  )((roughness, n, world, sun, eye) => {
    "use gpu";
    return environment.surfaceLight(
      d.vec3f(0),
      d.vec3f(0),
      roughness,
      0,
      WATER_METAL,
      1,
      n,
      world,
      sun,
      eye,
    );
  });
  return {
    /** The sky's and the sun's reflection off the surface of look `look` at
     *  `world`, of normal `n`, `sun` its sun's visibility: each lobe of its
     *  row (`roughness`) at its share (`lobes`). */
    waterReflection: tgpu.fn(
      [d.vec3f, d.vec3f, d.f32, d.vec3f, d.u32],
      d.vec3f,
    )((n, world, sun, eye, look) => {
      "use gpu";
      const w = terrainLayout.$.params.waterLooks[look];
      const sharp = std.mul(lobe(w.roughness.x, n, world, sun, eye), w.lobes.x);
      const sheen = std.mul(lobe(w.roughness.y, n, world, sun, eye), w.lobes.y);
      const broad = std.mul(lobe(w.roughness.z, n, world, sun, eye), w.lobes.z);
      return std.add(std.add(sharp, sheen), broad);
    }),
    /** The share of the light from under the surface, of normal `n`, that
     *  crosses it toward the eye along `view` (Schlick's Fresnel): nearly
     *  all of it seen from above, little at a grazing angle, where the
     *  surface turns to a mirror. */
    waterTransmission: tgpu.fn(
      [d.vec3f, d.vec3f],
      d.f32,
    )((n, view) => {
      "use gpu";
      const edgeOn = 1 - std.clamp(std.dot(n, view), 0, 1);
      return 1 - (WATER_F0 + (1 - WATER_F0) * std.pow(edgeOn, 5));
    }),
  };
}
