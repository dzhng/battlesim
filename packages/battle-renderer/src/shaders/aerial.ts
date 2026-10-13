// Adapted from ~/dev/game battle-renderer/src/shaders/aerial.ts (reuse
// manifest). Local changes: reads `presentation.light`; a view looking down
// takes its in-scattered light from the sky just above the horizon, not from
// the LUT's ground-bounce hemisphere (which hazed a high camera khaki). The
// light's numbers arrive in the `Aerial` uniform, so one text serves every battle.

/** How far above the horizon the haze looks for the horizon's sky. */
const HORIZON_SKY_Z = 0.004;

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
