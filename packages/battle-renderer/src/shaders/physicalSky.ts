// Renderer-independent WGSL bodies. Constants/preset derivation remain owned by
// skyParameters; these mirror SkyModel's TSL operations, including its z clamp.
// The material consumer must also retain NodeMaterial's final max(output, 0).
import * as sky from "../light/skyParameters";
import type { Vec3 } from "math";

/** Unnormalized world-space ray at UV (0,0), plus the UV x/y increments.
 * Camera projection/orientation belongs to the fixture; translation is excluded. */
export interface SkyRays {
  origin: Readonly<Vec3>;
  dx: Readonly<Vec3>;
  dy: Readonly<Vec3>;
}
const f = (value: number) => `${value.toExponential(16)}f`;
const rgb = (value: readonly number[]) => `vec3f(${value.map(f).join(", ")})`;

// Three's equirectangular storage is y-latitude even though scattering is z-up.
// Keep the inverse pair: changing either rotates the authored physical sky.
export const equirectDirectionWgsl = `(uv: vec2f) -> vec3f {
  let theta = (uv.x - 0.5) * ${f(Math.PI * 2)};
  let phi = (uv.y - 0.5) * ${f(Math.PI)};
  return vec3f(cos(phi) * cos(theta), sin(phi), cos(phi) * sin(theta));
}`;
export const equirectUvWgsl = `(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * ${f(1 / (Math.PI * 2))} + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * ${f(1 / Math.PI)} + 0.5);
}`;

export function skyRadianceWgsl(p: sky.SkyModelParams): string {
  const r0 = sky.PLANET_RADIUS_KM + sky.EYE_ALTITUDE_KM;
  const g2 = sky.MIE_G * sky.MIE_G;
  return `(dirIn: vec3f) -> vec3f {
  let betaR = ${rgb(sky.BETA_RAYLEIGH)};
  let betaMScatter = ${f(sky.BETA_MIE_SCATTER * p.mieScale)};
  let betaMExtinction = ${f(sky.BETA_MIE_EXTINCTION * p.mieScale)};
  let sunDir = ${rgb(p.sunDirection)};
  let mu = max(dirIn.z, 0.004);
  let dir = normalize(vec3f(dirIn.xy, mu));
  let tTop = sqrt(${f(r0 * r0)} * (mu * mu - 1.0) + ${f(sky.ATMOSPHERE_TOP_KM ** 2)}) - mu * ${f(r0)};
  let dt = tTop / 32.0;
  let cosTheta = dot(dir, sunDir);
  let phaseR = (cosTheta * cosTheta + 1.0) * ${f(3 / (16 * Math.PI))};
  let phaseM = ${f((1 - g2) / (4 * Math.PI))} / pow(${f(1 + g2)} - cosTheta * ${f(2 * sky.MIE_G)}, 1.5);
  var radiance = vec3f(0.0);
  var odView = vec3f(0.0);
  for (var i = 0; i < 32; i++) {
    let t = (f32(i) + 0.5) * dt;
    let px = dir.x * t;
    let py = dir.y * t;
    let pz = dir.z * t + ${f(r0)};
    let rp = sqrt(px * px + py * py + pz * pz);
    let h = rp - ${f(sky.PLANET_RADIUS_KM)};
    let rhoR = exp(h / ${f(-sky.RAYLEIGH_SCALE_KM)});
    let rhoM = exp(h / ${f(-sky.MIE_SCALE_KM)});
    odView += (betaR * rhoR + rhoM * betaMExtinction) * dt;
    let tView = exp(-odView);
    let muS = (px * sunDir.x + py * sunDir.y + pz * sunDir.z) / rp;
    let tSunTop = sqrt(rp * rp * (muS * muS - 1.0) + ${f(sky.ATMOSPHERE_TOP_KM ** 2)}) - muS * rp;
    let dts = tSunTop / 6.0;
    var odSun = vec3f(0.0);
    for (var j = 0; j < 6; j++) {
      let ts = (f32(j) + 0.5) * dts;
      let sx = px + sunDir.x * ts;
      let sy = py + sunDir.y * ts;
      let sz = pz + sunDir.z * ts;
      let hs = sqrt(sx * sx + sy * sy + sz * sz) - ${f(sky.PLANET_RADIUS_KM)};
      odSun += (betaR * exp(hs / ${f(-sky.RAYLEIGH_SCALE_KM)}) + exp(hs / ${f(-sky.MIE_SCALE_KM)}) * betaMExtinction) * dts;
    }
    let horizonMu = -sqrt(max(1.0 - ${f(sky.PLANET_RADIUS_KM ** 2)} / (rp * rp), 0.0));
    let shadow = smoothstep(horizonMu - 0.008, horizonMu + 0.004, muS);
    let tSun = exp(-odSun) * shadow;
    let scatterR = betaR * rhoR;
    let scatterM = rhoM * betaMScatter;
    let single = tSun * (scatterR * phaseR + scatterM * phaseM);
    let multiple = (scatterR + scatterM) * ${rgb(sky.MS_TINT)} * ${f((sky.MS_FLOOR * Math.sqrt(Math.max(p.sunDirection[2], 0))) / (4 * Math.PI))};
    radiance += tView * (single + multiple) * dt;
  }
  let aureole = smoothstep(${f(sky.LOW_SUN_AUREOLE_COS_OUTER)}, ${f(sky.LOW_SUN_AUREOLE_COS_INNER)}, cosTheta) * ${f(sky.lowSunAureoleStrength(p.sunDirection[2], p.overcast))};
  let clearSky = radiance * ${f(p.radiance)} + ${rgb(p.sunTransmittance)} * aureole;
  let overcastGradient = 1.05 - max(dirIn.z, 0.0) * 0.22;
  let overcastSky = ${rgb(sky.OVERCAST_ZENITH_RADIANCE)} * overcastGradient * ${f(Math.sqrt(Math.max(p.sunDirection[2], 0.05)))};
  let result = mix(clearSky, overcastSky, ${f(p.overcast)});
  let ground = smoothstep(0.0, 0.35, -dirIn.z);
  return mix(result, result * ${rgb(sky.GROUND_BOUNCE_TINT.map((c) => c + (0.88 - c) * p.overcast))}, ground);
}`;
}

/** Added only when displaying the background, never baked into the IBL LUT. */
export function skyDiscWgsl(p: sky.SkyModelParams): string {
  return `(dir: vec3f) -> vec3f {
    let disc = smoothstep(${f(sky.SUN_DISC_COS_OUTER)}, ${f(sky.SUN_DISC_COS_INNER)}, dot(dir, ${rgb(p.sunDirection)}))
      * smoothstep(-0.015, 0.01, dir.z) * ${f(sky.SUN_DISC_RADIANCE * (1 - p.overcast))};
    return ${rgb(p.sunTransmittance)} * disc;
  }`;
}

// The view's clouds: a cumulus layer `height_m` above the eye, its masses
// value-noise fBm thresholded to the coverage. Each sample is shaded by the
// cloud between it and the sun, so tops toward the sun are lit and bases and
// lee sides are grey; near the horizon they thin into the haze. Integer
// hashing keeps the pattern stable at the layer's large coordinates.
export const cloudHashWgsl = `(i: vec2i) -> f32 {
  var h = (bitcast<u32>(i.x) * 0x8da6b343u) ^ (bitcast<u32>(i.y) * 0xd8163841u);
  h = h ^ (h >> 13u);
  h = h * 0x5bd1e995u;
  h = h ^ (h >> 15u);
  return f32(h & 0xffffffu) / 16777216.0;
}`;
export const cloudNoiseWgsl = `(p: vec2f) -> f32 {
  let i = vec2i(floor(p));
  let f = fract(p);
  let u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2i(1, 0)), u.x), mix(hash(i + vec2i(0, 1)), hash(i + vec2i(1, 1)), u.x), u.y);
}`;
export const cloudFbmWgsl = `(p0: vec2f) -> f32 {
  // Warped, so masses round into heaps rather than streaks.
  var p = p0 + vec2f(noise(p0 * 0.5), noise(p0 * 0.5 + vec2f(5.2, 1.3))) * 0.9;
  var sum = 0.0;
  var amp = 0.55;
  var norm = 0.0;
  for (var k = 0; k < 5; k++) {
    sum += amp * noise(p);
    norm += amp;
    p = vec2f(p.x * 1.7 - p.y * 1.1, p.x * 1.1 + p.y * 1.7) + vec2f(17.3, 9.1);
    amp *= 0.45;
  }
  return sum / norm;
}`;

/** Clouds over the shown sky `sky` along `dir` (z-up); never baked into the LUT. */
export function skyCloudsWgsl(p: sky.SkyModelParams): string {
  const c = p.clouds;
  const toSun =
    Math.hypot(p.sunDirection[0], p.sunDirection[1]) > 1e-6
      ? [p.sunDirection[0], p.sunDirection[1]].map(
          (v) => v / Math.hypot(p.sunDirection[0], p.sunDirection[1]),
        )
      : [1, 0];
  const lo = 1 - c.coverage;
  return `(dir: vec3f, sky: vec3f) -> vec3f {
  if (dir.z < 0.012 || ${f(c.opacity)} <= 0.0) { return sky; }
  // A dome, not a plane: far clouds keep their bulk instead of thinning to
  // streaks along the horizon.
  let at = dir.xy * (${f(c.height_m / c.scale_m)} / (dir.z + 0.12));
  let n = fbm(at);
  let density = smoothstep(${f(lo - c.softness)}, ${f(lo + c.softness)}, n);
  // Volume: read the field as each cloud's height. Its slope faces the sun
  // or turns away; the cloud between a sample and the sun shades it; thick
  // cores grey toward their bases. Tops and sunward flanks stay bright.
  let e = 0.02;
  let slope = vec2f(fbm(at + vec2f(e, 0.0)) - n, fbm(at + vec2f(0.0, e)) - n) / e;
  let normal = normalize(vec3f(-slope * 0.35, 1.0));
  let facing = clamp(dot(normal, ${rgb(p.sunDirection)}) * 0.5 + 0.5, 0.0, 1.0);
  let towardSun = fbm(at + vec2f(${f(toSun[0] * 0.07)}, ${f(toSun[1] * 0.07)}));
  let shadow = smoothstep(${f(lo)}, ${f(lo + 0.3)}, towardSun) * (1.0 - facing);
  let core = smoothstep(${f(lo)}, ${f(lo + 0.35)}, n);
  let lit = ${rgb(p.sunTransmittance)} * ${f(c.radiance)};
  let base = ${rgb(c.shade)} * ${f(c.radiance)} * 0.7 + sky * 0.25;
  var cloud = mix(base, lit, clamp(facing * 1.2 - shadow * 0.5 - core * 0.3, 0.0, 1.0));
  // Thin edges toward the sun glow (forward scattering).
  let silver = pow(max(dot(dir, ${rgb(p.sunDirection)}), 0.0), 6.0) * (1.0 - core);
  cloud += ${rgb(p.sunTransmittance)} * silver * ${f(c.radiance * 0.6)};
  // Far clouds pile toward the horizon and take its haze.
  let near = smoothstep(0.0, 0.3, dir.z);
  cloud = mix(mix(sky, cloud, 0.55), cloud, near);
  let alpha = density * ${f(c.opacity)} * smoothstep(0.012, 0.04, dir.z);
  return mix(sky, cloud, alpha);
}`;
}
