import { tgpu, d } from "typegpu";
// Binary schema mirrors the published renderer-core Camera uniform, not camera math.
export const Camera = d.struct({
  viewProj: d.mat4x4f,
  invViewProj: d.mat4x4f,
  eye: d.vec3f,
  znear: d.f32,
  focus: d.vec2f,
  width: d.f32,
  height: d.f32,
  zoom: d.f32,
  tilt: d.f32,
  time: d.f32,
  zfar: d.f32,
  sunAz: d.f32,
  sunEl: d.f32,
  pad0: d.f32,
  pad1: d.f32,
});
export const typegpuCameraLayout = tgpu
  .bindGroupLayout({ cam: { uniform: Camera, visibility: ["vertex", "fragment"] } })
  .$idx(0);
