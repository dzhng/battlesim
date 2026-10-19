// Adapted from ~/dev/game battle-renderer/src/shaders/shadow.ts (reuse
// manifest). Local changes: SUN_CASCADES records instead of 2, and receiver
// depth normalised over the receiver range [split near, capped far], the same
// as the breaks.
/** Sun-shadow receiver shading for the world's layered depth texture. */
// The receiver block (`SunShadow`) is the presentation crate's record
// (`shadow.rs`), which documents each cascade's floats.
import { SUN_CASCADES } from "../records.gen";

/** Pinned Three r185 PCFShadowFilter: five Vogel disk taps with per-pixel IGN.
 * Shadow projection/fit and bias remain inputs, not a second lighting policy.
 * The radius is in TEXELS, so a given radius is equal texel softness — not
 * equal world blur across cascade extents or between a 1024 and a 2048 map.
 * The cascade fit supplies each cascade's radius. */
export function shadowPcfWgsl(): string {
  const taps = Array.from({ length: 5 }, (_, i) => {
    const r = Math.sqrt((i + 0.5) / 5);
    const theta = i * 2.399963229728653;
    return [r * Math.cos(theta), r * Math.sin(theta), r];
  });
  const samples = taps
    .map(
      ([x, y], i) =>
        `let b${i}=vec2f(${x},${y});let o${i}=vec2f(b${i}.x*cphi-b${i}.y*sphi,b${i}.x*sphi+b${i}.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o${i},layer,z);`,
    )
    .join("");
  return `(depth:texture_depth_2d_array, compare:sampler_comparison, layer:i32,uv:vec2f, z:f32, pixel:vec2f, radius:f32)->f32 {
  let phi=fract(52.9829189*fract(dot(pixel,vec2f(0.06711056,0.00583715))))*6.28318530718;
  let cphi=cos(phi);let sphi=sin(phi);
  let scale=radius/f32(textureDimensions(depth).x);
  var visibility=0.0;
  ${samples}
  return visibility*0.2;
}`;
}

/** Source reverse-Z coordinate/bias contract; world normal is the material's
 * resolved shading normal. Pixel coordinates are physical fragment coordinates.
 * The `z>=0` half of the bound keeps a receiver past a cascade's far plane from
 * failing the greater-equal comparison against cleared depth and appearing
 * shadowed. */
export function shadowVisibilityWgsl(): string {
  return `(depth:texture_depth_2d_array, compare:sampler_comparison, layer:i32,matrix:mat4x4f, settings:vec4f, world:vec3f, normal:vec3f, pixel:vec2f)->f32 {
  let clip=matrix*vec4f(world+normal*settings.y,1);
  let coord=clip.xyz/clip.w;
  let uv=vec2f(coord.x*0.5+0.5,0.5-coord.y*0.5);
  let z=coord.z-settings.x;
  let inside=all(uv>=vec2f(0))&&all(uv<=vec2f(1))&&z>=0&&z<=1;
  if (!inside) { return 1.0; }
  return shadowPcf(depth,compare,layer,uv,z,pixel,settings.z);
}`;
}

/** `sampleSunShadow(world, normal, pixel)`: the source's cascade fade.
 *  Receiver depth `(-viewZ - n) / (f - n)`, each interval widened by a quarter
 *  of its nearest edge squared, the first cascade unfaded on its near half and
 *  the last fading to unshadowed at the capped far. Across the internal overlap
 *  the two weights sum to one.
 *
 *  Requires the environment module's `environment.worldToView`: the same
 *  admitted frame that produced these fits, not a second view row. */
export function sunShadowSampleWgsl(): string {
  const record = (i: number) => `sunShadow.cascades[${i}]`;
  const sample = (i: number) =>
    `shadowVisibility(sunDepth,sunCompare,${i},${record(i)}.matrix,${record(i)}.bias,world,normal,pixel)`;
  const slice = (i: number, last: boolean) => `
  {
    let interval=${record(i)}.interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=${last ? "interval.y" : "interval.y+margin*0.5"};
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-${sample(i)})*${i === 0 ? "select(1.0,ratio,linearDepth>centre)" : "ratio"};
    }
  }`;
  return `fn sampleSunShadow(world:vec3f,normal:vec3f,pixel:vec2f)->f32 {
  let viewZ=(environment.worldToView*vec4f(world,1.0)).z;
  // Breaks are fractions of the receiver range [control.z, control.x]
  // (split near, capped far); the receiver normalises its depth the same way.
  // The range is sampled on a ray grid, so a sliver of map can sit just
  // nearer than it: the first cascade takes that too.
  let linearDepth=max((-viewZ-sunShadow.control.z)/(sunShadow.control.x-sunShadow.control.z),0.0);
  var shade=1.0;${Array.from({ length: SUN_CASCADES }, (_, i) => slice(i, i === SUN_CASCADES - 1)).join("")}
  return shade;
}`;
}
