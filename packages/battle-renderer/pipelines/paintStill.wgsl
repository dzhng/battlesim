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

fn clipOf(position: vec3f) -> vec4f {
  let toEye = normalize((cam.eye - position));
  let drawn = (position + (toEye * 1f));
  return (cam.viewProj * vec4f(drawn, 1f));
}

struct paintVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

@vertex fn paintVertex(@location(0) position: vec3f, @location(2) color: vec4f) -> paintVertex_Output {
  return paintVertex_Output(clipOf(position), vec3f(), color);
}

struct paintFragment_Input {
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

@fragment fn paintFragment(_arg_0: paintFragment_Input) -> @location(0) vec4f {
  let beat = fract(((cam.time * _arg_0.march.y) - _arg_0.march.x));
  let pulse = (0.5f + (0.5f * cos((beat * 6.2831853f))));
  return vec4f((_arg_0.color.xyz * 0.5f), (_arg_0.color.w * ((1f - _arg_0.march.z) + (_arg_0.march.z * pulse))));
}