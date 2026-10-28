// The wave field (water and wetlands slice 16): a height field over a square
// of water around what the camera looks at, which the wakes Rust decides
// (`EffectFrame::waves`) splat and which spreads as rings and V-wakes. It
// is RedGPU's lake wave equation (`lakeWaveSimulation.wgsl`: a 9-point
// Laplacian, damping, an absorbing border), stepped in fragment passes
// ping-ponging two `rgba16float` targets (no storage textures), at the
// fixed rate and count Rust's clock gives (`wave.rs`), so it spreads alike
// at any frame rate. Each step's height is red, the one before green. The
// water reads the latest from `shown` (`waveNormal.ts`); the renderer
// never decides who makes a wake.
import { tgpu, d, common, type TgpuCommandEncoder } from "typegpu";
import { WAVE, WaveSplat } from "../records.gen";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** Splats one frame takes at most; past it the rest are dropped. */
const MAX_SPLATS = 256;
/** Floats before the splats in `Presentation.waves`: steps, epoch, shift. */
const HEAD = 4;
const SPLAT_FLOATS = 8;
/** How far in from the field's edge its border absorbs a wave (texels). */
const BORDER = 24;

/** One step: which of the frame's it is, how many splats the frame has, and
 *  how far the field moved (texels; the first step only). */
const WaveRun = d.struct({ shift: d.vec2i, step: d.u32, splats: d.u32 });

const waveLayout = tgpu.bindGroupLayout({
  was: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
  run: { uniform: WaveRun, visibility: ["fragment"] },
  splats: {
    storage: d.arrayOf(WaveSplat, MAX_SPLATS),
    access: "readonly",
    visibility: ["fragment"],
  },
});

/** The field one step on at texel `pos` (RedGPU `lakeWaveSimulation.wgsl`),
 *  read moved by the run's shift, with every splat of this step added. */
const waveStep = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pos: vec2f) -> vec4f {
    let size = vec2i(textureDimensions(waveLayout.$.was));
    let c = vec2i(pos);
    let shift = select(vec2i(0), waveLayout.$.run.shift, waveLayout.$.run.step == 0u);
    var h: array<f32, 9>;
    var i = 0;
    for (var y = -1; y <= 1; y++) {
      for (var x = -1; x <= 1; x++) {
        let at = clamp(c + vec2i(x, y), vec2i(0), size - 1) + shift;
        let inside = all(at >= vec2i(0)) && all(at < size);
        h[i] = select(0.0, textureLoad(waveLayout.$.was, at, 0).r, inside);
        i++;
      }
    }
    let here = c + shift;
    let was = select(vec4f(0.0), textureLoad(waveLayout.$.was, here, 0),
      all(here >= vec2i(0)) && all(here < size));
    let laplacian = ((h[1] + h[3] + h[5] + h[7]) * 0.5 + (h[0] + h[2] + h[6] + h[8]) * 0.25
      - 3.0 * h[4]) * 0.333333;
    var next = (2.0 - ${WAVE.damping}) * h[4] - (1.0 - ${WAVE.damping}) * was.g
      + 4.0 * ${WAVE.speed} * ${WAVE.speed} * laplacian;
    var push = 0.0;
    for (var k = 0u; k < waveLayout.$.run.splats; k++) {
      let s = waveLayout.$.splats[k];
      if (u32(s.step) != waveLayout.$.run.step) { continue; }
      // A ring, not a mound: a dip with a rim, which sums to nothing, so
      // pushes made again and again never pile the water up.
      let d2 = dot(pos - s.splat.xy, pos - s.splat.xy) / max(s.splat.z * s.splat.z, 0.25);
      push += s.splat.w * (1.0 - d2) * exp(-d2);
    }
    next = clamp(next + clamp(push, -4.0, 4.0) * 0.25, -1.0, 1.0);
    let edge = min(min(c.x, size.x - 1 - c.x), min(c.y, size.y - 1 - c.y));
    next *= smoothstep(0.0, ${BORDER}.0, f32(edge));
    return vec4f(next, h[4], 0.0, 0.0);
  }`)
  .$uses({ waveLayout });

/** The field's textures, pipeline and the groups each step binds. */
export function createWaveField(root: Root, registry: GpuRegistry) {
  const texture = (label: string, usage: number) =>
    registry.texture({
      label,
      size: [WAVE.texels, WAVE.texels],
      format: "rgba16float",
      usage: GPUTextureUsage.TEXTURE_BINDING | usage,
    });
  const target = GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC;
  const fields = [texture("wave-a", target), texture("wave-b", target)];
  /** The latest step, which the water reads. */
  const shown = texture("wave-shown", GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT);
  const pipeline = root
    .createRenderPipeline({
      vertex: common.fullScreenTriangle,
      fragment: tgpu.fragmentFn({ in: { pos: d.builtin.position }, out: d.vec4f })((v) => {
        "use gpu";
        return waveStep(v.pos.xy);
      }),
      targets: { format: "rgba16float" },
    })
    .$name("waveStep");
  const splats = registry.own(
    root.createBuffer(d.arrayOf(WaveSplat, MAX_SPLATS)).$usage("storage"),
  );
  const runs = Array.from({ length: WAVE.maxSteps }, () =>
    registry.own(root.createBuffer(WaveRun).$usage("uniform")),
  );
  // Each step's group reading each field: step k reads field (k + current) % 2.
  const groups = runs.map((run) =>
    fields.map((was) => root.createBindGroup(waveLayout, { was: was.createView(), run, splats })),
  );
  let current = 0;
  let epoch = -1;
  const clear = (raw: GPUCommandEncoder, views: GPUTexture[]) =>
    raw
      .beginRenderPass({
        label: "wave-clear",
        colorAttachments: views.map((t) => ({
          view: t.createView(),
          loadOp: "clear" as const,
          storeOp: "store" as const,
          clearValue: [0, 0, 0, 0],
        })),
      })
      .end();
  return {
    pipeline,
    /** The latest step's heights, for the water's fog group. */
    view: shown.createView(),
    /** This frame's steps from `waves` (`Presentation.waves`): a new epoch
     *  clears the field first. */
    encode(encoder: TgpuCommandEncoder, waves: Float32Array) {
      if (waves.length < HEAD) return;
      const raw = root.unwrap(encoder);
      const [steps, next, sx, sy] = waves;
      if (next !== epoch) {
        epoch = next;
        clear(raw, [...fields, shown]);
      }
      if (steps === 0) return;
      const count = Math.min((waves.length - HEAD) / SPLAT_FLOATS, MAX_SPLATS);
      if (count > 0)
        root.device.queue.writeBuffer(
          root.unwrap(splats),
          0,
          Float32Array.from(waves.subarray(HEAD, HEAD + count * SPLAT_FLOATS)),
        );
      for (let k = 0; k < steps; k++) {
        runs[k].write({
          shift: k === 0 ? d.vec2i(sx, sy) : d.vec2i(0, 0),
          step: k,
          splats: count,
        });
        pipeline
          .with(encoder)
          .with(groups[k][current])
          .withColorAttachment({
            view: fields[1 - current].createView(),
            loadOp: "clear",
            storeOp: "store",
          })
          .draw(3);
        current = 1 - current;
      }
      raw.copyTextureToTexture({ texture: fields[current] }, { texture: shown }, [
        WAVE.texels,
        WAVE.texels,
      ]);
    },
  };
}

export type WaveField = ReturnType<typeof createWaveField>;
