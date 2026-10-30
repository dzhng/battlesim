// WGSL owner: passes_shared.

// @block passes_shared_BackdropFragmentInput
struct BackdropFragmentInput {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block passes_shared_meshVertex_Output
struct meshVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block passes_shared_finishFn
@compute @workgroup_size(1) fn finishFn() {
  var total = vec2u(0u);
  for (var tile = 0u; tile < params.grid.x * params.grid.y; tile++) {
    let at = tile * GRASS_TILE_RANGES;
    let count = ranges[at];
    ranges[at] = total;
    total += count;
  }
  for (var t = 0u; t < 2u; t++) {
    atomicStore(&args[10u + t], total[t]);
    atomicStore(&args[t * 5u + 1u], min(total[t], select(params.grid.w, params.grid.z, t == 0u)));
  }
}

// @block passes_shared_fragment_alternate
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return rawShader(_arg_0.uv);
}

// @block passes_shared_fragment_extended
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return compositeShader(_arg_0.uv);
}

// @block passes_shared_fragment_fallback
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return directShader(_arg_0.uv);
}

// @block passes_shared_fragment_layered
@fragment fn fragment(_arg_0: FragmentIn) -> @location(0) vec4f {
  return filterShade(_arg_0.uv, _arg_0.face);
}

// @block passes_shared_fragment_masked
@fragment fn fragment(_arg_0: FragmentIn) -> @location(0) vec4f {
  return bakeShade(_arg_0.uv, _arg_0.face);
}

// @block passes_shared_fragment_primary
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return finalShader(_arg_0.uv);
}

// @block passes_shared_fragment_quaternary
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return highpass(_arg_0.uv);
}

// @block passes_shared_fragment_special
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return highPlaneShader(_arg_0.uv);
}

// @block passes_shared_fragment_tertiary
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return bakeColor(_arg_0.uv);
}

// @block passes_shared_fragment_variant
@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return shader(_arg_0.uv);
}

// @block passes_shared_fullScreenTriangle
@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

// @block passes_shared_item_alternate
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return rimAt(pos.xy);
}

// @block passes_shared_item_extended
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceColumns(pos.xy);
}

// @block passes_shared_item_primary
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceRows(pos.xy);
}

// @block passes_shared_meshVertex
@vertex fn meshVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> meshVertex_Output {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let local = position;
  let world = vec3f((((local.x * c) - (local.y * s)) + placement.x), (((local.x * s) + (local.y * c)) + placement.y), (local.z + placement.z));
  let normal_1 = vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z);
  return meshVertex_Output((cam.viewProj * vec4f(world, 1f)), world, normal_1, vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

// @block passes_shared_placedVertex
@vertex fn placedVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> placedVertex_Output {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, f32(select(0i, 1i, (color.w > 0.5f))));
  return placedVertex_Output((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), vec3f((local.x + tint.w), local.y, local.z), vec3f(pose.x, pose.y, (pose.z + shape.w)));
}

// @block passes_shared_vertex
@vertex fn vertex(_arg_0: VertexIn) -> VertexOut {
  return VertexOut(vec4f(_arg_0.position, 1f), _arg_0.uv, _arg_0.face);
}
// @block passes_shared_FragmentIn
struct FragmentIn {
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

// @block passes_shared_VertexIn
struct VertexIn {
  @location(0) position: vec3f,
  @location(1) uv: vec2f,
  @location(2) face: f32,
}

// @block passes_shared_VertexOut
struct VertexOut {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

// @block passes_shared_fragment_Input
struct fragment_Input {
  @location(0) uv: vec2f,
}

// @block passes_shared_fullScreenTriangle_Output
struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

// @block passes_shared_placedVertex_Output
struct placedVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}
