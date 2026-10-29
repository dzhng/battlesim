// WGSL owner: passes_effects.

// @block passes_effects_item_alternate
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return compositeOverlay(pos.xy);
}

// @block passes_effects_item_extended
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowRows(pos.xy);
}

// @block passes_effects_item_primary
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowColumns(pos.xy);
}

// @block passes_effects_overlayFragment
@fragment fn overlayFragment(_arg_0: overlayFragment_Input) -> @location(0) vec4f {
  let toEye = (cam.eye - _arg_0.world);
  var n = normalize(_arg_0.normal);
  if ((dot(n, toEye) < 0f)) {
    n = -(n);
  }
  let key = vec3f(0.49965035915374756, -0.5496153831481934, 0.669531524181366);
  let light = max(dot(n, key), 0f);
  let shaded = (_arg_0.color.xyz * (0.3f + (0.75f * light)));
  let lit = (shaded + (vec3f(0.949999988079071, 0.800000011920929, 0.20000000298023224) * (_arg_0.highlight * 0.7f)));
  return vec4f(lit, _arg_0.color.w);
}

// @block passes_effects_overlayVertex
@vertex fn overlayVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> overlayVertex_Output {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let world = vec3f((((position.x * c) - (position.y * s)) + placement.x), (((position.x * s) + (position.y * c)) + placement.y), (position.z + placement.z));
  let clip = (cam.viewProj * vec4f(world, 1f));
  return overlayVertex_Output(vec4f(clip.x, clip.y, (clip.z * 1.003f), clip.w), world, vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z), vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

// @block passes_effects_paintFragment
@fragment fn paintFragment(_arg_0: paintFragment_Input) -> @location(0) vec4f {
  let beat = fract(((cam.time * _arg_0.march.y) - _arg_0.march.x));
  let pulse = (0.5f + (0.5f * cos((beat * 6.2831853f))));
  return vec4f((_arg_0.color.xyz * 0.5f), (_arg_0.color.w * ((1f - _arg_0.march.z) + (_arg_0.march.z * pulse))));
}

// @block passes_effects_paintMarchVertex
@vertex fn paintMarchVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f) -> paintMarchVertex_Output {
  return paintMarchVertex_Output(clipOf(position), normal, color);
}

// @block passes_effects_paintVertex
@vertex fn paintVertex(@location(0) position: vec3f, @location(2) color: vec4f) -> paintVertex_Output {
  return paintVertex_Output(clipOf(position), vec3f(), color);
}
