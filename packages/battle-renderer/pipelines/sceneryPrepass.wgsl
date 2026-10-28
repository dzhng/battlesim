fn placedPoint(local: vec3f, pose: vec4f) -> vec3f {
  let c = cos(pose.w);
  let s = sin(pose.w);
  return vec3f((((local.x * c) - (local.y * s)) + pose.x), (((local.x * s) + (local.y * c)) + pose.y), (local.z + pose.z));
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

fn cameraWind() -> Wind { return cam.wind[0]; }

fn pcgHash(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}

fn sway(foot: vec3f, point: vec3f, up: f32, height: f32, leaf: f32) -> vec3f {
  let wind = cameraWind();
  let t = cam.time;
  let dir = wind.toward.xy;
  let strength = clamp(wind.gustMps / 6, 0.0, 1.0);
  let along = dot(foot.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(0.2, 1.0, front);
  let seed = pcgHash(bitcast<u32>(floor(foot.x * 4.0)) ^ (bitcast<u32>(floor(foot.y * 4.0)) * 2654435761u));
  let phase = f32(seed >> 8u) * (6.2831853 / 16777216.0);
  let swing = sin(t * 1.6 + phase) + 0.25 * sin(1.5 * t * 1.6 + 1.8 * phase);
  let h = clamp(up, 0.0, 1.0);
  let reach = h * h * h;
  let carried = 0.15 + 0.7 * gust + 0.35 * swing * 0.5 * (1.0 + gust);
  let bend = height * 0.18 * strength * carried * reach;
  // Bent, the point keeps its distance from the foot: it drops a little.
  var offset = vec3f(dir * bend, -0.5 * bend * bend / max(height * h, 1.0));
  let q = point - foot;
  let shiver = vec3f(
    sin(t * 7 + q.x * 2.1 + q.y * 0.7 + phase),
    sin(t * 7 * 1.3 + q.y * 1.9 + q.z * 0.8 + phase),
    sin(t * 7 * 0.9 + q.z * 2.3 + q.x * 0.6 + phase),
  );
  offset += shiver * (0.15 * strength * 0.5 * (1.0 + gust) * leaf * h);
  return offset;
}

fn standing(local: vec3f, pose: vec4f, shape: vec4f, leaf: f32) -> vec3f {
  let at = placedPoint(local, pose);
  let height = max((shape.w / 0.62f), 1f);
  return (at + sway(pose.xyz, at, (local.z / height), height, leaf));
}

fn placedNormal(normal: vec3f, shape: vec4f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  let nx = (normal.x / shape.x);
  let ny = (normal.y / shape.y);
  return normalize(vec3f(((nx * c) - (ny * s)), ((nx * s) + (ny * c)), (normal.z / shape.z)));
}

struct placedVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

@vertex fn placedVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> placedVertex_Output {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, f32(select(0i, 1i, (color.w > 0.5f))));
  return placedVertex_Output((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), vec3f((local.x + tint.w), local.y, local.z), vec3f(pose.x, pose.y, (pose.z + shape.w)));
}