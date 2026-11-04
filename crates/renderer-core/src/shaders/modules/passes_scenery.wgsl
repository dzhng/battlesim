// WGSL owner: passes_scenery.

// @block passes_scenery_foliageBackdropFragment
@fragment fn foliageBackdropFragment(input: FoliageFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let seen = fogTerm(input.heart, vec3f(), clip.xy, false);
  return ColorFog(vec4f(scenerySurface(input.world, input.normal, input.color, input.local, 1f), 1f), fogCoverage(seen, 1f));
}

// @block passes_scenery_forestFragment
@fragment fn forestFragment(input: FoliageFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let n = normalize(input.normal);
  let seen = fogTerm(input.heart, vec3f(), clip.xy, false);
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let lit = scenerySurface(input.world, input.normal, input.color, input.local, sunlight);
  return ColorFog(vec4f(lit, 1f), fogCoverage(seen, 1f));
}

// @block passes_scenery_placedVertex
@vertex fn placedVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> FoliageVertexOutput {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, f32(select(0i, 1i, (color.w > 0.5f))));
  return FoliageVertexOutput((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), vec3f((local.x + tint.w), local.y, local.z), vec3f(pose.x, pose.y, (pose.z + shape.w)));
}

// @block passes_scenery_FoliageVertexOutput
struct FoliageVertexOutput {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_scenery_FoliageFragmentInput
struct FoliageFragmentInput {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_scenery_dressingVertex
@vertex fn dressingVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> FoliageVertexOutput {
  let viewCamera = (&cam);
  let row = vec3f(((*viewCamera).viewProj * vec4f(1, 0, 0, 0)).y, ((*viewCamera).viewProj * vec4f(0, 1, 0, 0)).y, ((*viewCamera).viewProj * vec4f(0, 0, 1, 0)).y);
  let pixelsPerMetre = ((0.5f * (*viewCamera).height) * length(row));
  let size = ((tint.w * pixelsPerMetre) / max(distance(pose.xyz, (*viewCamera).eye), 1e-3f));
  let grow = smoothstep(0.5f, 1f, size);
  let local = vec3f(((position.x * shape.x) * grow), ((position.y * shape.y) * grow), ((position.z * shape.z) * grow));
  let world = placedPoint(local, pose);
  return FoliageVertexOutput(((*viewCamera).viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), local, vec3f(pose.x, pose.y, (pose.z + (shape.w * grow))));
}

// @block passes_scenery_felledVertex
@vertex fn felledVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f, @location(6) fall: vec4f, @location(7) rest: vec4f, @location(8) lie: vec4f) -> FoliageVertexOutput {
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
  return FoliageVertexOutput((cam.viewProj * vec4f(world, 1f)), world, normalize(vec3f(nFlat.x, nFlat.y, (((n.z * c) - (na * s)) / pressUp))), vec4f((color.xyz * tint.xyz), color.w), vec3f((local.x + tint.w), local.y, local.z), vec3f((pose.x + (t.x * ((heart * s) + ((2f * trunk) * landed)))), (pose.y + (t.y * ((heart * s) + ((2f * trunk) * landed)))), (((pose.z + (trunk * landed)) + (hinge * (1f - landed))) + (heart * c))));
}

// @block passes_scenery_casterVertex
@vertex fn casterVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> FoliageVertexOutput {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, 0f);
  return FoliageVertexOutput((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), local, vec3f(pose.x, pose.y, (pose.z + shape.w)));
}
