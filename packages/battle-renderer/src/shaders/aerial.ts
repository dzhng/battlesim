import { d } from "typegpu";
import type { LightPresentation } from "../light/sceneLight";
import { aerialParams, aerialHorizonFade, HORIZON_SKY_Z } from "../light/aerialParameters";

// Adapted from ~/dev/game battle-renderer/src/shaders/aerial.ts (reuse
// manifest). Local changes: reads `presentation.light`; a view looking down
// takes its in-scattered light from the sky just above the horizon, not from
// the LUT's ground-bounce hemisphere (which hazed a high camera khaki). The
// light's numbers arrive in the `Aerial` uniform, so one text serves every battle.

/** The haze's per-battle numbers (`aerialParams`), as the environment uniform carries them. */
export const Aerial = d.struct({
  /** Per-channel extinction σ (km⁻¹), and the clear radius around the focus (km). */
  extinction: d.vec3f,
  clearRadiusKm: d.f32,
  /** The sunward Mie tint (linear rgb) and its strength. */
  sunTint: d.vec3f,
  sunStrength: d.f32,
  /** The valley mist's colour (linear rgb) and how much of it the far air takes. */
  mistColor: d.vec3f,
  mistColorStrength: d.f32,
  overviewFromM: d.f32,
  rangeNearM: d.f32,
  rangeFarM: d.f32,
  rangePower: d.f32,
  rangeStrength: d.f32,
  mistOpacity: d.f32,
  sunPower: d.f32,
  /** The mist's height band and distance ramp (metres). */
  mistHeightM: d.vec2f,
  mistDistanceM: d.vec2f,
  /** Where the horizon's sky takes over from the view's, below and above it. */
  horizonFade: d.vec2f,
});

/** The `Aerial` values for `light`. */
export function aerial(light: LightPresentation): d.Infer<typeof Aerial> {
  const p = aerialParams(light);
  const horizon = aerialHorizonFade(p.visibilityKm);
  return {
    extinction: d.vec3f(...p.extinction),
    clearRadiusKm: p.clearRadiusKm,
    sunTint: d.vec3f(...p.sunMieTint),
    sunStrength: p.sunMieStrength,
    mistColor: d.vec3f(...p.valleyMistColor),
    mistColorStrength: p.valleyMistColorStrength,
    overviewFromM: p.overviewFromM,
    rangeNearM: p.rangeFogNearM,
    rangeFarM: p.rangeFogFarM,
    rangePower: p.rangeFogPower,
    rangeStrength: p.rangeFogStrength,
    mistOpacity: p.valleyMistOpacityBoost,
    sunPower: p.sunMiePower,
    mistHeightM: d.vec2f(p.valleyMistHeightBottomM, p.valleyMistHeightTopM),
    mistDistanceM: d.vec2f(p.valleyMistDistanceStartM, p.valleyMistDistanceFullM),
    horizonFade: d.vec2f(horizon.horizonFadeStart, horizon.horizonFadeEnd),
  };
}

const f = (v: number) => `${v.toExponential(16)}f`;

/** Same post-lighting atmospheric function as the production fog node. The caller
 * provides the shared equirectUv function plus borrowed sky texture/sampler;
 * observer is the battle's ground focus, camera is the actual rig eye. */
export const aerialWgsl = `(surface:vec4f,position:vec3f,camera:vec3f,observer:vec3f,sun:vec3f,air:Aerial,lut:texture_2d<f32>,linear:sampler)->vec4f {
    let reach=position-observer;
    let distM=length(reach)/max(1.0,distance(camera,observer)/air.overviewFromM);
    let distKm=max(distM/1000.0-air.clearRadiusKm,0.0);
    let rangeDepth=pow(max(distM-air.rangeNearM,0.0)/air.rangeFarM,air.rangePower)*air.rangeStrength;
    let heightMist=1.0-smoothstep(air.mistHeightM.x,air.mistHeightM.y,position.z);
    let farMist=smoothstep(air.mistDistanceM.x,air.mistDistanceM.y,distM);
    let mistWeight=heightMist*farMist;
    let transmit=exp(-(air.extinction*distKm+vec3f(rangeDepth)+vec3f(mistWeight*air.mistOpacity)));
    let view=normalize(position-camera);
    let viewSky=textureSample(lut,linear,equirectUv(normalize(vec3f(view.xy,max(view.z,${f(HORIZON_SKY_Z)}))))).rgb;
    let horizonView=normalize(vec3f(view.x,view.y,${f(HORIZON_SKY_Z)}));
    let horizonSky=textureSample(lut,linear,equirectUv(horizonView)).rgb;
    let below=smoothstep(air.horizonFade.x,0.0,view.z);
    let above=1.0-smoothstep(0.0,air.horizonFade.y,view.z);
    let sunMie=pow(clamp(dot(view,sun),0.0,1.0),air.sunPower)*air.sunStrength;
    let skyLight=mix(mix(viewSky,horizonSky,below*above),air.sunTint,sunMie);
    let mistSkyLight=mix(skyLight,air.mistColor,mistWeight*air.mistColorStrength);
    return vec4f(surface.rgb*transmit+mistSkyLight*(vec3f(1)-transmit),surface.a);
  }`;
