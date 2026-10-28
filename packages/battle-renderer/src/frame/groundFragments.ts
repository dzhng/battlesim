// The ground's lit fragments: the terrain, FogTerm's ground, and the
// backdrop past the map's edge, both over the biome's ground material
// (`terrainMaterial.ts`). `worldPass.ts` builds their pipelines.
import { tgpu, d, std } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import type { EnvironmentFrame } from "./environmentFrame";
import { WORLD_VARYING } from "./geometry";
import {
  fogCoverage,
  fogIsGround,
  fogTerm,
  groundPaint,
  paintedAlbedo,
  paintedSeen,
  paintGlow,
} from "./fogTerm";
import {
  groundCell,
  groundClasses,
  groundClassView,
  groundColour,
  groundDapple,
  GroundPaved,
  groundPaved,
  groundRoadRelief,
  groundSite,
  groundBank,
  groundWater,
} from "./terrainMaterial";
import { groundScarsSeen, scarredNormal, scarredSurface, scarRegionContains } from "./groundScars";
import { WORLD_OUT } from "./targets";

/** Our vertex colours are sRGB display values; lighting wants linear albedo. */
export const srgbToLinear = tgpu.fn(
  [d.vec3f],
  d.vec3f,
)((c) => {
  "use gpu";
  return std.pow(std.max(c, d.vec3f(0)), d.vec3f(2.2));
});

/** A rough dielectric: the flat box world has no material maps yet. */
export const ROUGHNESS = 0.85;

/** The terrain's and the backdrop's fragment stages, lit by `environment`. */
export function createGroundFragments(environment: EnvironmentFrame) {
  const varyings = {
    clip: d.builtin.position,
    world: WORLD_VARYING,
    normal: d.vec3f,
    color: d.vec4f,
    highlight: d.f32,
  };
  /** The biome's ground, under the vertex tint (its alpha the tint's weight).
   *  `cell` is the point's `groundCell`, `paved` its `groundPaved`. */
  const groundAlbedo = tgpu.fn(
    [d.vec3f, d.vec4f, d.vec4u, GroundPaved],
    d.vec4f,
  )((world, tint, cell, paved) => {
    "use gpu";
    const footprint = std.length(std.fwidth(world.xy));
    const xy = world.xy;
    const site = groundSite(xy, cell, paved);
    const surface = groundColour(xy, footprint, site, paved, groundWater(xy, cell));
    const albedo = std.mix(surface.xyz, srgbToLinear(tint.xyz), tint.w);
    return d.vec4f(albedo, std.mix(surface.w, ROUGHNESS, tint.w));
  });
  /** What the class view's ground writes beside its bytes: seen ground,
   *  whether or not the frame has fog (the fog mask pass keeps only pixels
   *  that are wholly this). */
  const CLASS_GROUND = d.vec4f(0, 1, 1, 1);
  /** The terrain: FogTerm's ground. */
  const terrainFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    if (!scarRegionContains(v.world.xy)) std.discard();
    const eye = typegpuCameraLayout.$.cam.eye;
    const n = std.normalize(v.normal);
    const seen = fogTerm(v.world, n, v.clip.xy, fogIsGround());
    const footprint = std.length(std.fwidth(v.world.xy));
    // One lookup of the surface field serves the ground and its dapple.
    const cell = groundCell(v.world.xy, footprint);
    const paved = groundPaved(v.world.xy, cell);
    if (groundClassView()) {
      const xy = v.world.xy;
      const site = groundSite(xy, cell, paved);
      const classes = groundClasses(xy, footprint, site, groundWater(xy, cell));
      return { color: d.vec4f(classes, 1), fog: d.vec4f(CLASS_GROUND) };
    }
    const plain = groundAlbedo(v.world, v.color, cell, paved);
    // The side's learned scars, on the biome's ground (not under a tint).
    const biome = 1 - v.color.w;
    const scar = groundScarsSeen(v.world, eye, footprint);
    const surface = std.mix(plain, scarredSurface(plain, scar), biome);
    // A river's bank is lit as the cross-section it was cut to and its bed
    // flat, not as the grid's triangles (`groundBank`).
    const bank = groundBank(v.world.xy, cell, footprint);
    let ground = d.vec3f(n);
    if (bank.z > 0) {
      ground = std.normalize(std.mix(n, std.normalize(d.vec3f(-bank.x, -bank.y, 1)), bank.z));
    }
    // A road's ruts and its curb tilt it too: their sides catch the sun.
    const relief = groundRoadRelief(v.world.xy, footprint, paved);
    ground = std.normalize(std.sub(ground, std.mul(d.vec3f(relief, 0), ground.z * biome)));
    const shading = std.normalize(std.mix(ground, scarredNormal(ground, scar), biome));
    // The ground paint on it (its layer is painted), but not on the bed: the
    // water over it takes the paint (`waterPass.ts`), so a mark shows once.
    const water = groundWater(v.world.xy, cell);
    const dry = 1 - std.smoothstep(-footprint * 0.5, footprint * 0.5, water);
    const paint = std.mul(groundPaint(v.world), dry);
    // Sun flecks through the crowns lift the canopy's whole shadow.
    const flecks = groundDapple(v.world.xy, footprint, cell);
    const sun = std.max(environment.sampleSunShadow(v.world, n, v.clip.xy), flecks);
    const lit = environment.shade(
      paintedAlbedo(surface.xyz, paint),
      d.vec3f(0),
      surface.w,
      0,
      0,
      1,
      shading,
      v.world,
      sun,
      eye,
    );
    return {
      color: d.vec4f(std.add(lit.xyz, paintGlow(paint)), 1),
      fog: fogCoverage(paintedSeen(seen, paint), 1),
    };
  });
  /** The backdrop: the same ground, light and fog, never shadowed. */
  const backdropFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const cell = groundCell(v.world.xy, std.length(std.fwidth(v.world.xy)));
    // Past the map's edge there is no class to read: not ground, to that view.
    if (groundClassView()) {
      return { color: d.vec4f(0, 0, 0, 1), fog: d.vec4f(0, 0, 0, 1) };
    }
    const surface = groundAlbedo(v.world, v.color, cell, groundPaved(v.world.xy, cell));
    const up = std.normalize(v.normal);
    const paint = groundPaint(v.world);
    const albedo = paintedAlbedo(surface.xyz, paint);
    const lit = environment.shade(albedo, d.vec3f(0), surface.w, 0, 0, 1, up, v.world, 1, eye);
    // The rendered margin outside the playable rectangle is permanently fogged.
    const seen = fogTerm(v.world, up, v.clip.xy, fogIsGround());
    return {
      color: d.vec4f(std.add(lit.xyz, paintGlow(paint)), 1),
      fog: fogCoverage(paintedSeen(seen, paint), 1),
    };
  });
  return { terrain: terrainFragment, backdrop: backdropFragment };
}
