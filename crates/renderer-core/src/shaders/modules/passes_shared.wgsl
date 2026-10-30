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

// @block passes_shared_finishFn
@compute @workgroup_size(1) fn finishFn() {
  var total = vec2u(0u);
  for (var tile = 0u; tile < grassParams.grid.x * grassParams.grid.y; tile++) {
    let at = tile * GRASS_TILE_RANGES;
    let count = ranges[at];
    ranges[at] = total;
    total += count;
  }
  for (var t = 0u; t < 2u; t++) {
    atomicStore(&args[10u + t], total[t]);
    atomicStore(&args[t * 5u + 1u], min(total[t], select(grassParams.grid.w, grassParams.grid.z, t == 0u)));
  }
}

// @block passes_shared_fragment_raw
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return rawShader(input.uv);
}

// @block passes_shared_fragment_bloomComposite
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return compositeShader(input.uv);
}

// @block passes_shared_fragment_direct
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return directShader(input.uv);
}

// @block passes_shared_fragment_environmentFilter
@fragment fn fragment(input: EnvironmentFragmentInput) -> @location(0) vec4f {
  return filterShade(input.uv, input.face);
}

// @block passes_shared_fragment_environmentBake
@fragment fn fragment(input: EnvironmentFragmentInput) -> @location(0) vec4f {
  return bakeShade(input.uv, input.face);
}

// @block passes_shared_fragment_final
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return finalShader(input.uv);
}

// @block passes_shared_fragment_highpass
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return highpass(input.uv);
}

// @block passes_shared_fragment_highPlane
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return highPlaneShader(input.uv);
}

// @block passes_shared_fragment_skyBake
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return bakeColor(input.uv);
}

// @block passes_shared_fragment_blur
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return blurShader(input.uv);
}

// @block passes_shared_fullScreenTriangle
@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> FullScreenVertexOutput {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return FullScreenVertexOutput(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

// @block passes_shared_fogRim
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return rimAt(pos.xy);
}

// @block passes_shared_fogDistanceColumns
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceColumns(pos.xy);
}

// @block passes_shared_fogDistanceRows
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceRows(pos.xy);
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

// @block passes_shared_placedVertex
@vertex fn placedVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> FoliageVertexOutput {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, f32(select(0i, 1i, (color.w > 0.5f))));
  return FoliageVertexOutput((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), vec3f((local.x + tint.w), local.y, local.z), vec3f(pose.x, pose.y, (pose.z + shape.w)));
}

// @block passes_shared_vertex
@vertex fn vertex(input: EnvironmentVertexInput) -> EnvironmentVertexOutput {
  return EnvironmentVertexOutput(vec4f(input.position, 1f), input.uv, input.face);
}
// @block passes_shared_EnvironmentFragmentInput
struct EnvironmentFragmentInput {
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

// @block passes_shared_EnvironmentVertexInput
struct EnvironmentVertexInput {
  @location(0) position: vec3f,
  @location(1) uv: vec2f,
  @location(2) face: f32,
}

// @block passes_shared_EnvironmentVertexOutput
struct EnvironmentVertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) face: f32,
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

// @block passes_shared_FoliageVertexOutput
struct FoliageVertexOutput {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_shared_FoliageFragmentInput
struct FoliageFragmentInput {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}
