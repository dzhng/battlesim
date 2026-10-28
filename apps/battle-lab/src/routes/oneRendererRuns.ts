import type { RefObject } from "react";
import { common, d, tgpu } from "typegpu";
import { requestGpuDevice } from "@packages/renderer-core/src/device";
import { eyePosition, screenAxes, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { Renderer } from "@wasm/renderer_spike/renderer_wasm_spike.js";
import { BrowserGpu, Renderer as SharedRenderer } from "@wasm/renderer_browser";
import { loadPresentation, ScenePresentation } from "@web/battle/sim/presentation";
import { gameFogStyle } from "../gameFog";
import { gameLight } from "../gameLight";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "../gameOverlay";
import { groundPatch, FOUNDATION_CAMERA } from "./frame";
import { modelEncoder, modelPipeline } from "./oneRendererModel";
import { capturedParisFixture, realBattleFixture } from "./oneRendererBattle";
import { loadSharedAppearances, loadTankBundle, mw } from "./oneRendererAssets";
import { loadEffectAtlasLayers } from "@web/battle/render/effectAssets";
import { battleRustRun } from "./oneRendererBattleRust";
import { installSceneGeometry } from "@web/battle/render/sceneGeometry";
import { measureFrames } from "./oneRendererTiming";
import { captureInspectorViews, comparisonControls } from "./oneRendererControls";
import { captureSharedQueries, fogQueryPoints } from "./oneRendererQueries";

export const WIDTH = 640;
export const HEIGHT = 360;
export const FRAMES = 120;
export const REPEATS = 3;
export const REAL_WIDTH = 640;
export const REAL_HEIGHT = 360;
const MODEL_CAMERA: Camera3DParams = {
  ...FOUNDATION_CAMERA,
  target: [0, 0, 0.5],
  distance: 24,
};

export type Mode =
  | "typegpu"
  | "exported-wgsl"
  | "rust-wgpu"
  | "mixed"
  | "real-browser"
  | "real-rust-wgpu"
  | "model-browser"
  | "model-rust-wgpu"
  | "battle-browser"
  | "battle-rust-wgpu"
  | "shared-rust-core-browser";

export interface Result {
  mode: Mode;
  setupMs: number;
  encodeMs: number;
  submitMs: number;
  frames: number;
  error?: string;
  workload?: string;
  samples?: Result[];
}

export const LABELS: Record<Mode, string> = {
  typegpu: "TypeGPU authoring + browser WebGPU",
  "exported-wgsl": "Exported WGSL + browser WebGPU",
  "rust-wgpu": "Rust wgpu + BrowserWebGpu WASM",
  mixed: "Rust base + TypeGPU overlay",
  "real-browser": "Real presentation terrain + browser WebGPU",
  "real-rust-wgpu": "Real presentation terrain + Rust wgpu WASM",
  "model-browser": "Authored tank mesh + browser WebGPU",
  "model-rust-wgpu": "Authored tank mesh + Rust wgpu WASM",
  "battle-browser": "Production BattleFrame + browser WebGPU",
  "battle-rust-wgpu": "Battle inputs + Rust wgpu WASM (production-shaped graph)",
  "shared-rust-core-browser": "Shared native/browser Rust core (WASM fixture)",
};

function canvasOf(
  canvas: HTMLCanvasElement,
  device: GPUDevice,
  format: GPUTextureFormat,
  alphaMode: GPUCanvasAlphaMode = "opaque",
) {
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const context = canvas.getContext("webgpu");
  if (!context) throw new Error("Canvas refused a WebGPU context.");
  context.configure({
    device,
    format,
    alphaMode,
    usage: GPUTextureUsage.RENDER_ATTACHMENT,
  });
  return context;
}

function rawPipeline(device: GPUDevice, format: GPUTextureFormat, shader: string) {
  const module = device.createShaderModule({
    label: "one-renderer-exported-wgsl",
    code: shader,
  });
  return device.createRenderPipeline({
    label: "one-renderer-exported-wgsl-pipeline",
    layout: "auto",
    vertex: { module, entryPoint: "vertex" },
    fragment: { module, entryPoint: "fragment", targets: [{ format }] },
    primitive: { topology: "triangle-list" },
  });
}

function encodeRaw(
  device: GPUDevice,
  context: GPUCanvasContext,
  pipeline: GPURenderPipeline,
): void {
  const view = context.getCurrentTexture().createView();
  const encoder = device.createCommandEncoder({
    label: "one-renderer-raw-frame",
  });
  const pass = encoder.beginRenderPass({
    colorAttachments: [
      {
        view,
        clearValue: { r: 0.015, g: 0.025, b: 0.04, a: 1 },
        loadOp: "clear",
        storeOp: "store",
      },
    ],
  });
  pass.setPipeline(pipeline);
  pass.draw(3);
  pass.end();
  device.queue.submit([encoder.finish()]);
}

const REAL_SHADER = `
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32>, @location(1) uv: vec2<f32> };
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Out {
  var out: Out;
  out.position = camera.view_proj * vec4<f32>(position, 1.0);
  out.color = color;
  return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> { return in.color; }
`;

function realPipeline(
  device: GPUDevice,
  format: GPUTextureFormat,
  withDepth = false,
): GPURenderPipeline {
  const module = device.createShaderModule({
    label: "one-renderer-real-ground",
    code: REAL_SHADER,
  });
  return device.createRenderPipeline({
    label: "one-renderer-real-ground-pipeline",
    layout: "auto",
    vertex: {
      module,
      entryPoint: "vertex",
      buffers: [
        {
          arrayStride: 40,
          attributes: [
            { format: "float32x3", offset: 0, shaderLocation: 0 },
            { format: "float32x4", offset: 24, shaderLocation: 1 },
          ],
        },
      ],
    },
    fragment: { module, entryPoint: "fragment", targets: [{ format }] },
    depthStencil: withDepth
      ? {
          format: "depth32float",
          depthWriteEnabled: true,
          depthCompare: "greater",
        }
      : undefined,
  });
}

function encodeReal(
  device: GPUDevice,
  context: GPUCanvasContext,
  pipeline: GPURenderPipeline,
  vertices: Float32Array,
  camera: Float32Array,
): void {
  const vertex = device.createBuffer({
    label: "one-renderer-real-ground-vertices",
    size: vertices.byteLength,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(vertex, 0, vertices);
  const uniform = device.createBuffer({
    label: "one-renderer-real-ground-camera",
    size: 64,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(uniform, 0, camera.buffer, camera.byteOffset, 64);
  const bind = device.createBindGroup({
    layout: pipeline.getBindGroupLayout(0),
    entries: [{ binding: 0, resource: { buffer: uniform } }],
  });
  const view = context.getCurrentTexture().createView();
  const encoder = device.createCommandEncoder({
    label: "one-renderer-real-ground-frame",
  });
  const pass = encoder.beginRenderPass({
    colorAttachments: [
      {
        view,
        clearValue: { r: 0.02, g: 0.03, b: 0.05, a: 1 },
        loadOp: "clear",
        storeOp: "store",
      },
    ],
  });
  pass.setPipeline(pipeline);
  pass.setBindGroup(0, bind);
  pass.setVertexBuffer(0, vertex);
  pass.draw(vertices.length / 10);
  pass.end();
  device.queue.submit([encoder.finish()]);
}

function realFrame(cameraParams: Camera3DParams = FOUNDATION_CAMERA) {
  const world = groundPatch();
  const presentation = new ScenePresentation();
  presentation.setGround(world.terrain.input, world.scenery);
  presentation.setShadow(gameLight, world.terrain.mesh);
  const axes = screenAxes(cameraParams);
  const eye = eyePosition(
    new Float32Array(3) as unknown as Parameters<typeof eyePosition>[0],
    cameraParams,
  );
  const frame = presentation.frame(
    0,
    {
      eye: [...eye],
      target: [...cameraParams.target],
      up: axes.up,
      fovY: cameraParams.fovY,
      near: cameraParams.near,
      width: REAL_WIDTH,
      height: REAL_HEIGHT,
    },
    { selected: [], xray: false, ghosts: [] },
    0,
  );
  const cameraWords = new Float32Array(frame.camera.f32);
  const shadowWords = new Float32Array(frame.sun_shadow.f32);
  const casterWords = new Float32Array(frame.casters.f32);
  const batchWords = new Uint32Array(frame.batches.u32);
  return {
    mesh: world.terrain.mesh,
    camera: cameraWords,
    sun_shadow: shadowWords,
    casters: casterWords,
    batches: batchWords,
    presentation,
  };
}

export async function realBrowserRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  const info = await requestGpuDevice();
  const context = canvasOf(canvas, info.device, info.format);
  const pipeline = realPipeline(info.device, info.format);
  const real = realFrame();
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++)
    encodeReal(info.device, context, pipeline, real.mesh, real.camera);
  const encodeMs = performance.now() - encodeStart;
  await info.device.queue.onSubmittedWorkDone();
  const submitMs = performance.now() - encodeStart - encodeMs;
  real.presentation.free();
  return { mode: "real-browser", setupMs, encodeMs, submitMs, frames: FRAMES };
}

export async function realRustRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  const renderer = await Renderer.new(canvas);
  const real = realFrame();
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++) renderer.render_real_mesh(real.mesh, real.camera);
  const encodeMs = performance.now() - encodeStart;
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const submitMs = performance.now() - encodeStart - encodeMs;
  real.presentation.free();
  renderer.free();
  return {
    mode: "real-rust-wgpu",
    setupMs,
    encodeMs,
    submitMs,
    frames: FRAMES,
  };
}

export async function modelBrowserRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  const info = await requestGpuDevice();
  const context = canvasOf(canvas, info.device, info.format);
  canvas.width = 320;
  canvas.height = 180;
  context.configure({
    device: info.device,
    format: info.format,
    alphaMode: "opaque",
  });
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  context.configure({
    device: info.device,
    format: info.format,
    alphaMode: "opaque",
  });
  const real = realFrame(MODEL_CAMERA);
  const workload = mw(await loadTankBundle());
  const terrain = realPipeline(info.device, info.format, true);
  const model = modelPipeline(info.device, info.format, true);
  const encode = modelEncoder(
    info.device,
    terrain,
    model,
    real.mesh,
    workload,
    real.camera,
    REAL_WIDTH,
    REAL_HEIGHT,
  );
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++) encode(context);
  const encodeMs = performance.now() - encodeStart;
  const submitStart = performance.now();
  await info.device.queue.onSubmittedWorkDone();
  const submitMs = performance.now() - submitStart;
  real.presentation.free();
  return { mode: "model-browser", setupMs, encodeMs, submitMs, frames: FRAMES };
}

export async function modelRustRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  const renderer = await Renderer.new(canvas);
  canvas.width = 320;
  canvas.height = 180;
  renderer.resize(canvas);
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  renderer.resize(canvas);
  const real = realFrame(MODEL_CAMERA);
  const workload = mw(await loadTankBundle());
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++)
    (
      renderer as Renderer & {
        render_real_model: (
          terrain: Float32Array,
          vertices: Float32Array,
          indices: Uint32Array,
          instances: Float32Array,
          texture: Uint8Array,
          textureWidth: number,
          textureHeight: number,
          camera: Float32Array,
        ) => number;
      }
    ).render_real_model(
      real.mesh,
      workload.vertices,
      workload.indices,
      workload.instances,
      workload.texture,
      workload.textureWidth,
      workload.textureHeight,
      real.camera,
    );
  const encodeMs = performance.now() - encodeStart;
  await new Promise<void>((resolve) => {
    let frames = 3;
    const tick = () => (frames-- > 0 ? requestAnimationFrame(tick) : resolve());
    requestAnimationFrame(tick);
  });
  const submitMs = performance.now() - encodeStart - encodeMs;
  real.presentation.free();
  renderer.free();
  return {
    mode: "model-rust-wgpu",
    setupMs,
    encodeMs,
    submitMs,
    frames: FRAMES,
  };
}

export async function sharedCoreBrowserRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  const { memory } = await loadPresentation();
  const phases: Record<string, number> = {};
  const wasmLinearMemoryBytes: Record<string, number> = {};
  let phaseStarted = started;
  const stage = (name: string) => {
    const now = performance.now();
    if (canvas.dataset.stage) phases[canvas.dataset.stage] = now - phaseStarted;
    wasmLinearMemoryBytes[name] = memory.buffer.byteLength;
    canvas.dataset.stage = name;
    phaseStarted = now;
  };
  delete canvas.dataset.stage;
  stage("presentation fixture");
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  const capture = new URLSearchParams(window.location.search).get("capture") === "1";
  const dynamic = capture && new URLSearchParams(window.location.search).get("dynamic") === "1";
  const fixture = capture
    ? await capturedParisFixture(undefined, dynamic ? FRAMES : 0)
    : await realBattleFixture();
  stage("device setup");
  const playerFeatures = new URLSearchParams(window.location.search).get("playerFeatures") === "1";
  canvas.parentElement?.querySelector("canvas[data-high-air]")?.remove();
  const highCanvas = playerFeatures ? document.createElement("canvas") : undefined;
  if (highCanvas && canvas.parentElement) {
    highCanvas.dataset.highAir = "true";
    Object.assign(highCanvas.style, {
      position: "absolute",
      left: "0",
      bottom: "0",
      width: "100%",
      pointerEvents: "none",
    });
    canvas.parentElement.style.position = "relative";
    canvas.parentElement.append(highCanvas);
  }
  const gpu = await BrowserGpu.new();
  let initialFailure: string | undefined;
  void gpu.failure().then((reason) => {
    initialFailure = reason;
  });
  const renderer = await SharedRenderer.new(
    gpu,
    canvas,
    JSON.stringify({
      light: gameLight,
      fog: gameFogStyle,
      grassDensity: fixture.world.grass ? 1 : undefined,
      paint: gamePaint,
      xrayMinHiddenFraction: gameXrayMinHiddenFragmentFraction,
      controls: comparisonControls(),
    }),
    highCanvas,
  );
  stage("asset download and model admission");
  const assetAccounting = await loadSharedAppearances(renderer, fixture.presentation);
  renderer.load_effect_atlas(await loadEffectAtlasLayers());
  installSceneGeometry(renderer, fixture.presentation, fixture.world, gameLight);
  renderer.set_overlay_glow(gameOverlayGlow.radius_px, gameOverlayGlow.strength);
  renderer.set_overlays(
    fixture.overlay.opaque,
    fixture.overlay.translucent,
    fixture.overlay.painted,
    fixture.overlay.paintedMarching,
  );
  if (fixture.pointer) renderer.set_pointer_marks(fixture.pointer);
  stage("canonical runtime installation");
  fixture.presentation.installRenderer(renderer);
  await renderer.prepare_assets();
  const exportValidation = await renderer.validate_export();
  const startupMs = performance.now() - started;
  stage("initial frame proof");
  fixture.presentation.render(renderer);
  await renderer.drain();
  await captureInspectorViews(
    canvas,
    (view) => {
      fixture.presentation.setRendererControls(renderer, { ...comparisonControls(), view });
      fixture.presentation.render(renderer);
    },
    () => renderer.drain(),
  );
  const fogInitialUploadComplete = (JSON.parse(renderer.stats()) as { fogTablesReady: boolean })
    .fogTablesReady;
  await captureSharedQueries(renderer, fixture.presentation, canvas);
  const queryPoints =
    new URLSearchParams(window.location.search).get("readbacks") === "1"
      ? fogQueryPoints(fixture.presentation)
      : null;
  const beforeProbe = queryPoints
    ? Array.from(await fixture.presentation.probeFog(renderer, queryPoints))
    : null;
  const beforeRecovery = canvas.toDataURL();
  const beforeHighRecovery = highCanvas?.toDataURL();
  canvas.dataset.recoveryBefore = beforeRecovery;
  if (beforeHighRecovery) canvas.dataset.highRecoveryBefore = beforeHighRecovery;
  stage("resize and surface recovery");
  canvas.width = 320;
  canvas.height = 180;
  fixture.presentation.resizeRenderer(renderer, canvas);
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  fixture.presentation.resizeRenderer(renderer, canvas);
  renderer.recover_surface();
  // Consume resize's retained-table refresh before losing the device, so a
  // recovery query must initialize its own new GPU owner.
  fixture.presentation.render(renderer);
  await renderer.drain();
  renderer.destroy_device_for_test();
  stage("device loss callback");
  await new Promise<void>((resolve) => setTimeout(resolve, 50));
  const lostBeforeRecovery = JSON.parse(renderer.stats()) as {
    deviceLost: number;
  };
  if (lostBeforeRecovery.deviceLost < 1) {
    throw new Error(`device loss callback did not fire: ${renderer.stats()}`);
  }
  if (!initialFailure?.startsWith("WebGPU device lost"))
    throw new Error(
      `unexpected initial GPU failure: ${initialFailure ?? "missing loss notification"}`,
    );
  stage("device recovery");
  const recoveredGpu = await BrowserGpu.new();
  let recoveredFailure: string | undefined;
  void recoveredGpu.failure().then((reason) => {
    recoveredFailure = reason;
  });
  await renderer.recover_device(recoveredGpu, canvas);
  gpu.dispose();
  gpu.free();
  stage("recovered knowledge preparation");
  fixture.presentation.installRenderer(renderer);
  await renderer.prepare_assets();
  if (queryPoints) {
    canvas.dataset.fogRecoveryProbe = JSON.stringify({
      before: beforeProbe,
      recovered: Array.from(await fixture.presentation.probeFog(renderer, queryPoints)),
    });
  }
  fixture.presentation.render(renderer);
  await renderer.drain();
  const afterRecovery = canvas.toDataURL();
  canvas.dataset.recoveryAfter = afterRecovery;
  const afterHighRecovery = highCanvas?.toDataURL();
  if (afterHighRecovery) canvas.dataset.highRecoveryAfter = afterHighRecovery;
  const recoveryPixelsMatch =
    beforeRecovery === afterRecovery && beforeHighRecovery === afterHighRecovery;
  const setupMs = startupMs;
  renderer.set_gpu_timing(new URLSearchParams(window.location.search).get("timing") === "1");
  renderer.reset_gpu_timing();
  stage("frame encode");
  const measured = await measureFrames(
    FRAMES,
    () => {
      fixture.changing?.next(renderer.static_generation());
      fixture.presentation.render(renderer);
    },
    () => renderer.drain(),
  );
  if (capture) canvas.dataset.finalFrame = canvas.toDataURL();
  if (highCanvas) canvas.dataset.finalHighFrame = highCanvas.toDataURL();
  stage("complete");
  const workload = JSON.stringify({
    ...JSON.parse(renderer.stats()),
    ...assetAccounting,
    fogInitialUploadComplete,
    recoveryPixelsMatch,
    timing: measured.timing,
    gpu: renderer.gpu_timing(),
    allocations: renderer.memory(),
    phases,
    wasmLinearMemoryBytes,
    export: JSON.parse(exportValidation),
    ...(fixture.changing ? { changing: fixture.changing.result() } : {}),
  });
  renderer.free();
  recoveredGpu.dispose();
  const disposedFailure = await recoveredGpu.failure();
  recoveredGpu.free();
  if (recoveredFailure || disposedFailure)
    throw new Error(`recovered GPU failed: ${recoveredFailure ?? disposedFailure}`);
  if (!capture || dynamic) fixture.presentation.free();
  return {
    mode: "shared-rust-core-browser",
    setupMs,
    encodeMs: measured.encodeMs,
    submitMs: measured.submitMs,
    frames: FRAMES,
    workload,
  };
}

export async function browserRun(
  mode: "typegpu" | "exported-wgsl",
  canvas: HTMLCanvasElement,
): Promise<Result> {
  const started = performance.now();
  const info = await requestGpuDevice();
  const context = canvasOf(canvas, info.device, info.format);
  const shader = Renderer.shader_source();
  const root = mode === "typegpu" ? tgpu.initFromDevice({ device: info.device }) : null;
  const fragment =
    mode === "typegpu"
      ? tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })`{
          let glow = 0.04 + 0.18 * in.uv.x + 0.22 * in.uv.y;
          return vec4f(0.06 + glow, 0.16 + glow * 0.65, 0.28 + glow * 0.35, 1.0);
        }`
      : null;
  const pipeline = mode === "exported-wgsl" ? rawPipeline(info.device, info.format, shader) : null;
  const typegpuPipeline =
    mode === "typegpu" && root && fragment
      ? root.createRenderPipeline({
          vertex: common.fullScreenTriangle,
          fragment,
        })
      : null;
  typegpuPipeline?.initSync();
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++) {
    if (typegpuPipeline && root) {
      const view = context.getCurrentTexture().createView();
      const encoder = root["~unstable"].createCommandEncoder({
        label: "one-renderer-typegpu-frame",
      });
      typegpuPipeline.with(encoder).withColorAttachment({ view }).draw(3);
      info.device.queue.submit([root.unwrap(encoder).finish()]);
    } else if (pipeline) {
      encodeRaw(info.device, context, pipeline);
    }
  }
  const encodeMs = performance.now() - encodeStart;
  const submitStart = performance.now();
  await info.device.queue.onSubmittedWorkDone();
  const submitMs = performance.now() - submitStart;
  root?.destroy();
  return { mode, setupMs, encodeMs, submitMs, frames: FRAMES };
}

export async function rustRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const renderer = await Renderer.new(canvas);
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++) renderer.render();
  const encodeMs = performance.now() - encodeStart;
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const submitMs = performance.now() - encodeStart - encodeMs;
  renderer.free();
  return { mode: "rust-wgpu", setupMs, encodeMs, submitMs, frames: FRAMES };
}

export async function mixedRun(
  rustCanvas: HTMLCanvasElement,
  overlayCanvas: HTMLCanvasElement,
): Promise<Result> {
  const started = performance.now();
  rustCanvas.width = WIDTH;
  rustCanvas.height = HEIGHT;
  const renderer = await Renderer.new(rustCanvas);
  const info = await requestGpuDevice();
  const context = canvasOf(overlayCanvas, info.device, info.format, "premultiplied");
  const root = tgpu.initFromDevice({ device: info.device });
  const fragment = tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })`{
    if (in.uv.x > 0.02 && in.uv.x < 0.07 && in.uv.y > 0.02 && in.uv.y < 0.10) {
      return vec4f(0.0, 0.35, 0.5, 0.45);
    }
    return vec4f(0.0, 0.0, 0.0, 0.0);
  }`;
  const pipeline = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment,
  });
  pipeline.initSync();
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++) {
    renderer.render();
    const view = context.getCurrentTexture().createView();
    const encoder = root["~unstable"].createCommandEncoder({
      label: "one-renderer-mixed-overlay",
    });
    pipeline.with(encoder).withColorAttachment({ view }).draw(3);
    info.device.queue.submit([root.unwrap(encoder).finish()]);
  }
  const encodeMs = performance.now() - encodeStart;
  await info.device.queue.onSubmittedWorkDone();
  const submitMs = performance.now() - encodeStart - encodeMs;
  root.destroy();
  renderer.free();
  return { mode: "mixed", setupMs, encodeMs, submitMs, frames: FRAMES };
}

export function formatMs(value: number): string {
  return `${value.toFixed(1)} ms`;
}

export function averageResults(mode: Mode, samples: readonly Result[]): Result {
  const failure = samples.find((sample) => sample.error);
  return {
    mode,
    setupMs: samples.reduce((sum, sample) => sum + sample.setupMs, 0) / samples.length,
    encodeMs: samples.reduce((sum, sample) => sum + sample.encodeMs, 0) / samples.length,
    submitMs: samples.reduce((sum, sample) => sum + sample.submitMs, 0) / samples.length,
    frames: Math.min(...samples.map((sample) => sample.frames)),
    error: failure?.error,
    workload: samples.find((sample) => sample.workload)?.workload,
    samples: [...samples],
  };
}

export type RunCanvases = {
  typegpu: RefObject<HTMLCanvasElement | null>;
  exported: RefObject<HTMLCanvasElement | null>;
  rust: RefObject<HTMLCanvasElement | null>;
  mixedRust: RefObject<HTMLCanvasElement | null>;
  mixedOverlay: RefObject<HTMLCanvasElement | null>;
  realBrowser: RefObject<HTMLCanvasElement | null>;
  realRust: RefObject<HTMLCanvasElement | null>;
  modelBrowser: RefObject<HTMLCanvasElement | null>;
  modelRust: RefObject<HTMLCanvasElement | null>;
  battleBrowser: RefObject<HTMLCanvasElement | null>;
  battleRust: RefObject<HTMLCanvasElement | null>;
  sharedCore: RefObject<HTMLCanvasElement | null>;
};
export async function runMode(mode: Mode, canvases: RunCanvases): Promise<Result> {
  switch (mode) {
    case "typegpu":
      return browserRun("typegpu", canvases.typegpu.current!);
    case "exported-wgsl":
      return browserRun("exported-wgsl", canvases.exported.current!);
    case "rust-wgpu":
      return rustRun(canvases.rust.current!);
    case "mixed":
      return mixedRun(canvases.mixedRust.current!, canvases.mixedOverlay.current!);
    case "real-browser":
      return realBrowserRun(canvases.realBrowser.current!);
    case "real-rust-wgpu":
      return realRustRun(canvases.realRust.current!);
    case "model-browser":
      return modelBrowserRun(canvases.modelBrowser.current!);
    case "model-rust-wgpu":
      return modelRustRun(canvases.modelRust.current!);
    case "battle-rust-wgpu":
      return battleRustRun(canvases.battleRust.current!);
    case "shared-rust-core-browser":
      return sharedCoreBrowserRun(canvases.sharedCore.current!);
    case "battle-browser":
      throw new Error("battle-browser is owned by the battle fixture adapter");
  }
}
