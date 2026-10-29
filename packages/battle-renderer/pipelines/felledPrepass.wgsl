fn pastTrunk(d: f32, radius: f32, k: f32) -> f32 {
  let m = abs(d);
  return sign(d) * (min(m, radius) + max(m - radius, 0.0) * k);
}

fn placedNormal(normal: vec3f, shape: vec4f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  let nx = (normal.x / shape.x);
  let ny = (normal.y / shape.y);
  return normalize(vec3f(((nx * c) - (ny * s)), ((nx * s) + (ny * c)), (normal.z / shape.z)));
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

struct felledVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

@vertex fn felledVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f, @location(6) fall: vec4f, @location(7) rest: vec4f, @location(8) lie: vec4f) -> felledVertex_Output {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let yc = cos(pose.w);
  let ys = sin(pose.w);
  let p = vec3f(((local.x * yc) - (local.y * ys)), ((local.x * ys) + (local.y * yc)), local.z);
  let t = fall.xy;
  let c = cos(fall.z);
  let s = sin(fall.z);
  let hinge = fall.w;
  let trunk = rest.z;
  let landed = rest.w;
  let core = select(trunk, 0f, ((color.w > 0.5f) || (local.z > lie.x)));
  let a = dot(p.xy, t);
  let across = (p.xy - (t * a));
  let side = length(across);
  let z = max((p.z - hinge), 0f);
  let flat = ((t * (((a * c) + (z * s)) + ((2f * trunk) * landed))) + (across * (pastTrunk(side, core, rest.y) / max(side, 1e-6f))));
  let world = vec3f((pose.x + flat.x), (pose.y + flat.y), ((((pose.z + (trunk * landed)) + (hinge * (1f - landed))) + (z * c)) - (pastTrunk(a, core, rest.x) * s)));
  let n = placedNormal(normal, shape, pose.w);
  let na = dot(n.xy, t);
  let nAcross = (n.xy - (t * na));
  let pressAcross = select(1f, rest.y, (side > core));
  let pressUp = select(1f, rest.x, (abs(a) > core));
  let nFlat = ((t * ((na * c) + (n.z * s))) + (nAcross * (1f / pressAcross)));
  let heart = (shape.w - hinge);
  return felledVertex_Output((cam.viewProj * vec4f(world, 1f)), world, normalize(vec3f(nFlat.x, nFlat.y, (((n.z * c) - (na * s)) / pressUp))), vec4f((color.xyz * tint.xyz), color.w), vec3f((local.x + tint.w), local.y, local.z), vec3f((pose.x + (t.x * ((heart * s) + ((2f * trunk) * landed)))), (pose.y + (t.y * ((heart * s) + ((2f * trunk) * landed)))), (((pose.z + (trunk * landed)) + (hinge * (1f - landed))) + (heart * c))));
}