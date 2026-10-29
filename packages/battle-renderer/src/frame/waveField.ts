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
import { WAVE, WaveSplat, WaveSplatAt, WAVE_SPLAT_WORDS } from "../records.gen";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** Splats one step takes at most; past it the rest of that step's are
 *  dropped. A budget per step, not per frame, so how many steps a frame
 *  covers decides nothing. */
const STEP_SPLATS = 256;
/** Splats a frame holds: every step's budget. */
const MAX_SPLATS = STEP_SPLATS * WAVE.maxSteps;
/** Floats before the splats in `Presentation.waves`: steps, epoch, shift. */
const HEAD = 4;
const SPLAT_FLOATS = WAVE_SPLAT_WORDS;

/** The splats of `packet` (after its head), each step's first `STEP_SPLATS`
 *  kept in order. */
export function budgetedSplats(packet: Float32Array): Float32Array {
  const taken = new Map<number, number>();
  const out: number[] = [];
  for (let o = HEAD; o + SPLAT_FLOATS <= packet.length; o += SPLAT_FLOATS) {
    const step = packet[o + WaveSplatAt.step];
    const n = taken.get(step) ?? 0;
    if (n >= STEP_SPLATS) continue;
    taken.set(step, n + 1);
    for (let k = 0; k < SPLAT_FLOATS; k++) out.push(packet[o + k]);
  }
  return Float32Array.from(out);
}
/** How far in from the field's edge its border absorbs a wave (texels). */
const BORDER = 24;

/** One run: which of the frame's steps it is, how many splats the frame
 *  has, how far the field moved (texels; the first run only), and 1 when it
 *  only moves the field without stepping it. */
const WaveRun = d.struct({ shift: d.vec2i, step: d.u32, splats: d.u32, still: d.u32 });

/** One pass over the field: a step, or a frame with none moving it alone. */
export interface WaveRunPlan {
  shift: [number, number];
  step: number;
  still: boolean;
}

/** What `packet` (`Presentation.waves`) asks of a field last run in epoch
 *  `epoch` (null: no battle). A new epoch, or a battle gone (an empty
 *  packet), clears it first, even with no step to run, so one side's waves
 *  never linger for the next. A frame's first run carries its shift, and a
 *  frame with no step still moves the field by it, so every texel the camera
 *  moves reaches the field. */
export function wavePlan(
  packet: Float32Array,
  epoch: number | null,
): { epoch: number | null; clear: boolean; runs: WaveRunPlan[] } {
  if (packet.length < HEAD) return { epoch: null, clear: epoch !== null, runs: [] };
  const [steps, next, sx, sy] = packet;
  const clear = next !== epoch;
  const runs: WaveRunPlan[] = Array.from({ length: steps }, (_, k) => ({
    shift: k === 0 ? [sx, sy] : [0, 0],
    step: k,
    still: false,
  }));
  if (steps === 0 && !clear && (sx !== 0 || sy !== 0))
    runs.push({ shift: [sx, sy], step: 0, still: true });
  return { epoch: next, clear, runs };
}

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
    if (waveLayout.$.run.still == 1u) { return was; }
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
  let epoch: number | null = null;
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
    /** This frame's runs from `waves` (`Presentation.waves`, `wavePlan`). */
    encode(encoder: TgpuCommandEncoder, waves: Float32Array) {
      const raw = root.unwrap(encoder);
      const plan = wavePlan(waves, epoch);
      epoch = plan.epoch;
      if (plan.clear) clear(raw, [...fields, shown]);
      if (plan.runs.length === 0) return;
      const kept = budgetedSplats(waves);
      const count = kept.length / SPLAT_FLOATS;
      if (count > 0) root.device.queue.writeBuffer(root.unwrap(splats), 0, kept);
      for (const [k, run] of plan.runs.entries()) {
        runs[k].write({
          shift: d.vec2i(...run.shift),
          step: run.step,
          splats: count,
          still: run.still ? 1 : 0,
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
