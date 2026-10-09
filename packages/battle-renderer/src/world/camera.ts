import { tgpu, d } from "typegpu";
// Binary schema mirrors the published renderer-core Camera uniform, not camera math.
export const Camera = d.struct({
  viewProj: d.mat4x4f,
  invViewProj: d.mat4x4f,
  eye: d.vec3f,
  znear: d.f32,
  width: d.f32,
  height: d.f32,
  time: d.f32,
});
export const typegpuCameraLayout = tgpu
  .bindGroupLayout({ cam: { uniform: Camera, visibility: ["vertex", "fragment"] } })
  .$idx(0);
