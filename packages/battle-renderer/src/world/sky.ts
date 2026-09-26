import { tgpu, d, common } from "typegpu";
import type { SkyModelParams } from "../light/skyParameters";
import { SKY_LUT_WIDTH, SKY_LUT_HEIGHT } from "../light/skyParameters";
import {
  type SkyRays,
  equirectDirectionWgsl,
  equirectUvWgsl,
  skyRadianceWgsl,
  skyDiscWgsl,
} from "../shaders/physicalSky";

const Rays = d.struct({ origin: d.vec3f, dx: d.vec3f, dy: d.vec3f });

/** Linear HDR sky only: no exposure, tone mapping, PMREM or environment lighting. */
export async function createTypegpuSky(
  device: GPUDevice,
  params: SkyModelParams,
  backgroundSamples: 1 | 4 = 1,
) {
  const root = tgpu.initFromDevice({ device });
  const lut = root
    .createTexture({ size: [SKY_LUT_WIDTH, SKY_LUT_HEIGHT], format: "rgba16float" })
    .$usage("render", "sampled");
  const rays = root.createBuffer(Rays).$usage("uniform");
  const sampler = root.createSampler({
    minFilter: "linear",
    magFilter: "linear",
    addressModeU: "clamp-to-edge",
    addressModeV: "clamp-to-edge",
  });
  const direction = tgpu.fn([d.vec2f], d.vec3f)(equirectDirectionWgsl);
  const radiance = tgpu.fn([d.vec3f], d.vec3f)(skyRadianceWgsl(params));
  const equirectUv = tgpu.fn([d.vec3f], d.vec2f)(equirectUvWgsl);
  const disc = tgpu.fn([d.vec3f], d.vec3f)(skyDiscWgsl(params));
  const sample = tgpu
    .fn(
      [d.vec3f],
      d.vec3f,
    )(`(ray: vec3f) -> vec3f {
    let dir = normalize(ray);
    return textureSampleLevel(lutView, linearSampler, equirectUv(dir), 0.0).rgb + disc(dir);
  }`)
    .$uses({ lutView: lut.createView(), linearSampler: sampler, equirectUv, disc });
  // NodeMaterial.setup() applies max(output, 0) after colorNode. Preserve that
  // operation too: omitting it produces nonfinite lower-hemisphere texels.
  const bakeColor = tgpu
    .fn(
      [d.vec2f],
      d.vec4f,
    )(`(uv: vec2f) -> vec4f {
    return max(vec4f(radiance(direction(uv)), 1.0), vec4f(0.0));
  }`)
    .$uses({ radiance, direction });
  const backgroundColor = tgpu
    .fn(
      [d.vec2f],
      d.vec4f,
    )(`(uv: vec2f) -> vec4f {
    return max(vec4f(sample(rays.origin + rays.dx * uv.x + rays.dy * uv.y), 1.0), vec4f(0.0));
  }`)
    .$uses({ sample, rays: rays.as("uniform") });
  const bake = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })(({ uv }) => {
      "use gpu";
      return bakeColor(uv);
    }),
    targets: { format: "rgba16float" },
  });
  const background = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })(({ uv }) => {
      "use gpu";
      return backgroundColor(uv);
    }),
    targets: { format: "rgba16float" },
    multisample: { count: backgroundSamples },
  });
  try {
    await Promise.all([bake.initAsync(), background.initAsync()]);
    const encoder = device.createCommandEncoder({ label: "typegpu-sky-lut" });
    bake
      .with(encoder)
      .withColorAttachment({ view: lut.createView("render"), clearValue: [0, 0, 0, 1] })
      .draw(3);
    device.queue.submit([encoder.finish()]);
  } catch (error) {
    lut.destroy();
    rays.destroy();
    root.destroy();
    throw error;
  }
  return {
    // Native handle only for fixture bindings/readback; TypeGPU owns its lifetime.
    lut: root.unwrap(lut),
    setRays(value: SkyRays) {
      rays.write({
        origin: d.vec3f(...value.origin),
        dx: d.vec3f(...value.dx),
        dy: d.vec3f(...value.dy),
      });
    },
    encodeBackground(encoder: GPUCommandEncoder, target: GPUTextureView) {
      background
        .with(encoder)
        .withColorAttachment({ view: target, clearValue: [0, 0, 0, 1] })
        .draw(3);
    },
    dispose() {
      lut.destroy();
      rays.destroy();
      root.destroy();
    },
  };
}
