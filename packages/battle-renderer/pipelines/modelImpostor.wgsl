struct Joint {
  x: vec4f,
  y: vec4f,
  z: vec4f,
}

@group(1) @binding(0) var<storage, read> palette: array<Joint>;

fn moved(j: Joint, p: vec4f) -> vec4f {
  return vec4f(dot(j.x, p), dot(j.y, p), dot(j.z, p), p.w);
}

struct Camera {
  viewProj: mat4x4f,
  invViewProj: mat4x4f,
  eye: vec3f,
  znear: f32,
  width: f32,
  height: f32,
  time: f32,
}

@group(0) @binding(0) var<uniform> cam: Camera;

struct modelVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

@vertex fn modelVertex(@location(0) position: vec3f, @location(1) normal: vec4f, @location(2) uv: vec2f, @location(3) color: vec4f, @location(4) joints: vec4u, @location(5) weights: vec4f, @location(6) tangent: vec4f, @location(7) material: vec2u, @builtin(instance_index) instance: u32, @location(8) placement: vec4f, @location(9) data: vec4f, @location(10) tint: vec4f, @location(11) scale: vec4f) -> modelVertex_Output {
  let base = u32(data.x);
  let p = vec4f(position, 1f);
  let n = vec4f(normal.xyz, 0f);
  let m0 = (&palette[(base + joints.x)]);
  let m1 = (&palette[(base + joints.y)]);
  let m2 = (&palette[(base + joints.z)]);
  let m3 = (&palette[(base + joints.w)]);
  let skinned = (((moved((*m0), p) * weights.x) + (moved((*m1), p) * weights.y)) + ((moved((*m2), p) * weights.z) + (moved((*m3), p) * weights.w)));
  let skinnedNormal = (((moved((*m0), n) * weights.x) + (moved((*m1), n) * weights.y)) + ((moved((*m2), n) * weights.z) + (moved((*m3), n) * weights.w)));
  let t = vec4f(tangent.xyz, 0f);
  let skinnedTangent = (((moved((*m0), t) * weights.x) + (moved((*m1), t) * weights.y)) + ((moved((*m2), t) * weights.z) + (moved((*m3), t) * weights.w)));
  let c = cos(placement.w);
  let s = sin(placement.w);
  let local = (skinned.xyz * scale.xyz);
  let localNormal = (skinnedNormal.xyz / scale.xyz);
  let localTangent = (skinnedTangent.xyz * scale.xyz);
  let world = vec3f((((local.x * c) - (local.y * s)) + placement.x), (((local.x * s) + (local.y * c)) + placement.y), (local.z + placement.z));
  let normal_1 = vec3f(((localNormal.x * c) - (localNormal.y * s)), ((localNormal.x * s) + (localNormal.y * c)), localNormal.z);
  let tangent_1 = vec4f(((localTangent.x * c) - (localTangent.y * s)), ((localTangent.x * s) + (localTangent.y * c)), localTangent.z, tangent.w);
  var scroll = 0f;
  var track = 0f;
  if ((material.y == 1u)) {
    scroll = data.y;
    track = (1f + fract((uv.x - scroll)));
  }
  if ((material.y == 2u)) {
    scroll = data.z;
    track = (1f + fract((uv.x - scroll)));
  }
  let packed = data.w;
  let red = floor((packed / 65536f));
  let green = (floor((packed / 256f)) - (red * 256f));
  let blue = (packed - (floor((packed / 256f)) * 256f));
  return modelVertex_Output((cam.viewProj * vec4f(world, 1f)), world, normal_1, tangent_1, color, vec2f((uv.x - scroll), uv.y), material.x, track, tint.xyz, max(0f, (-1f - tint.w)), placement.xyz, vec4f((red / 255f), (green / 255f), (blue / 255f), scale.w), instance);
}

@group(1) @binding(1) var<storage, read> materials: array<vec4f>;

@group(1) @binding(2) var albedo: texture_2d_array<f32>;

@group(1) @binding(4) var tiled: sampler;

@group(1) @binding(3) var surface: texture_2d_array<f32>;

struct ModelSurface {
  albedo: vec3f,
  normal: vec3f,
  roughness: f32,
  metallic: f32,
  occlusion: f32,
  tint: f32,
}

fn modelSurface(color: vec4f, material: u32, track: f32, uv: vec2f, normal: vec3f, tangent: vec4f) -> ModelSurface {
  let row = (material * 5u);
  let base = (&materials[row]);
  let factors = (&materials[(row + 1u)]);
  let layers = (&materials[(row + 2u)]);
  let worn = (&materials[(row + 3u)]);
  let a = textureSample(albedo, tiled, uv, i32(max((*layers).x, 0f)));
  let nm = textureSample(surface, tiled, uv, i32(max((*layers).y, 0f)));
  let orm = textureSample(surface, tiled, uv, i32(max((*layers).z, 0f)));
  var albedo_1 = ((color.xyz * (*factors).w) * (*base).xyz);
  var threshold = 0.5f;
  if (((*layers).x >= 0f)) {
    albedo_1 = (albedo_1 * a.xyz);
    threshold = a.w;
  }
  else {
    if (((*layers).x <= -2f)) {
      let mean = textureSampleLevel(albedo, tiled, vec2f(0.5), i32((-2f - (*layers).x)), 16);
      albedo_1 = (albedo_1 * mean.xyz);
    }
    else {
      if (((track >= 1f) && ((track - 1f) < 0.5f))) {
        albedo_1 = (albedo_1 * 0.55f);
      }
    }
  }
  var roughness = (*factors).y;
  var metallic = (*factors).x;
  var occlusion = 1f;
  var tint = (*factors).z;
  if (((*layers).z >= 0f)) {
    occlusion = orm.x;
    roughness = (roughness * orm.y);
    metallic = (metallic * orm.z);
    tint = (tint * orm.w);
  }
  var n = normal;
  let tangentLength = length(tangent.xyz);
  if ((((*layers).y >= 0f) && (tangentLength > 0.5f))) {
    let t = normalize((tangent.xyz - (n * dot(n, tangent.xyz))));
    let b = (cross(n, t) * tangent.w);
    let m = ((nm.xyz * 2f) - vec3f(1));
    n = normalize((((t * m.x) + (b * m.y)) + (n * m.z)));
  }
  if (((*layers).w > 0f)) {
    let w = smoothstep((threshold - 0.04f), (threshold + 0.04f), color.w);
    albedo_1 = mix(albedo_1, (*worn).xyz, w);
    roughness = mix(roughness, (*worn).w, w);
    metallic = mix(metallic, 0f, w);
  }
  return ModelSurface(albedo_1, n, roughness, metallic, occlusion, tint);
}

struct impostor_Output {
  @location(0) albedo: vec4f,
  @location(1) normal: vec4f,
  @location(2) mask: vec4f,
}

struct impostor_Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

@fragment fn impostor(_arg_0: impostor_Input) -> impostor_Output {
  let surface_1 = modelSurface(_arg_0.color, _arg_0.material, _arg_0.track, _arg_0.uv, normalize(_arg_0.normal), _arg_0.tangent);
  let albedo_1 = (surface_1.albedo * surface_1.occlusion);
  return impostor_Output(vec4f(pow(max(albedo_1, vec3f()), vec3f(0.4545454680919647)), 1f), vec4f(((surface_1.normal * 0.5f) + vec3f(0.5)), 1f), vec4f(surface_1.tint, 0f, 0f, 1f));
}