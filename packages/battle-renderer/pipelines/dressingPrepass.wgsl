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

@group(0) @binding(0) var<uniform> cam_1: Camera;

fn placedPoint(local: vec3f, pose: vec4f) -> vec3f {
  let c = cos(pose.w);
  let s = sin(pose.w);
  return vec3f((((local.x * c) - (local.y * s)) + pose.x), (((local.x * s) + (local.y * c)) + pose.y), (local.z + pose.z));
}

fn placedNormal(normal: vec3f, shape: vec4f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  let nx = (normal.x / shape.x);
  let ny = (normal.y / shape.y);
  return normalize(vec3f(((nx * c) - (ny * s)), ((nx * s) + (ny * c)), (normal.z / shape.z)));
}

struct dressingVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

@vertex fn dressingVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> dressingVertex_Output {
  let cam = (&cam_1);
  let row = vec3f(((*cam).viewProj * vec4f(1, 0, 0, 0)).y, ((*cam).viewProj * vec4f(0, 1, 0, 0)).y, ((*cam).viewProj * vec4f(0, 0, 1, 0)).y);
  let pixelsPerMetre = ((0.5f * (*cam).height) * length(row));
  let size = ((tint.w * pixelsPerMetre) / max(distance(pose.xyz, (*cam).eye), 1e-3f));
  let grow = smoothstep(0.5f, 1f, size);
  let local = vec3f(((position.x * shape.x) * grow), ((position.y * shape.y) * grow), ((position.z * shape.z) * grow));
  let world = placedPoint(local, pose);
  return dressingVertex_Output(((*cam).viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), local, vec3f(pose.x, pose.y, (pose.z + (shape.w * grow))));
}