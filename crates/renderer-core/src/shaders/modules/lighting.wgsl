// WGSL owner: lighting.

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

// @block lighting_backdropFragment_Input
struct backdropFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block lighting_bakeColor
fn bakeColor(uv: vec2f) -> vec4f {
    return max(vec4f(radiance(direction(uv)), 1.0), vec4f(0.0));
  }

// @block lighting_castAt
/** The cast lights' irradiance at `p` in smoke, over π: each one's falloff,
 *  with no facing (light scatters through smoke). */
fn castAt(p: vec3f) -> vec3f {
  var sum = vec3f(0.0);
  let count = u32(casts.header.x);
  for (var i = 0u; i < count; i++) {
    let toward = casts.lights[i].position.xyz - p;
    let x2 = dot(toward, toward) * casts.lights[i].position.w;
    if (x2 < 1.0) {
      sum += casts.lights[i].color.rgb * (1.0 - x2) * (1.0 - x2) / (1.0 + 6.0 * x2);
    }
  }
  return sum * 0.31830988;
}

// @block lighting_castLight_alternate
fn castLight(base: vec3f, normal_1: vec3f, position: vec3f) -> vec3f {
  let count = u32(lights.header.x);
  let wrap = lights.header.y;
  let n = normalize(normal_1);
  var sum = vec3f();
  for (var i = 0u; (i < count); i++) {
    let toward = (lights.lights[i].position.xyz - position);
    let d2 = dot(toward, toward);
    let x2 = (d2 * lights.lights[i].position.w);
    if ((x2 < 1f)) {
      let falloff = (((1f - x2) * (1f - x2)) / (1f + (6f * x2)));
      let facing = (dot(n, toward) * inverseSqrt(max(d2, 1e-4f)));
      let wrapped = clamp(((facing + wrap) / (1f + wrap)), 0f, 1f);
      sum = (sum + (lights.lights[i].color.xyz * (falloff * wrapped)));
    }
  }
  let grey = dot(base, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B));
  let albedo_1 = max(mix(base, vec3f(grey), 0.35f), vec3f(0.10000000149011612));
  return (albedo_1 * (sum * 0.3183098861837907f));
}

// @block lighting_casts
@group(0) @binding(6) var<uniform> casts: CastLights;

// @block lighting_data_alternate
@group(3) @binding(0) var<uniform> data: Environment;

// @block lighting_data_extended
@group(1) @binding(0) var<uniform> data: Environment;

// @block lighting_data_primary
@group(2) @binding(0) var<uniform> data: Environment;

// @block lighting_disc
fn disc(dir: vec3f) -> vec3f {
    let disc = smoothstep(9.9955000000000005e-1f, 9.9985000000000002e-1f, dot(dir, light.sunDirection))
      * smoothstep(-0.015, 0.01, dir.z) * light.discRadiance;
    return light.sunTransmittance * disc;
  }

// @block lighting_light
@group(0) @binding(4) var<uniform> light: Environment;

// @block lighting_lights_alternate
@group(3) @binding(1) var<uniform> lights: CastLights;

// @block lighting_lights_extended
@group(1) @binding(1) var<uniform> lights: CastLights;

// @block lighting_lights_primary
@group(2) @binding(1) var<uniform> lights: CastLights;

// @block lighting_meshVertex_Output
struct meshVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block lighting_sunCompare_alternate
@group(1) @binding(8) var sunCompare: sampler_comparison;

// @block lighting_sunCompare_extended
@group(3) @binding(8) var sunCompare: sampler_comparison;

// @block lighting_sunCompare_primary
@group(2) @binding(8) var sunCompare: sampler_comparison;

// @block lighting_sunDepth_alternate
@group(3) @binding(7) var sunDepth: texture_depth_2d_array;

// @block lighting_sunDepth_extended
@group(1) @binding(7) var sunDepth: texture_depth_2d_array;

// @block lighting_sunDepth_primary
@group(2) @binding(7) var sunDepth: texture_depth_2d_array;

// @block lighting_worldFragment_Input
struct worldFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block lighting_lightAlgorithm
fn lightAlgorithm(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normalWorld: vec3f, worldPosition: vec3f, shadow: f32, eye: vec3f, sunDirection: vec3f, sunRadiance: vec3f, environmentIntensity: vec3f, maxMip: f32, pmrem: texture_2d<f32>, dfg: texture_2d<f32>, linear: sampler) -> vec3f {
 return standardPbr(base,emissive,roughness,geomRoughness,metal,ao,normalize(normalWorld),normalize(eye-worldPosition),sunDirection,sunRadiance,shadow,environmentIntensity,pmrem,linear,maxMip,dfg,linear);
 }

// @block lighting_surfaceLight
fn surfaceLight(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec3f {
  let sunlight = mix(data.sun.settings.y, 1f, shadow);
  return lightAlgorithm(base, (emissive + castLight(base, normal, position)), roughness, geomRoughness, metal, ao, normal, position, sunlight, eye, data.sun.direction.xyz, data.sun.radiance.xyz, data.sun.fill.xyz, data.sun.settings.x, pmrem, dfg, linear);
}

// @block lighting_air
fn air(colour: vec3f, position: vec3f, eye: vec3f) -> vec4f {
  return applyAerial(vec4f(colour, 1f), position, eye, data.observer.xyz, data.sun.direction.xyz, data.aerial, sky, linear);
}
