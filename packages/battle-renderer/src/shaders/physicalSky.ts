// Renderer-independent WGSL bodies. Constants/preset derivation remain owned by
// skyParameters; these mirror SkyModel's TSL operations, including its z clamp.
// The material consumer must also retain NodeMaterial's final max(output, 0).
import * as sky from "../light/skyParameters";

/** Unnormalized world-space ray at UV (0,0), plus the UV x/y increments.
 * Camera projection/orientation belongs to the fixture; translation is excluded. */
export interface SkyRays {
  origin: sky.Rgb;
  dx: sky.Rgb;
  dy: sky.Rgb;
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
