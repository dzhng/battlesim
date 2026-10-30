// WGSL owner: models_geometry.

// @block models_geometry_Joint
struct Joint {
  x: vec4f,
  y: vec4f,
  z: vec4f,
}

// @block models_geometry_cardDirection
fn cardDirection(direction: vec3f, scale: vec3f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  return (vec3f(((direction.x * c) + (direction.y * s)), ((-(direction.x) * s) + (direction.y * c)), direction.z) / scale);
}

// @block models_geometry_cardNormal
fn cardNormal(localNormal: vec3f, scale: vec3f, yaw: f32) -> vec3f {
  return normalize(cardPoint((localNormal / scale), vec3f(1), yaw, vec3f()));
}

// @block models_geometry_cardPoint
fn cardPoint(point: vec3f, scale: vec3f, yaw: f32, at: vec3f) -> vec3f {
  let p = (point * scale);
  let c = cos(yaw);
  let s = sin(yaw);
  return (at + vec3f(((p.x * c) - (p.y * s)), ((p.x * s) + (p.y * c)), p.z));
}

// @block models_geometry_countCoverage
fn countCoverage(clip: vec4f, instance: u32, xray: vec4f) -> vec4f {
    if (xray.w <= 0.0) { discard; }
    atomicAdd(&counts[instance].total, 1u);
    // Same float reverse-Z offset as the hardware depth bias in the draw.
    let biased = bitcast<f32>(bitcast<u32>(clip.z) + 65536u);
    let world = textureLoad(depth, vec2i(clip.xy), 0);
    if (biased < world) { atomicAdd(&counts[instance].hidden, 1u); }
    return vec4f(0.0);
  }

// @block models_geometry_drawCoverage
fn drawCoverage(instance: u32, xray: vec4f) -> vec4f {
    let coverage = counts[instance];
    if (xray.w <= 0.0 || f32(coverage.y) < f32(coverage.x) * minimum) { discard; }
    return vec4f(xray.rgb * xray.a, xray.a);
  }

// @block models_geometry_moved
fn moved(j: Joint, p: vec4f) -> vec4f {
  return vec4f(dot(j.x, p), dot(j.y, p), dot(j.z, p), p.w);
}

// @block models_geometry_palette
@group(1) @binding(0) var<storage, read> palette: array<Joint>;

// @block models_geometry_Count
struct Count {
  total: atomic<u32>,
  hidden: atomic<u32>,
}

// @block models_geometry_albedo_cardLayout
@group(1) @binding(0) var albedo: texture_2d_array<f32>;

// @block models_geometry_counts_xrayReadLayout
@group(2) @binding(0) var<storage, read> counts: array<vec2u>;

// @block models_geometry_counts_xrayCountLayout
@group(2) @binding(0) var<storage, read_write> counts: array<Count>;

// @block models_geometry_frames
@group(1) @binding(3) var<storage, read> frames: array<vec4f>;

// @block models_geometry_minimum
@group(2) @binding(1) var<uniform> minimum: f32;

// @block models_geometry_normal
@group(1) @binding(1) var normal: texture_2d_array<f32>;

// @block models_geometry_depth_xrayCountLayout
@group(2) @binding(1) var depth: texture_depth_multisampled_2d;

// @block models_geometry_materialSampler
@group(1) @binding(2) var materialSampler: sampler;
