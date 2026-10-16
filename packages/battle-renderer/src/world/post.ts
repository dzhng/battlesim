import { beginGpuAdmission } from "../gpuAdmission";
import { tgpu, d, std, common, type TgpuFn, type TgpuBindGroup } from "typegpu";
import type { ResolvedLight } from "../light/sceneLight";
import { Bloom, Grade } from "../records.gen";
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
  postHighPlaneWgsl,
  postDirectWgsl,
} from "../shaders/post";

const highpassLayout = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  source: { texture: d.texture2d() },
  bloom: { uniform: Bloom },
});
/** One axis of a level's blur; `step` is one texel along it, in uv. */
const blurLayout = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  source: { texture: d.texture2d() },
  step: { uniform: d.vec2f },
});
const compositeLayout = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  level0: { texture: d.texture2d() },
  level1: { texture: d.texture2d() },
  level2: { texture: d.texture2d() },
  level3: { texture: d.texture2d() },
  level4: { texture: d.texture2d() },
  bloom: { uniform: Bloom },
});
const finalLayout = tgpu.bindGroupLayout({
  linearSampler: { sampler: "filtering" },
  scene: { texture: d.texture2d() },
  bloom: { texture: d.texture2d() },
  grade: { uniform: Grade },
});

/** What post makes of the scene: the look (bloom, grade, tone map); the tone
 *  map alone, so a mask's white stays white and its black black; or the
 *  scene's values untouched, for a view whose bytes are data. */
export type PostMode = "look" | "ungraded" | "raw";

/** Borrowed input/output/device; TypeGPU owns all intermediate HDR resources and
 * pipeline encoding. The light's bloom and grade records are fixed for the
 * chain's life, so their uniforms are written once; the shaders depend on
 * neither them nor the size. */
export async function createTypegpuPost(
  device: GPUDevice,
  input: GPUTextureView,
  width: number,
  height: number,
  outputFormat: GPUTextureFormat,
  light: Pick<ResolvedLight, "bloom" | "grade">,
) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 64 || height < 64)
    throw new Error("Five-level parity bloom requires a framebuffer at least 64×64");
  if (outputFormat.endsWith("-srgb"))
    throw new Error("Post shader already applies sRGB transfer; output must not encode twice");
  // Surface pipeline validation errors that would otherwise fail silently.
  device.onuncapturederror = (event) => {
    console.error("WebGPU uncaptured error:", event.error);
  };
  device.pushErrorScope("validation");
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
    const uniform = root.createBuffer(Grade).$usage("uniform");
    uniform.write(light.grade.buffer);
    owned.push(uniform);
    root.unwrap(uniform);
    const bloom = root.createBuffer(Bloom).$usage("uniform");
    bloom.write(light.bloom.buffer);
    owned.push(bloom);
    root.unwrap(bloom);
    root.unwrap(sampler);
    const pipeline = (
      name: string,
      shade: TgpuFn<(uv: d.Vec2f) => d.Vec4f>,
      format: GPUTextureFormat = "rgba16float",
    ) =>
      root
        .createRenderPipeline({
          vertex: common.fullScreenTriangle,
          fragment: tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })(({ uv }) => {
            "use gpu";
            return shade(uv);
          }),
          targets: { format },
        })
        .$name(name);
    const stages: ((encoder: GPUCommandEncoder) => void)[] = [];
    const stage = (
      p: ReturnType<typeof pipeline>,
      group: TgpuBindGroup,
      target: ReturnType<typeof texture>,
    ) => {
      const drawn = p.with(admittedGroup(group));
      init.push(drawn.initAsync());
      const targetView = target.createView("render");
      stages.push((encoder) =>
        drawn
          .with(encoder)
          .withColorAttachment({ view: targetView, clearValue: [0, 0, 0, 0] })
          .draw(3),
      );
    };
    const highpassAlgorithm = tgpu
      .fn(
        [d.vec2f, d.texture2d(), d.sampler(), Bloom],
        d.vec4f,
      )(bloomHighpassWgsl)
      .$uses({ Bloom });
    const highpass = tgpu.fn(
      [d.vec2f],
      d.vec4f,
    )((uv) => {
      "use gpu";
      return highpassAlgorithm(
        uv,
        highpassLayout.$.source,
        highpassLayout.$.linearSampler,
        highpassLayout.$.bloom,
      );
    });
    /** A level's blur, along whichever axis its `step` uniform points. */
    const blur = BLOOM_KERNEL_RADII.map((radius, level) => {
      const algorithm = tgpu.fn(
        [d.vec2f, d.texture2d(), d.sampler(), d.vec2f],
        d.vec4f,
      )(bloomBlurWgsl(radius));
      const shader = tgpu.fn(
        [d.vec2f],
        d.vec4f,
      )((uv) => {
        "use gpu";
        return algorithm(uv, blurLayout.$.source, blurLayout.$.linearSampler, blurLayout.$.step);
      });
      return pipeline(`bloomBlur${level}`, shader);
    });
    const blurStage = (
      level: number,
      source: GPUTextureView,
      step: d.v2f,
      target: ReturnType<typeof texture>,
    ) => {
      const uniform = root.createBuffer(d.vec2f, step).$usage("uniform");
      owned.push(uniform);
      const group = root.createBindGroup(blurLayout, {
        source,
        linearSampler: sampler,
        step: uniform,
      });
      stage(blur[level], group, target);
    };
    let w = Math.floor(width / 2),
      h = Math.floor(height / 2);
    const bright = texture(w, h);
    stage(
      pipeline("bloomHighpass", highpass),
      root.createBindGroup(highpassLayout, { source: input, linearSampler: sampler, bloom }),
      bright,
    );
    let source = bright;
    const levels: ReturnType<typeof texture>[] = [];
    let compositeTarget: ReturnType<typeof texture> | undefined;
    for (const level of BLOOM_KERNEL_RADII.keys()) {
      const horizontal = texture(w, h),
        vertical = texture(w, h);
      compositeTarget ??= horizontal;
      blurStage(level, root.unwrap(source.createView()), d.vec2f(1 / w, 0), horizontal);
      blurStage(level, root.unwrap(horizontal.createView()), d.vec2f(0, 1 / h), vertical);
      source = vertical;
      levels.push(vertical);
      w = Math.floor(w / 2);
      h = Math.floor(h / 2);
    }
    const compositeAlgorithm = tgpu
      .fn(
        [
          d.vec2f,
          d.texture2d(),
          d.texture2d(),
          d.texture2d(),
          d.texture2d(),
          d.texture2d(),
          d.sampler(),
          Bloom,
        ],
        d.vec4f,
      )(bloomCompositeWgsl)
      .$uses({ Bloom });
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
        compositeLayout.$.bloom,
      );
    });
    const composite = pipeline("bloomComposite", compositeShader).with(
      admittedGroup(
        root.createBindGroup(compositeLayout, {
          linearSampler: sampler,
          level0: levels[0],
          level1: levels[1],
          level2: levels[2],
          level3: levels[3],
          level4: levels[4],
          bloom,
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
    const highPlaneAlgorithm = tgpu
      .fn(
        [d.vec2f, d.texture2d(), d.texture2d(), d.sampler(), Grade],
        d.vec4f,
      )(postHighPlaneWgsl)
      .$uses({ gradeColor, agx, outputSrgb });
    const highPlaneShader = tgpu.fn(
      [d.vec2f],
      d.vec4f,
    )((uv) => {
      "use gpu";
      return highPlaneAlgorithm(
        uv,
        finalLayout.$.scene,
        finalLayout.$.bloom,
        finalLayout.$.linearSampler,
        finalLayout.$.grade,
      );
    });
    const highPlane = pipeline("postHighPlane", highPlaneShader, outputFormat);
    init.push(highPlane.initAsync());
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
    const direct = pipeline("postUngraded", directShader, outputFormat);
    init.push(direct.initAsync());
    const rawShader = tgpu.fn(
      [d.vec2f],
      d.vec4f,
    )((uv) => {
      "use gpu";
      const texel = std.textureSample(finalLayout.$.scene, finalLayout.$.linearSampler, uv);
      return d.vec4f(texel.xyz, 1);
    });
    const raw = pipeline("postRaw", rawShader, outputFormat);
    init.push(raw.initAsync());
    const final = pipeline("postLook", finalShader, outputFormat);
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
    // Check for validation errors from pipeline creation.
    const validationError = await device.popErrorScope();
    if (validationError) {
      console.error("WebGPU validation error during post pipeline creation:", validationError);
    }
    return {
      encode(encoder: GPUCommandEncoder, output: GPUTextureView, mode: PostMode = "look") {
        if (disposed) throw new Error("TypeGPU post is disposed");
        if (mode !== "look") {
          (mode === "raw" ? raw : direct)
            .with(groups[0])
            .with(encoder)
            .withColorAttachment({ view: output, clearValue: [0, 0, 0, 0] })
            .draw(3);
          return;
        }
        for (const stage of stages) stage(encoder);
        final
          .with(groups[1])
          .with(encoder)
          .withColorAttachment({ view: output, clearValue: [0, 0, 0, 0] })
          .draw(3);
      },
      /** The high-air plane, drawn into the input since the look was made
       *  from it, into `output` with its coverage premultiplied, under this
       *  frame's bloom. Follows `encode`'s look. */
      encodeHighPlane(encoder: GPUCommandEncoder, output: GPUTextureView) {
        if (disposed) throw new Error("TypeGPU post is disposed");
        highPlane
          .with(groups[1])
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
