import type { CivsimEnvironment } from "../../../game-renderer/src/environment/environment";
import {
  aerialParams,
  aerialHorizonFade,
  HORIZON_SKY_Z,
} from "../../../game-renderer/src/environment/aerialParameters";

/** Same post-lighting atmospheric function as the production fog node. The caller
 * provides the shared equirectUv function plus borrowed sky texture/sampler;
 * observer is the battle's ground focus, camera is the actual rig eye. */
export function aerialWgsl(env: CivsimEnvironment): string {
  const p = aerialParams(env);
  const horizon = aerialHorizonFade(p.visibilityKm);
  const f = (v: number) => `${v.toExponential(16)}f`;
  const rgb = (v: readonly number[]) => `vec3f(${v.map(f).join(",")})`;
  const sun = [
    Math.cos(env.sunElevation) * Math.cos(env.sunAzimuth),
    Math.cos(env.sunElevation) * Math.sin(env.sunAzimuth),
    Math.sin(env.sunElevation),
  ];
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
    let viewSky=textureSample(lut,linear,equirectUv(view)).rgb;
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
