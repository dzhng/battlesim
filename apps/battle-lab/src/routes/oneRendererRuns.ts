import type { RefObject } from "react";
import { common, d, tgpu } from "typegpu";
import { requestGpuDevice } from "@packages/renderer-core/src/device";
import { eyePosition, screenAxes, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { Renderer } from "@wasm/renderer_spike/renderer_wasm_spike.js";
import { Renderer as SharedRenderer } from "@wasm/renderer_browser/renderer_browser.js";
import { gzipTransport, readBundle, readGzip } from "@packages/scene-assets/src/gzip";
import { bundlePath, texturePath } from "@packages/scene-assets/src/schema";
import type { Bundle, RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { posedPalette } from "@packages/battle-renderer/src/models/modelInstances";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import {
  loadPresentation,
  resolveFogLook,
  resolveLight,
  ScenePresentation,
} from "@web/battle/sim/presentation";
import { gameFogStyle } from "../gameFog";
import { gameLight } from "../gameLight";
import { gameOverlayGlow } from "../gameOverlay";
import { FOUNDATION, groundPatch, FOUNDATION_CAMERA } from "./frame";
import { modelEncoder, modelPipeline, type ModelWorkload } from "./oneRendererModel";
import { battleFixture, BATTLE_HEIGHT, BATTLE_WIDTH } from "./oneRendererBattle";

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
  const module = device.createShaderModule({ label: "one-renderer-exported-wgsl", code: shader });
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
  const encoder = device.createCommandEncoder({ label: "one-renderer-raw-frame" });
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
      ? { format: "depth32float", depthWriteEnabled: true, depthCompare: "greater" }
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
  const encoder = device.createCommandEncoder({ label: "one-renderer-real-ground-frame" });
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

let tankBundle: Promise<Exclude<Bundle, { kind: "clips" }>> | null = null;

async function loadTankBundle(): Promise<Exclude<Bundle, { kind: "clips" }>> {
  if (!tankBundle) {
    tankBundle = (async () => {
      const catalog = (await (await fetch("/catalog.json")).json()) as RuntimeCatalog;
      const hash = catalog.appearances?.test_tank?.bundle;
      if (!hash) throw new Error("runtime catalog has no test_tank bundle");
      return (await readBundle(
        catalog,
        hash,
        async (path) => new Uint8Array(await (await fetch(`/${path}`)).arrayBuffer()),
      )) as Exclude<Bundle, { kind: "clips" }>;
    })();
  }
  return tankBundle;
}

function modelWorkload(bundle: Exclude<Bundle, { kind: "clips" }>): ModelWorkload {
  const tier = 1;
  const posed = posedPalette(bundle, null);
  const parts =
    bundle.kind === "articulated"
      ? bundle.nodes.map((node, nodeIndex) => ({
          mesh: node.tiers[Math.min(tier, node.tiers.length - 1)],
          transform:
            posed.kind === "posed"
              ? posed.palette.subarray(nodeIndex * 12, nodeIndex * 12 + 12)
              : null,
        }))
      : [
          {
            mesh:
              bundle.kind === "skinned"
                ? bundle.tiers[Math.min(tier, bundle.tiers.length - 1)]
                : bundle.states[0].tiers[Math.min(tier, bundle.states[0].tiers.length - 1)],
            transform: null,
          },
        ];
  const vertices: number[] = [];
  const indices: number[] = [];
  for (const { mesh, transform } of parts) {
    if (!mesh) continue;
    const base = vertices.length / 9;
    for (let i = 0; i < mesh.positions.length / 3; i++) {
      let x = mesh.positions[i * 3];
      let y = mesh.positions[i * 3 + 1];
      let z = mesh.positions[i * 3 + 2];
      if (transform) {
        [x, y, z] = [
          transform[0] * x + transform[1] * y + transform[2] * z + transform[3],
          transform[4] * x + transform[5] * y + transform[6] * z + transform[7],
          transform[8] * x + transform[9] * y + transform[10] * z + transform[11],
        ];
      }
      vertices.push(
        x,
        y,
        z,
        mesh.colors[i * 4] / 255,
        mesh.colors[i * 4 + 1] / 255,
        mesh.colors[i * 4 + 2] / 255,
        mesh.colors[i * 4 + 3] / 255,
        mesh.uvs[i * 2],
        mesh.uvs[i * 2 + 1],
      );
    }
    for (const index of mesh.indices) indices.push(base + index);
  }
  const modelPlacements = FOUNDATION.filter((p) => p.kind === "test_tank").concat(
    FOUNDATION.filter((p) => p.kind === "test_tank").map((p) => ({ ...p, x: p.x + 9, y: p.y + 7 })),
  );
  const instances = modelPlacements.flatMap((p, i) => [
    p.x,
    p.y,
    p.z,
    p.yaw,
    i % 2 ? 0.78 : 0.58,
    i % 2 ? 0.88 : 0.72,
    1.0,
    1.0,
  ]);
  // Use the bundle's first authored texture as a deliberately bounded material
  // proof; the production renderer's material table can add the remaining
  // channels once the full frame is ported.
  const source = bundle.textures.find((texture) => texture.levels[0]?.length >= 4);
  const textureWidth = source?.width ?? 1;
  const textureHeight = source?.height ?? 1;
  return {
    vertices: new Float32Array(vertices),
    indices: new Uint32Array(indices),
    instances: new Float32Array(instances),
    texture: source?.levels[0] ?? Uint8Array.of(255, 255, 255, 255),
    textureWidth,
    textureHeight,
  };
}

function realFrame(cameraParams: Camera3DParams = FOUNDATION_CAMERA) {
  const world = groundPatch();
  const presentation = new ScenePresentation();
  presentation.setGround(world.terrain.input, world.scenery);
  // Fit the same directional cascades that production consumes. The real
  // frame rows use this frozen fit when the shared Rust shell uploads its
  // receiver, caster cameras and batch cascade masks.
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
  return { mode: "real-rust-wgpu", setupMs, encodeMs, submitMs, frames: FRAMES };
}

export async function modelBrowserRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  const info = await requestGpuDevice();
  const context = canvasOf(canvas, info.device, info.format);
  canvas.width = 320;
  canvas.height = 180;
  context.configure({ device: info.device, format: info.format, alphaMode: "opaque" });
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  context.configure({ device: info.device, format: info.format, alphaMode: "opaque" });
  const real = realFrame(MODEL_CAMERA);
  const workload = modelWorkload(await loadTankBundle());
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
  const workload = modelWorkload(await loadTankBundle());
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
  // wgpu presents through the browser queue; allow a few compositor turns
  // before the harness captures this canvas after repeated interleaved runs.
  await new Promise<void>((resolve) => {
    let frames = 3;
    const tick = () => (frames-- > 0 ? requestAnimationFrame(tick) : resolve());
    requestAnimationFrame(tick);
  });
  const submitMs = performance.now() - encodeStart - encodeMs;
  real.presentation.free();
  renderer.free();
  return { mode: "model-rust-wgpu", setupMs, encodeMs, submitMs, frames: FRAMES };
}

export async function battleRustRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  canvas.width = BATTLE_WIDTH;
  canvas.height = BATTLE_HEIGHT;
  const renderer = await Renderer.new(canvas);
  canvas.width = 320;
  canvas.height = 180;
  renderer.resize(canvas);
  canvas.width = BATTLE_WIDTH;
  canvas.height = BATTLE_HEIGHT;
  renderer.resize(canvas);
  const fixture = battleFixture();
  const workload = modelWorkload(await loadTankBundle());
  await loadPresentation();
  const resolvedLight = await resolveLight(gameLight);
  const fogLook = resolveFogLook(gameFogStyle);
  const modelInstances = new Float32Array([
    -1, 0, 0, 0.35, 0.58, 0.72, 1, 1, 8, 6, 0, -0.9, 1, 0.48, 0.38, 1, -10, -8, 0, 1.7, 0.58, 0.72,
    1, 1, 12, -8, 0, -2.1, 1, 0.48, 0.38, 1,
  ]);
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  const production = renderer as Renderer & {
    render_production: (
      terrain: Float32Array,
      vertices: Float32Array,
      indices: Uint32Array,
      instances: Float32Array,
      effects: Float32Array,
      texture: Uint8Array,
      textureWidth: number,
      textureHeight: number,
      camera: Float32Array,
      shadowCameras: Float32Array,
      environment: Float32Array,
      fogParams: Float32Array,
    ) => number;
    production_stats: () => string;
  };
  const camera = new Float32Array(fixture.camera.camera.f32.subarray(0, 16));
  const shadowCameras = new Float32Array(64);
  const casterWords = fixture.camera.casters.f32;
  for (let cascade = 0; cascade < 4; cascade++)
    shadowCameras.set(casterWords.subarray(cascade * 40, cascade * 40 + 16), cascade * 16);
  const environment = new Float32Array(resolvedLight.sun);
  const fogParams = new Float32Array([
    fogLook[0],
    fogLook[1],
    fogLook[2],
    fogLook[3],
    fogLook[6],
    fogLook[7],
    fogLook[5],
    fogLook[4],
  ]);
  const effects = new Float32Array([
    5, 1, 2.8, 1, 1.0, 0.35, 0.05, 0.8, 10, -2, 2.4, 1, 0.3, 0.65, 1.0, 0.75,
  ]);
  const staticTerrain = concatMeshes([
    fixture.terrain as Float32Array<ArrayBuffer>,
    fixture.world.props,
  ]);
  for (let index = 0; index < FRAMES; index++)
    production.render_production(
      staticTerrain,
      workload.vertices,
      workload.indices,
      modelInstances,
      effects,
      workload.texture,
      workload.textureWidth,
      workload.textureHeight,
      camera,
      shadowCameras,
      environment,
      fogParams,
    );
  const encodeMs = performance.now() - encodeStart;
  await new Promise<void>((resolve) => {
    let frames = 3;
    const tick = () => (frames-- > 0 ? requestAnimationFrame(tick) : resolve());
    requestAnimationFrame(tick);
  });
  const submitMs = performance.now() - encodeStart - encodeMs;
  const productionWorkload = production.production_stats();
  fixture.presentation.free();
  renderer.free();
  return {
    mode: "battle-rust-wgpu",
    setupMs,
    encodeMs,
    submitMs,
    frames: FRAMES,
    workload: productionWorkload,
  };
}

export async function sharedCoreBrowserRun(canvas: HTMLCanvasElement): Promise<Result> {
  const started = performance.now();
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  const renderer = await SharedRenderer.new(canvas);
  const exportValidation = await (
    renderer as SharedRenderer & {
      validate_export(): Promise<string>;
    }
  ).validate_export();
  const catalog = (await (await fetch("/catalog.json")).json()) as RuntimeCatalog;
  const bundleHash = catalog.appearances?.test_tank?.bundle;
  if (!bundleHash) throw new Error("runtime catalog has no test_tank bundle");
  const bundleWireBytes =
    gzipTransport(catalog, bundleHash).bytes +
    (catalog.textures?.[bundleHash] ?? []).reduce(
      (bytes, id) => bytes + gzipTransport(catalog, id).bytes,
      0,
    );
  const read = async (path: string) =>
    new Uint8Array(await (await fetch(`/${path}`)).arrayBuffer());
  const bundleBytes = await readGzip(catalog, "bundle", bundleHash, bundlePath, read);
  const textureBytes = await Promise.all(
    (catalog.textures?.[bundleHash] ?? []).map((id) =>
      readGzip(catalog, "texture", id, texturePath, read),
    ),
  );
  const assets = renderer as SharedRenderer & {
    load_bundle: (name: string, bundle: Uint8Array, textures: Uint8Array[]) => string;
  };
  assets.load_bundle(
    "test_tank",
    bundleBytes,
    textureBytes.map((bytes) => bytes),
  );
  (
    renderer as SharedRenderer & { enable_model_opaque_diagnostic: () => void }
  ).enable_model_opaque_diagnostic();
  const frozenFixture = battleFixture();
  const frozen = frozenFixture.camera;
  const upload = renderer as SharedRenderer & {
    upload_presentation_frame: (
      camera: Float32Array,
      environment: Float32Array,
      fog: Float32Array,
      overlay: Float32Array,
    ) => void;
    upload_fog_sight: (
      params: Float32Array,
      eyes: Float32Array,
      foliage: Float32Array,
      occluders: Float32Array,
      rebuild: Uint32Array,
      mapWords: Uint32Array,
      tiles: Uint32Array,
      wholes: Uint32Array,
      wholeItems: Uint32Array,
      wholeUniform: Float32Array,
    ) => void;
    upload_shared_ground: (mesh: Float32Array) => void;
    upload_shared_overlay: (mesh: Float32Array) => void;
  };
  // Upload one frozen presentation record set for all backend comparisons.
  const resolvedLight = await resolveLight(gameLight);
  const fogLook = resolveFogLook(gameFogStyle);
  upload.upload_presentation_frame(
    new Float32Array(frozen.camera.f32),
    new Float32Array(resolvedLight.sun),
    new Float32Array(fogLook),
    new Float32Array([gameOverlayGlow.radius_px, gameOverlayGlow.strength, 0, 0]),
  );
  upload.upload_shared_ground(concatMeshes([frozenFixture.terrain, frozenFixture.world.props]));
  upload.upload_shared_overlay(
    concatMeshes([
      frozenFixture.overlay.opaque,
      frozenFixture.overlay.translucent,
      frozenFixture.overlay.painted,
      frozenFixture.overlay.paintedMarching,
    ]),
  );
  type FogSightSnapshot = Parameters<typeof upload.upload_fog_sight>;
  let fogSightSnapshot: FogSightSnapshot | null = null;
  const uploadFogSight = (refresh: boolean) => {
    if (!fogSightSnapshot || refresh) {
      const ground = frozenFixture.presentation.presentedGround();
      const up = ground?.prepare_fog(
        new Float32Array(frozen.camera.f32),
        REAL_WIDTH,
        REAL_HEIGHT,
        0,
      );
      if (!up) return;
      const reinterpret = (words: Uint32Array) => new Float32Array(new Uint32Array(words).buffer);
      try {
        const foliage = up.foliage();
        const occluders = up.occluders();
        fogSightSnapshot = [
          reinterpret(up.params()),
          reinterpret(up.eyes()),
          foliage ? new Float32Array(foliage) : new Float32Array(0),
          occluders ? reinterpret(occluders) : new Float32Array(0),
          new Uint32Array(up.rebuild()),
          new Uint32Array(up.map_words()),
          new Uint32Array(up.tiles()),
          new Uint32Array(up.wholes() ?? []),
          new Uint32Array(up.whole_items()),
          new Float32Array(up.whole_uniform()),
        ];
      } finally {
        up.free();
      }
    }
    upload.upload_fog_sight(...fogSightSnapshot!);
  };
  uploadFogSight(true);
  (
    renderer as SharedRenderer & {
      upload_shadow_frame: (
        receiver: Float32Array,
        casters: Float32Array,
        batches: Uint32Array,
      ) => void;
    }
  ).upload_shadow_frame(
    new Float32Array(frozen.sun_shadow.f32),
    new Float32Array(frozen.casters.f32),
    new Uint32Array(frozen.batches.u32),
  );
  canvas.width = 320;
  canvas.height = 180;
  renderer.resize(canvas);
  canvas.width = REAL_WIDTH;
  canvas.height = REAL_HEIGHT;
  renderer.resize(canvas);
  renderer.recover_surface();
  renderer.destroy_device_for_test();
  await new Promise<void>((resolve) => setTimeout(resolve, 50));
  const lostBeforeRecovery = JSON.parse(renderer.stats()) as {
    deviceLost: number;
  };
  if (lostBeforeRecovery.deviceLost < 1) {
    throw new Error(`device loss callback did not fire: ${renderer.stats()}`);
  }
  await renderer.recover_device(canvas);
  // Device recreation swaps every GPU handle, so rebind the same frozen
  // presentation records before the first post-recovery frame.
  upload.upload_presentation_frame(
    new Float32Array(frozen.camera.f32),
    new Float32Array(resolvedLight.sun),
    new Float32Array(fogLook),
    new Float32Array([gameOverlayGlow.radius_px, gameOverlayGlow.strength, 0, 0]),
  );
  upload.upload_shared_ground(concatMeshes([frozenFixture.terrain, frozenFixture.world.props]));
  upload.upload_shared_overlay(
    concatMeshes([
      frozenFixture.overlay.opaque,
      frozenFixture.overlay.translucent,
      frozenFixture.overlay.painted,
      frozenFixture.overlay.paintedMarching,
    ]),
  );
  uploadFogSight(false);
  (
    renderer as SharedRenderer & {
      upload_shadow_frame: (
        receiver: Float32Array,
        casters: Float32Array,
        batches: Uint32Array,
      ) => void;
    }
  ).upload_shadow_frame(
    new Float32Array(frozen.sun_shadow.f32),
    new Float32Array(frozen.casters.f32),
    new Uint32Array(frozen.batches.u32),
  );
  renderer.render();
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let index = 0; index < FRAMES; index++) renderer.render();
  const encodeMs = performance.now() - encodeStart;
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const submitMs = performance.now() - encodeStart - encodeMs;
  const workload = renderer
    .stats()
    .replace(/\}$/, `,"bundleWireBytes":${bundleWireBytes},"export":${exportValidation}}`);
  renderer.free();
  frozenFixture.presentation.free();
  return {
    mode: "shared-rust-core-browser",
    setupMs,
    encodeMs,
    submitMs,
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
      ? root.createRenderPipeline({ vertex: common.fullScreenTriangle, fragment })
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
  // `render` submits synchronously to wgpu's browser backend. Give the browser
  // one frame boundary so this row includes the same present pressure.
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
    // The overlay surface is premultiplied; keep its colour premultiplied so
    // the browser can composite its registration cue over the Rust base.
    if (in.uv.x > 0.02 && in.uv.x < 0.07 && in.uv.y > 0.02 && in.uv.y < 0.10) {
      return vec4f(0.0, 0.35, 0.5, 0.45);
    }
    return vec4f(0.0, 0.0, 0.0, 0.0);
  }`;
  const pipeline = root.createRenderPipeline({ vertex: common.fullScreenTriangle, fragment });
  pipeline.initSync();
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let i = 0; i < FRAMES; i++) {
    renderer.render();
    const view = context.getCurrentTexture().createView();
    const encoder = root["~unstable"].createCommandEncoder({ label: "one-renderer-mixed-overlay" });
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
