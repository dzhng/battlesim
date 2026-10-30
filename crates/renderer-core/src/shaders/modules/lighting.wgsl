// WGSL owner: lighting.

// @block lighting_shade
fn shade(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec4f {
  return air(surfaceLight(base, emissive, roughness, geomRoughness, metal, ao, normal, position, shadow, eye), position, eye);
}

// @block lighting_unlit
fn unlit(colour: vec3f, position: vec3f, eye: vec3f) -> vec4f {
  let worldPosition = position;
  let observer = environment.observer.xyz;
  let sunDirection = environment.sun.direction.xyz;
  let sunRadiance = environment.sun.radiance.xyz;
  let environmentIntensity = environment.sun.fill.xyz;
  let shadowFloor = environment.sun.settings.y;
  let maxMip = environment.sun.settings.x;
  let air = environment.aerial;
    // What a white matte surface facing the sky returns in sun shadow: the
    // sky's light and the shadow floor's share of the sun's, as shade and
    // standardPbr's diffuse terms have them.
    let skyLight=samplePmrem(pmrem,environmentSampler,vec3f(0.0,0.0,1.0),1.0,maxMip)*environmentIntensity;
    let white=sunRadiance*max(sunDirection.z,0.0)*shadowFloor*0.3183098861837907+skyLight;
    return applyAerial(vec4f(colour*white,1),worldPosition,eye,observer,sunDirection,air,sky,environmentSampler);
  }

// @block lighting_Aerial
struct Aerial {
  extinction: vec3f,
  clearRadiusKm: f32,
  sunTint: vec3f,
  sunStrength: f32,
  mistColor: vec3f,
  mistColorStrength: f32,
  overviewFromM: f32,
  rangeNearM: f32,
  rangeFarM: f32,
  rangePower: f32,
  rangeStrength: f32,
  mistOpacity: f32,
  sunPower: f32,
  mistHeightM: vec2f,
  mistDistanceM: vec2f,
  horizonFade: vec2f,
}

// @block lighting_CastLight
struct CastLight {
  position: vec4f,
  color: vec4f,
}

// @block lighting_CastLights
struct CastLights {
  header: vec4f,
  lights: array<CastLight, LIGHT_CAST_LIGHTS_MAX>,
}

// @block lighting_Environment
struct Sun {
  direction: vec4f,
  radiance: vec4f,
  settings: vec4f,
  fill: vec4f,
}

struct Environment {
  worldToView: mat4x4f,
  observer: vec4f,
  sun: Sun,
  aerial: Aerial,
}

// @block lighting_castAt
/** The cast lights' irradiance at `p` in smoke, over π: each one's falloff,
 *  with no facing (light scatters through smoke). */
fn castAt(p: vec3f) -> vec3f {
  var sum = vec3f(0.0);
  let count = u32(castLights.header.x);
  for (var i = 0u; i < count; i++) {
    let toward = castLights.lights[i].position.xyz - p;
    let x2 = dot(toward, toward) * castLights.lights[i].position.w;
    if (x2 < 1.0) {
      sum += castLights.lights[i].color.rgb * (1.0 - x2) * (1.0 - x2) / (1.0 + 6.0 * x2);
    }
  }
  return sum * 0.31830988;
}

// @block lighting_castLight
fn castLight(base: vec3f, surfaceNormal: vec3f, position: vec3f) -> vec3f {
  let count = u32(castLights.header.x);
  let wrap = castLights.header.y;
  let n = normalize(surfaceNormal);
  var sum = vec3f();
  for (var i = 0u; (i < count); i++) {
    let toward = (castLights.lights[i].position.xyz - position);
    let d2 = dot(toward, toward);
    let x2 = (d2 * castLights.lights[i].position.w);
    if ((x2 < 1f)) {
      let falloff = (((1f - x2) * (1f - x2)) / (1f + (6f * x2)));
      let facing = (dot(n, toward) * inverseSqrt(max(d2, 1e-4f)));
      let wrapped = clamp(((facing + wrap) / (1f + wrap)), 0f, 1f);
      sum = (sum + (castLights.lights[i].color.xyz * (falloff * wrapped)));
    }
  }
  let grey = dot(base, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B));
  let diffuseAlbedo = max(mix(base, vec3f(grey), 0.35f), vec3f(0.10000000149011612));
  return (diffuseAlbedo * (sum * 0.3183098861837907f));
}

// @block lighting_castLights_effects
@group(0) @binding(6) var<uniform> castLights: CastLights;

// @block lighting_environment
@group(GROUP) @binding(0) var<uniform> environment: Environment;

// @block lighting_environment_effects
@group(0) @binding(4) var<uniform> environment: Environment;

// @block lighting_castLights
@group(GROUP) @binding(1) var<uniform> castLights: CastLights;

// @block lighting_surfaceLight
fn surfaceLight(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec3f {
  let sunlight = mix(environment.sun.settings.y, 1f, shadow);
  let surfaceEmission = (emissive + castLight(base, normal, position));
  return standardPbr(base,surfaceEmission,roughness,geomRoughness,metal,ao,normalize(normal),normalize(eye-position),environment.sun.direction.xyz,environment.sun.radiance.xyz,sunlight,environment.sun.fill.xyz,pmrem,environmentSampler,environment.sun.settings.x,dfg,environmentSampler);
}

// @block lighting_air
fn air(colour: vec3f, position: vec3f, eye: vec3f) -> vec4f {
  return applyAerial(vec4f(colour, 1f), position, eye, environment.observer.xyz, environment.sun.direction.xyz, environment.aerial, sky, environmentSampler);
}

// @block lighting_applyAerial
fn applyAerial(surface: vec4f, position: vec3f, camera: vec3f, observer: vec3f, sun: vec3f, air: Aerial, lut: texture_2d<f32>, linearSampler: sampler) -> vec4f {
    let reach=position-observer;
    let distM=length(reach)/max(1.0,distance(camera,observer)/air.overviewFromM);
    let distKm=max(distM/1000.0-air.clearRadiusKm,0.0);
    let rangeDepth=pow(max(distM-air.rangeNearM,0.0)/air.rangeFarM,air.rangePower)*air.rangeStrength;
    let heightMist=1.0-smoothstep(air.mistHeightM.x,air.mistHeightM.y,position.z);
    let farMist=smoothstep(air.mistDistanceM.x,air.mistDistanceM.y,distM);
    let mistWeight=heightMist*farMist;
    let transmit=exp(-(air.extinction*distKm+vec3f(rangeDepth)+vec3f(mistWeight*air.mistOpacity)));
    let view=normalize(position-camera);
    let viewSky=textureSample(lut,linearSampler,equirectUv(normalize(vec3f(view.xy,max(view.z,4.0000000000000001e-3f))))).rgb;
    let horizonView=normalize(vec3f(view.x,view.y,4.0000000000000001e-3f));
    let horizonSky=textureSample(lut,linearSampler,equirectUv(horizonView)).rgb;
    let below=smoothstep(air.horizonFade.x,0.0,view.z);
    let above=1.0-smoothstep(0.0,air.horizonFade.y,view.z);
    let sunMie=pow(clamp(dot(view,sun),0.0,1.0),air.sunPower)*air.sunStrength;
    let skyLight=mix(mix(viewSky,horizonSky,below*above),air.sunTint,sunMie);
    let mistSkyLight=mix(skyLight,air.mistColor,mistWeight*air.mistColorStrength);
    return vec4f(surface.rgb*transmit+mistSkyLight*(vec3f(1)-transmit),surface.a);
  }

// @block lighting_standardPbr
fn standardPbr(base: vec3f, emissive: vec3f, authoredRoughness: f32, geometryRoughness: f32, metal: f32, ao: f32, n: vec3f, v: vec3f, l: vec3f, sunRadiance: vec3f, shadow: f32, environmentIntensity: vec3f, pmrem: texture_2d<f32>, linearSampler: sampler, maxMip: f32, dfg: texture_2d<f32>, dfgSampler: sampler) -> vec3f {
  let roughness=min(max(authoredRoughness,0.0525)+geometryRoughness,1.0);
  let nl=clamp(dot(n,l),0.0,1.0);
  let nv=clamp(dot(n,v),0.0,1.0);
  let h=normalize(l+v);
  let nh=clamp(dot(n,h),0.0,1.0);
  let vh=clamp(dot(v,h),0.0,1.0);
  let alpha=roughness*roughness;
  let a2=alpha*alpha;
  let fresnel=exp2((-5.55473*vh-6.98316)*vh);
  let f0=mix(vec3f(0.04),base,metal);
  let f=f0*(1.0-fresnel)+vec3f(fresnel);
  let gv=nl*sqrt(a2+(1.0-a2)*nv*nv);
  let gl=nv*sqrt(a2+(1.0-a2)*nl*nl);
  let visibility=0.5/max(gv+gl,0.000001);
  let denominator=1.0-nh*nh*(1.0-a2);
  let distribution=a2/(denominator*denominator)*0.3183098861837907;
  let singleDirect=f*visibility*distribution;
  let fabV=textureSampleLevel(dfg,dfgSampler,vec2f(roughness,nv),0.0).rg;
  let fabL=textureSampleLevel(dfg,dfgSampler,vec2f(roughness,nl),0.0).rg;
  let ssV=f0*fabV.x+vec3f(fabV.y);
  let ssL=f0*fabL.x+vec3f(fabL.y);
  let emsV=1.0-(fabV.x+fabV.y);
  let emsL=1.0-(fabL.x+fabL.y);
  let favg=f0+(vec3f(1)-f0)*0.047619;
  let directMs=ssV*ssL*favg/(vec3f(1)-emsV*emsL*favg*favg+vec3f(0.000001))*(emsV*emsL);
  let diffuseContribution=base*(1.0-metal);
  let irradiance=nl*(sunRadiance*shadow);
  let directDiffuse=irradiance*(diffuseContribution*0.3183098861837907);
  let directSpecular=irradiance*(singleDirect+directMs);

  let dielectricSs=vec3f(0.04)*fabV.x+vec3f(fabV.y);
  let dielectricAvg=vec3f(0.04)+(vec3f(1)-vec3f(0.04))*0.047619;
  let dielectricMs=dielectricSs*dielectricAvg/(vec3f(1)-emsV*dielectricAvg)*emsV;
  let metallicSs=base*fabV.x+vec3f(fabV.y);
  let metallicAvg=base+(vec3f(1)-base)*0.047619;
  let metallicMs=metallicSs*metallicAvg/(vec3f(1)-emsV*metallicAvg)*emsV;
  let singleScatter=mix(dielectricSs,metallicSs,metal);
  let multiScatter=mix(dielectricMs,metallicMs,metal);
  let diffuse=diffuseContribution*(vec3f(1)-(dielectricSs+dielectricMs));
  let reflected=normalize(mix(reflect(-v,n),n,roughness*roughness*roughness*roughness));
  let radiance=samplePmrem(pmrem,linearSampler,vec3f(reflected.x,-reflected.y,reflected.z),roughness,maxMip)*environmentIntensity;
  let iblIrradiance=samplePmrem(pmrem,linearSampler,vec3f(n.x,-n.y,n.z),1.0,maxMip)*3.141592653589793*environmentIntensity;
  let cosineIrradiance=iblIrradiance*0.3183098861837907;
  let aoSpecular=clamp(ao-(1.0-pow(nv+ao,exp2(-(1.0+16.0*roughness)))),0.0,1.0);
  let indirectSpecular=(radiance*singleScatter+multiScatter*cosineIrradiance)*aoSpecular;
  let indirectDiffuse=diffuse*cosineIrradiance*ao;
  return max((directDiffuse+indirectDiffuse)+(directSpecular+indirectSpecular)+emissive,vec3f(0));
}

// @block lighting_environmentSun
fn environmentSun() -> vec3f { return environment.sun.direction.xyz; }
