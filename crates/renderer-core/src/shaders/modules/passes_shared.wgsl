// WGSL owner: passes_shared.

// @block passes_shared_MeshFragmentInput
struct MeshFragmentInput {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block passes_shared_MeshVertexOutput
struct MeshVertexOutput {
  @invariant @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block passes_shared_fullScreenTriangle
@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> FullScreenVertexOutput {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return FullScreenVertexOutput(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

// @block passes_shared_meshVertex
@vertex fn meshVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> MeshVertexOutput {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let local = position;
  let world = vec3f((((local.x * c) - (local.y * s)) + placement.x), (((local.x * s) + (local.y * c)) + placement.y), (local.z + placement.z));
  let worldNormal = vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z);
  return MeshVertexOutput((cam.viewProj * vec4f(world, 1f)), world, worldNormal, vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

// @block passes_shared_ScreenFragmentInput
struct ScreenFragmentInput {
  @location(0) uv: vec2f,
}

// @block passes_shared_FullScreenVertexOutput
struct FullScreenVertexOutput {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}
