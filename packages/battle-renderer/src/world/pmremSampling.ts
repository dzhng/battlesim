import { tgpu, d } from "typegpu";
import { cubeUvFunctions as cube } from "../shaders/pmrem";
export const cubeFace = tgpu.fn([d.vec3f], d.f32)(cube.cubeFace);
export const cubeUv = tgpu.fn([d.vec3f, d.f32], d.vec2f)(cube.cubeUv);
export const cubeDirection = tgpu.fn([d.vec2f, d.f32], d.vec3f)(cube.cubeDirection);
export const cubeSample = tgpu
  .fn(
    [d.texture2d(), d.sampler(), d.vec3f, d.f32, d.f32],
    d.vec3f,
  )(cube.cubeSample)
  .$uses({ cubeFace, cubeUv });
const roughnessMip = tgpu.fn([d.f32], d.f32)(cube.roughnessMip);
export const samplePmrem = tgpu
  .fn(
    [d.texture2d(), d.sampler(), d.vec3f, d.f32, d.f32],
    d.vec3f,
  )(cube.samplePmrem)
  .$uses({ cubeSample, roughnessMip });
