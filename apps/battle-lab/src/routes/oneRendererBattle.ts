import { createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import {
  MeshBuilder,
  groundAnnulus,
  groundStrip,
  type Mesh,
} from "@packages/battle-renderer/src/mesh";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import { posedPalette } from "@packages/battle-renderer/src/models/modelInstances";
import { eyePosition, screenAxes } from "@packages/renderer-core/src/camera3d";
import {
  loadPresentation,
  resolveFogLook,
  resolveLight,
  ScenePresentation,
} from "@web/battle/sim/presentation";
import { gameFogGeometry, gameFogStyle } from "../gameFog";
import { gameGlass, gameModelDetail } from "../gameModels";
import { gameLight } from "../gameLight";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "../gameOverlay";
import { readBundle } from "@packages/scene-assets/src/gzip";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { FOUNDATION, FOUNDATION_CAMERA, groundPatch } from "./frame";

export const BATTLE_WIDTH = 640;
export const BATTLE_HEIGHT = 360;

export interface BattleFixture {
  terrain: Float32Array;
  camera: ReturnType<ScenePresentation["frame"]>;
  instances: readonly SceneInstance[];
  world: ReturnType<typeof groundPatch>;
  presentation: ScenePresentation;
  overlay: { opaque: Mesh; translucent: Mesh; painted: Mesh; paintedMarching: Mesh };
}

/** A frozen presentation frame for the production BattleFrame constructor. */
export function battleFixture(): BattleFixture {
  const base = groundPatch();
  // Keep the terrain source identical to the existing frame lab, but add a
  // production-shaped static layer: revetments, a road strip, and a ruined
  // wall line. These are world props, so BattleFrame sends them through its
  // depth, shadow, fog and post passes instead of treating them as UI.
  const props = new MeshBuilder();
  const propColour = [0.23, 0.27, 0.25, 1] as const;
  for (const [x, y, w, h, z] of [
    [-17, 7, 7, 1.3, 0.15],
    [-8, 8, 5, 1.1, 0.15],
    [15, 5, 8, 1.2, 0.15],
    [18, -8, 4, 2.0, 0.15],
  ] as const)
    props.orientedBox(x, y, 0.12, [w, h, 1.1], z, propColour);
  for (let i = 0; i < 7; i++)
    props.box(-22 + i * 2.2, -12, 1.0, 0.8, 0.8, 1.0, [0.32, 0.28, 0.22, 1]);
  const world = { ...base, props: props.build() };
  const presentation = new ScenePresentation();
  presentation.setGround(world.terrain.input, world.scenery);
  // Two eyes produce a real seen/unseen boundary over the same static world.
  // The sensors object is the fixture's authoritative game sensor contract;
  // presentation owns the sight-light and occluder expansion.
  presentation.setFog({
    sensors: awaitableGameSensors() as object,
    eyes: [
      {
        key: "blue-0",
        position: [-8, -5, 2],
        forward: 0.3,
        shape: { front: 23, side: 16, rear: 9 },
        range: 30,
      },
      {
        key: "blue-1",
        position: [4, 1, 2],
        forward: 2.7,
        shape: { front: 18, side: 13, rear: 8 },
        range: 25,
      },
    ],
  });
  presentation.setShadow(gameLight, world.terrain.mesh);
  const axes = screenAxes(FOUNDATION_CAMERA);
  const eye = eyePosition(new Float32Array(3) as never, FOUNDATION_CAMERA);
  const camera = presentation.frame(
    0,
    {
      eye: [...eye],
      target: [...FOUNDATION_CAMERA.target],
      up: axes.up,
      fovY: FOUNDATION_CAMERA.fovY,
      near: FOUNDATION_CAMERA.near,
      width: BATTLE_WIDTH,
      height: BATTLE_HEIGHT,
    },
    { selected: [], xray: false, ghosts: [] },
    0,
  );
  const instances = FOUNDATION.filter((placed) => placed.kind === "box").map((placed) => ({
    kind: "box" as const,
    x: placed.x,
    y: placed.y,
    z: placed.z,
    yaw: placed.yaw,
    color: [1, 1, 1] as const,
  }));
  const paint = new MeshBuilder();
  const marching = new MeshBuilder();
  const overlay = new MeshBuilder();
  const translucent = new MeshBuilder();
  const z = (x: number, y: number) => 0.02 * Math.exp(-((x + 18) ** 2 + (y - 16) ** 2) / 90);
  groundAnnulus(paint, [-1, 1], 4.4, 4.9, {
    z,
    segments: 32,
    colorIn: [0.12, 0.8, 0.35, 0.8],
    colorOut: [0.05, 0.25, 0.12, 0.25],
  });
  groundAnnulus(marching, [12, -8], 3.7, 4.1, {
    z,
    segments: 24,
    dashed: true,
    colorIn: [1, 0.55, 0.08, 0.9],
  });
  groundStrip(paint, [-13, -5], [15, 4], 0.12, [0.25, 0.75, 1, 0.9], { z, dashed: true, step: 2 });
  overlay.segment([-17, 11, 1.2], [17, 11, 1.2], 0.06, [0.96, 0.72, 0.2, 0.95]);
  translucent.mark([7, 2, 3.2], 1.1, [1, 0.18, 0.06, 0.78]);
  return {
    terrain: world.terrain.mesh,
    // Presentation views are backed by its reusable WASM memory and are only
    // valid until the next call into the module. Clone the frozen record before
    // handing it to BattleFrame and the Rust comparison.
    camera: structuredClone(camera),
    instances,
    world,
    presentation,
    overlay: {
      opaque: overlay.build(),
      translucent: translucent.build(),
      painted: paint.build(),
      paintedMarching: marching.build(),
    },
  };
}

// `game.json` is intentionally imported lazily here to keep this helper's
// static fixture independent from the route's React catalog provider.
function awaitableGameSensors() {
  return {
    squad_identification_fraction: 0.2,
    squad_hidden_fraction: 0.8,
    contact_radius_factor: 3,
    contact_lifetime_s: 30,
    acquisition_grace_s: 1.5,
    foliage_full_block: 1,
    building_range_multiplier: 0.2,
    hearing_shot_m: 2000,
    sound_bucket_s: 0.5,
    fog_target_height_m: 1,
    min_sight_gap_m: 0.5,
    unaided_high_air_m: { on_foot: 600, aircraft: 600, ground_vehicle: 300 },
    radar_report: { bearing_step_deg: 5, range_step_m: 100, min_radius_m: 50 },
  };
}

export interface BattleRunResult {
  setupMs: number;
  encodeMs: number;
  submitMs: number;
  frames: number;
  workload?: string;
}

/** Read only the authored tank bundle for this spike. The full appearance
 * loader also admits optional city/scenery bundles, some of which are LFS
 * pointers in a source checkout; this bounded generation keeps the battle
 * control reproducible while still exercising the production model layer. */
async function loadSpikeAppearances(): Promise<InstalledAppearances> {
  const catalog = (await (await fetch("/catalog.json")).json()) as RuntimeCatalog;
  const entry = catalog.appearances?.test_tank;
  if (!entry) throw new Error("runtime catalog has no test_tank appearance");
  const bundle = await readBundle(
    catalog,
    entry.bundle,
    async (path) => new Uint8Array(await (await fetch(`/${path}`)).arrayBuffer()),
  );
  if (bundle.kind === "clips") throw new Error("test_tank resolved to a clip bundle");
  return {
    generation: 1,
    sides: catalog.sides,
    skeletons: new Map(),
    appearances: new Map([
      [
        "test_tank",
        {
          unit: entry.unit,
          scenery: entry.scenery ?? null,
          footprint: entry.footprint_half_m ?? null,
          mounts: entry.mounts ?? null,
          regionalFamily: entry.regional_family ?? null,
          paints: entry.paints ?? null,
          wreck: entry.wreck ?? null,
          factions: entry.factions ?? null,
          bundle,
        },
      ],
    ]),
    onRequest: new Map(),
  };
}

/** Runs the actual browser BattleFrame with its production pass graph. */
export async function fullBattleBrowserRun(
  canvas: HTMLCanvasElement,
  frames: number,
): Promise<BattleRunResult> {
  await loadPresentation();
  const started = performance.now();
  canvas.width = BATTLE_WIDTH;
  canvas.height = BATTLE_HEIGHT;
  const context = canvas.getContext("webgpu");
  if (!context) throw new Error("Canvas refused a WebGPU context.");
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
  if (!adapter) throw new Error("WebGPU adapter unavailable for BattleFrame control.");
  const device = await adapter.requestDevice();
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: "opaque" });
  const fixture = battleFixture();
  const frame = await createBattleFrame(device, format, {
    light: await resolveLight(gameLight),
    fogGeometry: gameFogGeometry,
    fogLook: resolveFogLook(gameFogStyle),
    overlayGlow: gameOverlayGlow,
    paint: gamePaint,
    xrayMinHiddenFragmentFraction: gameXrayMinHiddenFragmentFraction,
    models: gameModelDetail,
    glass: gameGlass,
    world: fixture.world,
    instances: fixture.instances,
    width: BATTLE_WIDTH,
    height: BATTLE_HEIGHT,
  });
  // Install the same runtime appearance generation used by the battle lab,
  // then feed several authored vehicles through BattleFrame's real model
  // layer. This makes the control include material upload, tier selection,
  // shadow caster selection, model fog and post rather than a proxy cube.
  const installed = await loadSpikeAppearances();
  await frame.setAppearances(installed);
  const authored = (
    appearance: string,
    x: number,
    y: number,
    yaw: number,
    tint: readonly [number, number, number],
  ): ModelInstance => {
    const entry = installed.appearances.get(appearance);
    if (!entry) throw new Error(`runtime catalog has no ${appearance} appearance`);
    return {
      appearance,
      x,
      y,
      z: 0,
      yaw,
      tint,
      tier: 1,
      pose: posedPalette(entry.bundle, null),
      xray: [0.1, 0.65, 1, 0.75],
    };
  };
  frame.setModels([
    authored("test_tank", -1, 0, 0.35, [0.58, 0.72, 1]),
    authored("test_tank", 8, 6, -0.9, [1, 0.48, 0.38]),
    authored("test_tank", -10, -8, 1.7, [0.58, 0.72, 1]),
    authored("test_tank", 12, -8, -2.1, [1, 0.48, 0.38]),
  ]);
  frame.setPresentedGround(fixture.presentation.presentedGround());
  frame.setGround(fixture.presentation.ground);
  frame.setOverlay(fixture.overlay);
  frame.setOverlayGlow(gameOverlayGlow);
  // The camera record is also a valid empty dynamic batch record. Calling
  // setFrame keeps the production model/effect ownership path exercised even
  // though this frozen fixture does not field a live simulation publication.
  frame.setFrame(fixture.camera, { models: true, effects: true });
  frame.setCamera(fixture.camera);
  const setupMs = performance.now() - started;
  const encodeStart = performance.now();
  for (let index = 0; index < frames; index++)
    frame.render(context.getCurrentTexture().createView());
  const encodeMs = performance.now() - encodeStart;
  const submitStart = performance.now();
  await device.queue.onSubmittedWorkDone();
  const submitMs = performance.now() - submitStart;
  const stats = frame.stats();
  frame.dispose();
  fixture.presentation.free();
  device.destroy();
  return {
    setupMs,
    encodeMs,
    submitMs,
    frames,
    workload: JSON.stringify({
      passes: ["shadows", "depth", "world", "effects", "fog", "post", "overlays"],
      models: stats.models.instances,
      worldVertices: stats.worldVertices,
      effects: stats.effects.instances,
      shadowCascades: stats.shadow.cascades,
      fogEyes: stats.fog.eyes,
      allocations: stats.memory,
    }),
  };
}
