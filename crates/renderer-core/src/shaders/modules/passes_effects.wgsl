// WGSL owner: passes_effects.

// @block passes_effects_OverlayVertexOutput
struct OverlayVertexOutput {
  @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block passes_effects_PaintFragmentInput
struct PaintFragmentInput {
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block passes_effects_PaintVertexOutput
struct PaintVertexOutput {
  @builtin(position) clip: vec4f,
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block passes_effects_compositeOverlay
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return compositeOverlay(pos.xy);
}

// @block passes_effects_glowRows
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowRows(pos.xy);
}

// @block passes_effects_glowColumns
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowColumns(pos.xy);
}

// @block passes_effects_overlayFragment
@fragment fn overlayFragment(input: MeshFragmentInput) -> @location(0) vec4f {
  let toEye = (cam.eye - input.world);
  var n = normalize(input.normal);
  if ((dot(n, toEye) < 0f)) {
    n = -(n);
  }
  let key = vec3f(0.49965035915374756, -0.5496153831481934, 0.669531524181366);
  let light = max(dot(n, key), 0f);
  let shaded = (input.color.xyz * (0.3f + (0.75f * light)));
  let lit = (shaded + (vec3f(0.949999988079071, 0.800000011920929, 0.20000000298023224) * (input.highlight * 0.7f)));
  return vec4f(lit, input.color.w);
}

// @block passes_effects_overlayVertex
@vertex fn overlayVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> OverlayVertexOutput {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let world = vec3f((((position.x * c) - (position.y * s)) + placement.x), (((position.x * s) + (position.y * c)) + placement.y), (position.z + placement.z));
  let clip = (cam.viewProj * vec4f(world, 1f));
  return OverlayVertexOutput(vec4f(clip.x, clip.y, (clip.z * 1.003f), clip.w), world, vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z), vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

// @block passes_effects_paintFragment
@fragment fn paintFragment(input: PaintFragmentInput) -> @location(0) vec4f {
  let beat = fract(((cam.time * input.march.y) - input.march.x));
  let pulse = (0.5f + (0.5f * cos((beat * 6.2831853f))));
  return vec4f((input.color.xyz * 0.5f), (input.color.w * ((1f - input.march.z) + (input.march.z * pulse))));
}

// @block passes_effects_paintMarchVertex
@vertex fn paintMarchVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f) -> PaintVertexOutput {
  return PaintVertexOutput(clipOf(position), normal, color);
}

// @block passes_effects_paintVertex
@vertex fn paintVertex(@location(0) position: vec3f, @location(2) color: vec4f) -> PaintVertexOutput {
  return PaintVertexOutput(clipOf(position), vec3f(), color);
}
