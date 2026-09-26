import { sunDirection, type LightPresentation } from "../light/sceneLight";
import { aerialParams, aerialHorizonFade, HORIZON_SKY_Z } from "../light/aerialParameters";

// Adapted from ~/dev/game battle-renderer/src/shaders/aerial.ts (reuse
// manifest). Local changes: reads `presentation.light`; a view looking down
// takes its in-scattered light from the sky just above the horizon, not from
// the LUT's ground-bounce hemisphere (which hazed a high camera khaki).

/** Same post-lighting atmospheric function as the production fog node. The caller
 * provides the shared equirectUv function plus borrowed sky texture/sampler;
 * observer is the battle's ground focus, camera is the actual rig eye. */
export function aerialWgsl(light: LightPresentation): string {
  const p = aerialParams(light);
  const horizon = aerialHorizonFade(p.visibilityKm);
  const f = (v: number) => `${v.toExponential(16)}f`;
  const rgb = (v: readonly number[]) => `vec3f(${v.map(f).join(",")})`;
  const sun = sunDirection(light);
  return `(surface:vec4f,position:vec3f,camera:vec3f,observer:vec3f,lut:texture_2d<f32>,linear:sampler)->vec4f {
    let reach=position-observer;
    let distM=length(reach);
    let distKm=max(distM/1000.0-${f(p.clearRadiusKm)},0.0);
    let rangeDepth=pow(max(distM-${f(p.rangeFogNearM)},0.0)/${f(p.rangeFogFarM)},${f(p.rangeFogPower)})*${f(p.rangeFogStrength)};
    let heightMist=1.0-smoothstep(${f(p.valleyMistHeightBottomM)},${f(p.valleyMistHeightTopM)},position.z);
    let farMist=smoothstep(${f(p.valleyMistDistanceStartM)},${f(p.valleyMistDistanceFullM)},distM);
    let mistWeight=heightMist*farMist;
    let transmit=exp(-(${rgb(p.extinction)}*distKm+vec3f(rangeDepth)+vec3f(mistWeight*${f(p.valleyMistOpacityBoost)})));
    let view=normalize(position-camera);
    let viewSky=textureSample(lut,linear,equirectUv(normalize(vec3f(view.xy,max(view.z,${f(HORIZON_SKY_Z)}))))).rgb;
    let horizonView=normalize(vec3f(view.x,view.y,${f(HORIZON_SKY_Z)}));
    let horizonSky=textureSample(lut,linear,equirectUv(horizonView)).rgb;
    let below=smoothstep(${f(horizon.horizonFadeStart)},0.0,view.z);
    let above=1.0-smoothstep(0.0,${f(horizon.horizonFadeEnd)},view.z);
    let sunMie=pow(clamp(dot(view,${rgb(sun)}),0.0,1.0),${f(p.sunMiePower)})*${f(p.sunMieStrength)};
    let skyLight=mix(mix(viewSky,horizonSky,below*above),${rgb(p.sunMieTint)},sunMie);
    let mistSkyLight=mix(skyLight,${rgb(p.valleyMistColor)},mistWeight*${f(p.valleyMistColorStrength)});
    return vec4f(surface.rgb*transmit+mistSkyLight*(vec3f(1)-transmit),surface.a);
  }`;
}
