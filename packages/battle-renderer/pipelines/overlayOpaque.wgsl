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

struct overlayVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

@vertex fn overlayVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> overlayVertex_Output {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let world = vec3f((((position.x * c) - (position.y * s)) + placement.x), (((position.x * s) + (position.y * c)) + placement.y), (position.z + placement.z));
  let clip = (cam.viewProj * vec4f(world, 1f));
  return overlayVertex_Output(vec4f(clip.x, clip.y, (clip.z * 1.003f), clip.w), world, vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z), vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

struct overlayFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

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