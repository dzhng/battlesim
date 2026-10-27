import { Renderer } from "@wasm/renderer_spike/renderer_wasm_spike.js";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import { loadPresentation, resolveFogLook, resolveLight } from "@web/battle/sim/presentation";
import { gameFogStyle } from "../gameFog";
import { gameLight } from "../gameLight";
import { battleFixture } from "./oneRendererBattle";
import { loadTankBundle, mw } from "./oneRendererAssets";
import type { Result } from "./oneRendererRuns";

const FRAMES = 120;
const BATTLE_WIDTH = 640;
const BATTLE_HEIGHT = 360;

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
  const workload = mw(await loadTankBundle());
  await loadPresentation();
  const rl = await resolveLight(gameLight);
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
  const environment = new Float32Array(rl.sun);
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
  const pw = production.production_stats();
  fixture.presentation.free();
  renderer.free();
  return {
    mode: "battle-rust-wgpu",
    setupMs,
    encodeMs,
    submitMs,
    frames: FRAMES,
    workload: pw,
  };
}
