// WGSL owner: passes_sky.

// @block passes_sky_backgroundFragment
@fragment fn backgroundFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return backgroundColor(input.uv);
}

// @block passes_sky_environmentFilterFragment
@fragment fn environmentFilterFragment(input: EnvironmentFragmentInput) -> @location(0) vec4f {
  return filterShade(input.uv, input.face);
}

// @block passes_sky_environmentBakeFragment
@fragment fn environmentBakeFragment(input: EnvironmentFragmentInput) -> @location(0) vec4f {
  return bakeShade(input.uv, input.face);
}

// @block passes_sky_skyBakeFragment
@fragment fn skyBakeFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return bakeColor(input.uv);
}

// @block passes_sky_environmentVertex
@vertex fn environmentVertex(input: EnvironmentVertexInput) -> EnvironmentVertexOutput {
  return EnvironmentVertexOutput(vec4f(input.position, 1f), input.uv, input.face);
}

// @block passes_sky_EnvironmentFragmentInput
struct EnvironmentFragmentInput {
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

// @block passes_sky_EnvironmentVertexInput
struct EnvironmentVertexInput {
  @location(0) position: vec3f,
  @location(1) uv: vec2f,
  @location(2) face: f32,
}

// @block passes_sky_EnvironmentVertexOutput
struct EnvironmentVertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) face: f32,
}
