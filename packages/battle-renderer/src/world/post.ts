import { beginGpuAdmission } from "../gpuAdmission";
import { tgpu, d, std, common, type TgpuFn, type TgpuBindGroup } from "typegpu";
import type { PostSettings } from "../light/sceneLight";
import {
  GRADE_LUMA,
  BLOOM_KERNEL_RADII,
  bloomHighpassWgsl,
  bloomBlurWgsl,
  bloomCompositeWgsl,
  gradeColorWgsl,
  agxWgsl,
  outputSrgbWgsl,
  postFinalWgsl,
  postDirectWgsl,
} from "../shaders/post";

/** The grade uniform: `PostSettings` as the final pass reads it. */
const Grade = d.struct({
  saturation: d.f32,
  contrast: d.f32,
  splitTone: d.f32,
  shadowLift: d.f32,
  exposure: d.f32,
  shadowTint: d.vec4f,
  highlightTint: d.vec4f,
  lift: d.vec4f,
});

function gradeUniform(settings: PostSettings) {
  const g = settings.grade;
  const numbers = [
    settings.exposure,
    g.saturation,
    g.contrast,
    g.split_tone,
    g.shadow_lift,
    ...g.shadow_tint,
    ...g.highlight_tint,
    ...g.lift,
  ];
  if (!numbers.every(Number.isFinite)) throw new Error("Post parameters must be finite");
  return {
    saturation: g.saturation,
    contrast: g.contrast,
    splitTone: g.split_tone,
    shadowLift: g.shadow_lift,
    exposure: settings.exposure,
    shadowTint: d.vec4f(...g.shadow_tint, 1),
    highlightTint: d.vec4f(...g.highlight_tint, 1),
    lift: d.vec4f(...g.lift, 0),
  };
}
const sampled = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  source: { texture: d.texture2d() },
});
const compositeLayout = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  level0: { texture: d.texture2d() },
  level1: { texture: d.texture2d() },
  level2: { texture: d.texture2d() },
  level3: { texture: d.texture2d() },
  level4: { texture: d.texture2d() },
});
const finalLayout = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  scene: { texture: d.texture2d() },
  bloom: { texture: d.texture2d() },
  grade: { uniform: Grade },
});

/** Borrowed input/output/device; TypeGPU owns all intermediate HDR resources and
 * pipeline encoding. Adapted (reuse manifest): `settings` (exposure, grade,
 * bloom from `presentation.light`) are fixed for the chain's life, so the
 * bloom's numbers are compiled in and the grade uniform is written once. */
export async function createTypegpuPost(
  device: GPUDevice,
  input: GPUTextureView,
  width: number,
  height: number,
  outputFormat: GPUTextureFormat,
  settings: PostSettings,
) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 64 || height < 64)
    throw new Error("Five-level parity bloom requires a framebuffer at least 64×64");
  if (outputFormat.endsWith("-srgb"))
    throw new Error("Post shader already applies sRGB transfer; output must not encode twice");
  const root = tgpu.initFromDevice({ device });
  const owned: { destroy(): void }[] = [];
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const resource of owned) resource.destroy();
    root.destroy();
  };
  const texture = (w: number, h: number) => {
    const result = root
      .createTexture({ size: [w, h], format: "rgba16float" })
      .$usage("render", "sampled");
    owned.push(result);
    root.unwrap(result);
    return result;
  };
  const admission = beginGpuAdmission(device);
  const init: Promise<unknown>[] = [];
  const admittedGroup = <T extends TgpuBindGroup>(group: T): T => {
    root.unwrap(group);
    return group;
  };
  try {
    const sampler = root.createSampler({ minFilter: "linear", magFilter: "linear" });
    const uniform = root.createBuffer(Grade, gradeUniform(settings)).$usage("uniform");
    owned.push(uniform);
    root.unwrap(uniform);
    root.unwrap(sampler);
    const pipeline = (
      shade: TgpuFn<(uv: d.Vec2f) => d.Vec4f>,
      format: GPUTextureFormat = "rgba16float",
    ) =>
      root.createRenderPipeline({
        vertex: common.fullScreenTriangle,
        fragment: tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })(({ uv }) => {
          "use gpu";
          return shade(uv);
        }),
        targets: { format },
      });
    const stages: ((encoder: GPUCommandEncoder) => void)[] = [];
    const blurStage = (
      body: string,
      source: GPUTextureView,
      target: ReturnType<typeof texture>,
    ) => {
      const algorithm = tgpu.fn([d.vec2f, d.texture2d(), d.sampler()], d.vec4f)(body);
      const shader = tgpu.fn(
        [d.vec2f],
        d.vec4f,
      )((uv) => {
        "use gpu";
        return algorithm(uv, sampled.$.source, sampled.$.linearSampler);
      });
      const p = pipeline(shader).with(
        admittedGroup(root.createBindGroup(sampled, { source, linearSampler: sampler })),
      );
      init.push(p.initAsync());
      const targetView = target.createView("render");
      stages.push((encoder) =>
        p
          .with(encoder)
          .withColorAttachment({ view: targetView, clearValue: [0, 0, 0, 0] })
          .draw(3),
      );
    };
    let w = Math.floor(width / 2),
      h = Math.floor(height / 2);
    const bright = texture(w, h);
    blurStage(bloomHighpassWgsl(settings.bloom), input, bright);
    let source = bright;
    const levels: ReturnType<typeof texture>[] = [];
    let compositeTarget: ReturnType<typeof texture> | undefined;
    for (const radius of BLOOM_KERNEL_RADII) {
      const horizontal = texture(w, h),
        vertical = texture(w, h);
      compositeTarget ??= horizontal;
      blurStage(bloomBlurWgsl(radius, w, h, 0), root.unwrap(source.createView()), horizontal);
      blurStage(bloomBlurWgsl(radius, w, h, 1), root.unwrap(horizontal.createView()), vertical);
      source = vertical;
      levels.push(vertical);
      w = Math.floor(w / 2);
      h = Math.floor(h / 2);
    }
    const compositeAlgorithm = tgpu.fn(
      [
        d.vec2f,
        d.texture2d(),
        d.texture2d(),
        d.texture2d(),
        d.texture2d(),
        d.texture2d(),
        d.sampler(),
      ],
      d.vec4f,
    )(bloomCompositeWgsl(settings.bloom));
    const compositeShader = tgpu.fn(
      [d.vec2f],
      d.vec4f,
    )((uv) => {
      "use gpu";
      return compositeAlgorithm(
        uv,
        compositeLayout.$.level0,
        compositeLayout.$.level1,
        compositeLayout.$.level2,
        compositeLayout.$.level3,
        compositeLayout.$.level4,
        compositeLayout.$.linearSampler,
      );
    });
    const composite = pipeline(compositeShader).with(
      admittedGroup(
        root.createBindGroup(compositeLayout, {
          linearSampler: sampler,
          level0: levels[0],
          level1: levels[1],
          level2: levels[2],
          level3: levels[3],
          level4: levels[4],
        }),
      ),
    );
    init.push(composite.initAsync());
    const compositeView = compositeTarget!.createView("render");
    stages.push((encoder) =>
      composite
        .with(encoder)
        .withColorAttachment({ view: compositeView, clearValue: [0, 0, 0, 0] })
        .draw(3),
    );
    const gradeColor = tgpu
      .fn(
        [d.vec3f, Grade],
        d.vec3f,
      )(gradeColorWgsl)
      .$uses({ LUMA: tgpu.const(d.vec3f, d.vec3f(...GRADE_LUMA)) });
    const agx = tgpu.fn([d.vec3f, d.f32], d.vec3f)(agxWgsl);
    const outputSrgb = tgpu.fn([d.vec3f], d.vec3f)(outputSrgbWgsl);
    const finalAlgorithm = tgpu
      .fn(
        [d.vec2f, d.texture2d(), d.texture2d(), d.sampler(), Grade],
        d.vec4f,
      )(postFinalWgsl)
      .$uses({ gradeColor, agx, outputSrgb });
    const finalShader = tgpu.fn(
      [d.vec2f],
      d.vec4f,
    )((uv) => {
      "use gpu";
      return finalAlgorithm(
        uv,
        finalLayout.$.scene,
        finalLayout.$.bloom,
        finalLayout.$.linearSampler,
        finalLayout.$.grade,
      );
    });
    const directAlgorithm = tgpu
      .fn(
        [d.vec4f, d.f32],
        d.vec4f,
      )(postDirectWgsl)
      .$uses({ agx, outputSrgb });
    const directShader = tgpu.fn(
      [d.vec2f],
      d.vec4f,
    )((uv) => {
      "use gpu";
      return directAlgorithm(
        std.textureSample(finalLayout.$.scene, finalLayout.$.linearSampler, uv),
        finalLayout.$.grade.exposure,
      );
    });
    const direct = pipeline(directShader, outputFormat);
    init.push(direct.initAsync());
    const final = pipeline(finalShader, outputFormat);
    init.push(final.initAsync());
    const zero = texture(1, 1);
    const groups = [zero, compositeTarget!].map((bloom) =>
      admittedGroup(
        root.createBindGroup(finalLayout, {
          linearSampler: sampler,
          scene: input,
          bloom,
          grade: uniform,
        }),
      ),
    );
    const pipelinesReady = Promise.all(init);
    await Promise.all([pipelinesReady, admission()]);
    return {
      encode(encoder: GPUCommandEncoder, output: GPUTextureView, bloom = true, enabled = true) {
        if (disposed) throw new Error("TypeGPU post is disposed");
        if (!enabled) {
          direct
            .with(groups[0])
            .with(encoder)
            .withColorAttachment({ view: output, clearValue: [0, 0, 0, 0] })
            .draw(3);
          return;
        }
        if (bloom) for (const stage of stages) stage(encoder);
        final
          .with(groups[bloom ? 1 : 0])
          .with(encoder)
          .withColorAttachment({ view: output, clearValue: [0, 0, 0, 0] })
          .draw(3);
      },
      dispose,
    };
  } catch (error) {
    await Promise.allSettled([...init, admission()]);
    dispose();
    throw error;
  }
}
