struct Joint {
  x: vec4f,
  y: vec4f,
  z: vec4f,
}

@group(1) @binding(0) var<storage, read> palette: array<Joint>;

fn moved(j: Joint, p: vec4f) -> vec4f {
  return vec4f(dot(j.x, p), dot(j.y, p), dot(j.z, p), p.w);
}

struct Wind {
  toward: vec3f,
  driftMps: f32,
  gustMps: f32,
  gustPerM: f32,
}

struct Camera {
  viewProj: mat4x4f,
  invViewProj: mat4x4f,
  eye: vec3f,
  znear: f32,
  width: f32,
  height: f32,
  time: f32,
  wind: array<Wind, 1>,
  waves: vec4f,
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
  @location(8) @interpolate(flat) anchor: vec3f,
  @location(9) @interpolate(flat) xray: vec4f,
  @location(10) @interpolate(flat) instance: u32,
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
  return modelVertex_Output((cam.viewProj * vec4f(world, 1f)), world, normal_1, tangent_1, color, vec2f((uv.x - scroll), uv.y), material.x, track, tint.xyz, placement.xyz, vec4f((red / 255f), (green / 255f), (blue / 255f), scale.w), instance);
}

@group(1) @binding(1) var<storage, read> materials: array<vec4f>;

@group(1) @binding(3) var surface: texture_2d_array<f32>;

@group(1) @binding(4) var tiled: sampler;

fn coverageValue(material: u32, uv: vec2f) -> f32 {
  let row = (material * 5u);
  let layer = materials[(row + 4u)].y;
  let image = textureSample(surface, tiled, uv, i32(max(layer, 0f)));
  var value = materials[row].w;
  if ((layer >= 0f)) {
    value = (value * image.w);
  }
  return value;
}

fn cutoutCoverage(material: u32, uv: vec2f) -> f32 {
  let cutoff = materials[((material * 5u) + 4u)].x;
  return saturate(((coverageValue(material, uv) * 0.5f) / max(cutoff, 1e-3f)));
}

fn pixelNoise(pixel: vec2f) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2f(0.06711056, 0.00583715))));
  }

struct modelCutoutCaster_Input {
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
}

@fragment fn modelCutoutCaster(_arg_0: modelCutoutCaster_Input, @builtin(position) clip: vec4f) -> @builtin(sample_mask) u32 {
  return u32((cutoutCoverage(_arg_0.material, _arg_0.uv) > pixelNoise(clip.xy)));
}