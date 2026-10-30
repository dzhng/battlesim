// WGSL owner: passes_scenery.

// @block passes_scenery_grassVertex_Output
struct grassVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}

// @block passes_scenery_dressingVertex
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

// @block passes_scenery_felledVertex
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

// @block passes_scenery_grassVertex
@vertex fn grassVertex(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> grassVertex_Output {
  let g = grassVertexOf(vid, iid);
  return grassVertex_Output(g.clip, g.world, g.root, g.normal, g.albedo, g.ground);
}
// @block passes_scenery_dressingVertex_Output
struct dressingVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_scenery_felledVertex_Output
struct felledVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_scenery_casterVertex_Output
struct casterVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_scenery_casterVertex
@vertex fn casterVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> casterVertex_Output {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, 0f);
  return casterVertex_Output((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), local, vec3f(pose.x, pose.y, (pose.z + shape.w)));
}
