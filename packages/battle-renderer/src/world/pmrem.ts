import { cubeDirection, cubeSample } from "./pmremSampling";
import { tgpu, d } from "typegpu";
import { ggxFunctions as ggx, pmremPlanes } from "../shaders/pmrem";
import { equirectUvWgsl } from "../shaders/physicalSky";

const Vertex = d.unstruct({ position: d.vec3f, uv: d.vec2f, face: d.f32 });
const vertices = tgpu.vertexLayout(d.disarrayOf(Vertex));
const sampled = tgpu.bindGroupLayout({
  source: { texture: d.texture2d() },
  linear: { sampler: "filtering" },
});
const filtering = tgpu.bindGroupLayout({
  source: { texture: d.texture2d() },
  linear: { sampler: "filtering" },
  params: { uniform: d.vec4f },
});
const equirectUv = tgpu.fn([d.vec3f], d.vec2f)(equirectUvWgsl);
const radicalInverse = tgpu.fn([d.u32], d.f32)(ggx.radicalInverse);
const importanceGGX = tgpu.fn([d.vec2f, d.f32], d.vec3f)(ggx.importanceGGX);
const convolve = tgpu
  .fn(
    [d.texture2d(), d.sampler(), d.vec3f, d.f32, d.f32, d.f32],
    d.vec3f,
  )(ggx.convolve)
  .$uses({ cubeSample, radicalInverse, importanceGGX });

/** TypeGPU owns atlas resources, typed pipelines and all preparation commands.
 * Its public experimental command encoder supplies viewport/scissor operations;
 * this pinned API dependency is a comparison complexity cost. */
export async function createTypegpuPmrem(device: GPUDevice, sourceLut: GPUTexture) {
  if (sourceLut.width !== 384 || sourceLut.height !== 192 || sourceLut.format !== "rgba16float")
    throw new Error("PMREM comparison expects canonical HDR sky LUT");
  const root = tgpu.initFromDevice({ device });
  const owned: { destroy(): void }[] = [];
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const r of owned) r.destroy();
    root.destroy();
  };
  const maxMip = Math.floor(Math.log2(sourceLut.width / 4)),
    cubeSize = 2 ** maxMip,
    width = 3 * Math.max(cubeSize, 112),
    height = 4 * cubeSize,
    count = maxMip - 4 + 1 + 6;
  try {
    const atlas = root
      .createTexture({ size: [width, height], format: "rgba16float" })
      .$usage("render", "sampled");
    owned.push(atlas);
    const ping = root
      .createTexture({ size: [width, height], format: "rgba16float" })
      .$usage("render", "sampled");
    owned.push(ping);
    const sampler = root.createSampler({ minFilter: "linear", magFilter: "linear" });
    const bakeAlgorithm = tgpu
      .fn(
        [d.vec2f, d.f32, d.texture2d(), d.sampler()],
        d.vec4f,
      )(`(uv:vec2f,face:f32,source:texture_2d<f32>,linear:sampler)->vec4f {
      return textureSampleLevel(source,linear,equirectUv(normalize(cubeDirection(uv,face))),0);
    }`)
      .$uses({ equirectUv, cubeDirection });
    const bakeShade = tgpu.fn(
      [d.vec2f, d.f32],
      d.vec4f,
    )((uv, face) => {
      "use gpu";
      return bakeAlgorithm(uv, face, sampled.$.source, sampled.$.linear);
    });
    const filterAlgorithm = tgpu
      .fn(
        [d.vec2f, d.f32, d.texture2d(), d.sampler(), d.vec4f],
        d.vec4f,
      )(`(uv:vec2f,face:f32,source:texture_2d<f32>,linear:sampler,params:vec4f)->vec4f {
      return vec4f(convolve(source,linear,normalize(cubeDirection(uv,face)),params.x,params.y,params.z),1);
    }`)
      .$uses({ convolve, cubeDirection });
    const filterShade = tgpu.fn(
      [d.vec2f, d.f32],
      d.vec4f,
    )((uv, face) => {
      "use gpu";
      return filterAlgorithm(uv, face, filtering.$.source, filtering.$.linear, filtering.$.params);
    });
    const pipeline = (shade: typeof bakeShade) =>
      root.createRenderPipeline({
        attribs: vertices.attrib,
        vertex: ({ position, uv, face }) => {
          "use gpu";
          return { $position: d.vec4f(position, 1), uv, face };
        },
        fragment: ({ uv, face }) => {
          "use gpu";
          return shade(uv, face);
        },
        targets: { format: "rgba16float" },
      });
    const bake = pipeline(bakeShade),
      filter = pipeline(filterShade);
    await Promise.all([bake.initAsync(), filter.initAsync()]);
    const geometry = Array.from({ length: count }, (_, lod) => {
      const data = pmremPlanes(2 ** Math.max(4, maxMip - lod));
      const records = Array.from({ length: 36 }, (_, v) => ({
        position: d.vec3f(data[v * 6], data[v * 6 + 1], data[v * 6 + 2]),
        uv: d.vec2f(data[v * 6 + 3], data[v * 6 + 4]),
        face: data[v * 6 + 5],
      }));
      const buffer = root.createBuffer(vertices.schemaForCount(36), records).$usage("vertex");
      owned.push(buffer);
      return buffer;
    });
    const encoder = root["~unstable"].createCommandEncoder({ label: "TypeGPU PMREM" });
    const draw = (pipeline: typeof bake, target: typeof atlas, lod: number, clear: boolean) => {
      const size = 2 ** Math.max(4, maxMip - lod),
        x = 3 * size * Math.max(lod - maxMip + 4, 0),
        y = 4 * (cubeSize - size);
      const pass = encoder.beginRenderPass({
        colorAttachments: {
          view: target.createView("render"),
          loadOp: clear ? "clear" : "load",
          storeOp: "store",
          clearValue: [0, 0, 0, 0],
        },
      });
      pass.setViewport(x, y, 3 * size, 2 * size, 0, 1);
      pass.setScissorRect(x, y, 3 * size, 2 * size);
      pipeline.with(vertices, geometry[lod]).with(pass).draw(36);
      pass.end();
    };
    draw(
      bake.with(root.createBindGroup(sampled, { source: sourceLut.createView(), linear: sampler })),
      atlas,
      0,
      true,
    );
    const group = (source: typeof atlas, params: readonly [number, number, number, number]) => {
      const buffer = root.createBuffer(d.vec4f, d.vec4f(...params)).$usage("uniform");
      owned.push(buffer);
      return root.createBindGroup(filtering, {
        source: source.createView(),
        linear: sampler,
        params: buffer,
      });
    };
    for (let lod = 1; lod < count; lod++) {
      const target = lod / (count - 1),
        previous = (lod - 1) / (count - 1),
        roughness = Math.sqrt(target * target - previous * previous) * (target * 1.25);
      draw(
        filter.with(group(atlas, [roughness, maxMip - lod + 1, maxMip, 0])),
        ping,
        lod,
        lod === 1,
      );
      draw(filter.with(group(ping, [0, maxMip - lod, maxMip, 0])), atlas, lod, false);
    }
    encoder.submit();
    return { texture: root.unwrap(atlas), maxMip, width, height, dispose };
  } catch (error) {
    dispose();
    throw error;
  }
}
