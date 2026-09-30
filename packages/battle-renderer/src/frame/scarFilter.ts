import { tgpu, d } from "typegpu";
import type { GpuRegistry } from "./registry";

/** Observe normalized-f32 bits before the product: otherwise an inlined
 * cubic tap's /size then *size can cancel its hardware UV rounding boundary.
 * The decoded mantissa product still rounds to f32, as hardware does. */
const scarFilterAxis = tgpu.fn(
  [d.f32, d.f32],
  d.u32,
)(/* wgsl */ `(uv:f32,size:f32)->u32 {
 let bits=bitcast<u32>(clamp(uv,0.0,1.0))&0x7fffffffu;let exponent=bits>>23u;
 let mantissa=(bits&0x7fffffu)|select(0u,0x800000u,exponent!=0u);
 let product=bitcast<u32>(f32(mantissa)*size);
 let value=(product&0x7fffffu)|select(0u,0x800000u,product>>23u!=0u);
 // Each decoded f32 mantissa has exponent bias 150; Q8 adds eight bits.
 let shift=292-i32(exponent)-i32(product>>23u);
 if(shift>=32){return 0u;}if(shift<=0){return value<<u32(-shift);}
 return (value+(1u<<u32(shift-1)))>>u32(shift);
}`);
export const scarFilterPosition = tgpu
  .fn(
    [d.vec2f, d.vec2f],
    d.vec2f,
  )(/* wgsl */ `(uv:vec2f,size:vec2f)->vec2f {
 let q=vec2f(f32(scarFilterAxis(uv.x,size.x)),f32(scarFilterAxis(uv.y,size.y)));
 return clamp((q-128.0)/256.0,vec2f(0.0),size-1.0);
}`)
  .$uses({ scarFilterAxis });

/** The measured RGBA8 filter arithmetic. The startup probe checks the same
 * function against hardware; passing is measured compatibility, not a proof
 * about every coordinate or untested device. */
export const scarFilterQuanta = tgpu.fn(
  [d.vec4u, d.vec4u, d.vec4u, d.vec4u, d.vec2f],
  d.vec4u,
)(/* wgsl */ `(a:vec4u,b:vec4u,c:vec4u,other:vec4u,f:vec2f)->vec4u {
 let wx=u32(floor(f.x*256.0+0.5));let wy=u32(floor(f.y*256.0+0.5));
 let total=a*((256u-wx)*(256u-wy))+b*(wx*(256u-wy))+c*((256u-wx)*wy)+other*(wx*wy);
 return (total+vec4u(2048u))/vec4u(4096u);
}`);

const compatible = new WeakMap<GPUDevice, Promise<void>>();

/** Once per borrowed device, before a source can be consumed. Rejected
 * initialization leaves no cached failure and owns no persistent resources. */
export function groundFilterReady(
  root: ReturnType<typeof tgpu.initFromDevice>,
  registry: GpuRegistry,
  sampler: GPUSampler,
): Promise<void> {
  const device = registry.device;
  let ready = compatible.get(device);
  if (!ready) {
    ready = measureFilter(root, registry, sampler).catch((error) => {
      compatible.delete(device);
      throw error;
    });
    compatible.set(device, ready);
  }
  return ready;
}

async function measureFilter(
  root: ReturnType<typeof tgpu.initFromDevice>,
  registry: GpuRegistry,
  sampler: GPUSampler,
) {
  const device = registry.device,
    scope = registry.scope();
  const patterns = [
    [
      [37, 73, 101, 113],
      [37, 73, 101, 113],
      [91, 77, 31, 43],
      [37, 73, 101, 113],
    ],
    [
      [91, 77, 31, 43],
      [37, 73, 101, 113],
      [37, 73, 101, 113],
      [181, 113, 127, 89],
    ],
    [
      [0, 255, 0, 255],
      [255, 0, 255, 0],
      [255, 0, 0, 255],
      [0, 255, 255, 0],
    ],
    [
      [53, 137, 251, 17],
      [113, 241, 11, 127],
      [181, 23, 79, 223],
      [7, 149, 197, 61],
    ],
    Array.from({ length: 4 }, () => [37, 73, 101, 113]),
  ];
  const points: number[][] = [];
  for (let p = 0; p < patterns.length; p++) {
    for (const y of [0.125, 0.625, 0.91])
      for (let i = 0; i <= 1024; i++) points.push([i / 1024, y, p, 0]);
    for (let i = 0; i < 256; i++)
      for (const epsilon of [-1e-6, 0, 1e-6]) points.push([(i + 0.5) / 256 + epsilon, 0.625, p, 0]);
  }
  const normalized = new Float32Array(4081);
  for (let q = 0; q < normalized.length; q++) normalized[q] = q / 4080;
  const normalizedBits = new Uint32Array(normalized.buffer);
  const layout = tgpu.bindGroupLayout({
    count: { uniform: d.vec4u },
    points: { storage: (n: number) => d.arrayOf(d.vec4f, n), access: "readonly" },
    output: { storage: (n: number) => d.arrayOf(d.vec4u, n), access: "mutable" },
    reference: { texture: d.texture2dArray(d.f32) },
    sampler: { sampler: "filtering" },
  });
  const kernel = tgpu
    .computeFn({ in: { gid: d.builtin.globalInvocationId }, workgroupSize: [64] })(/* wgsl */ `{
 if(gid.x>=layout.$.count.x){return;}let p=layout.$.points[gid.x];let layer=i32(p.z);
 let size=vec2f(textureDimensions(layout.$.reference));let hi=vec2i(size)-1;
 let uv=select((p.xy+0.5)/2.0,p.xy,layout.$.count.y!=0u);
 let position=scarFilterPosition(uv,size);let cell=vec2i(floor(position));
 let a=vec4u(round(textureLoad(layout.$.reference,cell,layer,0)*255.0));
 let b=vec4u(round(textureLoad(layout.$.reference,min(cell+vec2i(1,0),hi),layer,0)*255.0));
 let c=vec4u(round(textureLoad(layout.$.reference,min(cell+vec2i(0,1),hi),layer,0)*255.0));
 let other=vec4u(round(textureLoad(layout.$.reference,min(cell+vec2i(1,1),hi),layer,0)*255.0));
 layout.$.output[gid.x*2u]=scarFilterQuanta(a,b,c,other,fract(position));
 layout.$.output[gid.x*2u+1u]=bitcast<vec4u>(textureSampleLevel(layout.$.reference,layout.$.sampler,uv,layer,0.0));
}`)
    .$uses({ layout, scarFilterQuanta, scarFilterPosition });
  try {
    const pipeline = root.createComputePipeline({ compute: kernel });
    await pipeline.initAsync();
    const count = scope.own(root.createBuffer(d.vec4u).$usage("uniform"));
    const texture = scope.texture({
      label: "ground-filter-reference",
      size: [2, 2, 256],
      format: "rgba8unorm",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
    const input = scope.buffer({
      size: points.length * 16,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    const output = scope.buffer({
      size: points.length * 32,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    });
    const read = scope.buffer({
      size: points.length * 32,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });
    const normalizationPatterns = Array.from({ length: 256 }, (_, v) => [
      [v, v, v, v],
      Array(4).fill(Math.min(255, v + 1)),
      [v, v, v, v],
      Array(4).fill(Math.min(255, v + 1)),
    ]);
    const normalizationPoints = Array.from({ length: 4081 }, (_, q) => [
      (q % 16) / 16,
      0.625,
      Math.floor(q / 16),
      0,
    ]);
    // A non-power-of-two full texture exercises the original global UV path,
    // including fractions close to hardware weight boundaries near its edge.
    const globalTexture = scope.texture({
      label: "ground-filter-global-reference",
      size: [600, 600, 1],
      format: "rgba8unorm",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
    const globalColors = new Uint8Array(600 * 600 * 4);
    for (let y = 0; y < 600; y++)
      for (let x = 0; x < 600; x++)
        globalColors.set(
          [
            (x * 11 + y * 7) % 256,
            (x * 17 + y * 13) % 256,
            (x * 23 + y * 19) % 256,
            (x * 31 + y * 29) % 256,
          ],
          (y * 600 + x) * 4,
        );
    const globalPoints: number[][] = [];
    for (const at of [0, 15, 590])
      for (let i = 0; i <= 2048; i++)
        globalPoints.push([(at + 0.5 + i / 2048) / 600, (at + 0.625) / 600, 0, 0]);
    globalPoints.push(
      [-0, -0, 0, 0],
      [-0.25, 0.25, 0, 0],
      [1e-42, 1e-42, 0, 0],
      [1.25, 1.25, 0, 0],
    );
    for (const batch of [
      {
        texture,
        colors: Uint8Array.from(normalizationPatterns.flat(2)),
        points: normalizationPoints,
        size: [2, 2, 256] as const,
        global: 0,
      },
      {
        texture,
        colors: Uint8Array.from(patterns.flat(2)),
        points,
        size: [2, 2, patterns.length] as const,
        global: 0,
      },
      {
        texture: globalTexture,
        colors: globalColors,
        points: globalPoints,
        size: [600, 600, 1] as const,
        global: 1,
      },
    ]) {
      device.queue.writeTexture(
        { texture: batch.texture },
        batch.colors,
        { bytesPerRow: batch.size[0] * 4, rowsPerImage: batch.size[1] },
        batch.size,
      );
      device.queue.writeBuffer(input, 0, Float32Array.from(batch.points.flat()));
      count.write(d.vec4u(batch.points.length, batch.global, 0, 0));
      const group = root.createBindGroup(layout, {
        count,
        points: input,
        output,
        reference: batch.texture.createView({ dimension: "2d-array" }),
        sampler,
      });
      const encoder = root["~unstable"].createCommandEncoder();
      pipeline
        .with(group)
        .with(encoder)
        .dispatchWorkgroups(Math.ceil(batch.points.length / 64));

      root.unwrap(encoder).copyBufferToBuffer(output, 0, read, 0, batch.points.length * 32);
      encoder.submit();
      await read.mapAsync(GPUMapMode.READ, 0, batch.points.length * 32);
      try {
        const values = new Uint32Array(read.getMappedRange(0, batch.points.length * 32));
        for (let i = 0; i < batch.points.length; i++)
          for (let c = 0; c < 4; c++)
            if (normalizedBits[values[i * 8 + c]] !== values[i * 8 + 4 + c])
              throw new Error(
                `ground filter measured compatibility failed; exact sampling is not admitted on this device: size=${batch.size} point=${batch.points[i]} channel=${c} q=${values[i * 8 + c]} expected=${normalizedBits[values[i * 8 + c]]} hardware=${values[i * 8 + 4 + c]}`,
              );
      } finally {
        read.unmap();
      }
    }
  } finally {
    scope.release();
  }
}
