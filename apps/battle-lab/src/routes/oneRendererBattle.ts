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
import {
  decodePresentationCapture,
  type PresentationFrame,
} from "@web/battle/benchmark/presentationCapture";
import { gameFogGeometry, gameFogStyle } from "../gameFog";
import { gameBuildingStyle, gameGlass, gameModelDetail } from "../gameModels";
import { gameLight } from "../gameLight";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "../gameOverlay";
import { readBundle, readGzip } from "@packages/scene-assets/src/gzip";
import { bundlePath } from "@packages/scene-assets/src/schema";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { FOUNDATION, FOUNDATION_CAMERA, groundPatch } from "./frame";
import { catalogSet } from "@web/battle/catalog/sets";
import { loadMap } from "@web/maps/browser";
import { loadSimModule, loadWasm } from "@web/battle/sim/module";
import { fieldedBattle } from "../presentedFrame";
import type { RuntimeFiles } from "../presentedFrame";
import {
  gameAppearances,
  gameRuntime,
  mapAppearances,
  mapDownloads,
  reloadGameAppearances,
} from "../gameAppearances";
import { gameBiome } from "../gameBiome";
import {
  buildWorldLayers,
  readWorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { mapProps } from "@packages/battle-renderer/src/models/propAppearance";
import { indexBuildings } from "@packages/battle-renderer/src/models/buildingReferences";
import parisCapture from "@fixtures/parity/capture/run/paris-corner/capture.json";
import { measureFrames } from "./oneRendererTiming";

export const BATTLE_WIDTH = 640;
export const BATTLE_HEIGHT = 360;

export interface BattleFixture {
  terrain: Float32Array;
  camera: ReturnType<ScenePresentation["frame"]>;
  instances: readonly SceneInstance[];
  world: ReturnType<typeof groundPatch>;
  presentation: ScenePresentation;
  appearances?: InstalledAppearances;
  overlay: { opaque: Mesh; translucent: Mesh; painted: Mesh; paintedMarching: Mesh };
  changing?: {
    next(generation: number): BattleFixture["camera"];
    result(): {
      inputs: {
        fingerprint: string;
        seed: number;
        first: PresentationFrame;
        last: PresentationFrame;
      };
      frames: number;
      publications: number;
      state: ReturnType<ScenePresentation["probe"]>;
      records: { camera: number[]; view: number[]; shadow: number[]; casters: number[] };
    };
  };
}

/**
 * Build the same battle records that the live browser session consumes.
 *
 * The old fixture below is intentionally synchronous and remains useful for
 * the small pass-graph controls. This async variant is the parity gate: the
 * test map and catalog go through the simulation WASM, one publication is
 * encoded with its real observation layout, and `fieldedBattle` resolves the
 * production unit/effect sections and clip tables from the runtime catalog.
 */
export async function realBattleFixture(): Promise<BattleFixture> {
  const [catalog, map] = await Promise.all([catalogSet("test"), loadMap("ground")]);
  const runtimeCatalog = (await (await fetch("/catalog.json")).json()) as RuntimeCatalog;
  // Fielding consumes the bundle records, not their texture pixels. Read only
  // those bundle files here; this keeps the real presentation fixture usable in
  // a source checkout where optional texture payloads may still be LFS pointers.
  const runtime: RuntimeFiles = {
    catalog: runtimeCatalog,
    contents: (hashes) =>
      Promise.all(
        hashes.map((hash) =>
          readGzip(
            runtimeCatalog,
            "bundle",
            hash,
            bundlePath,
            async (path) => new Uint8Array(await (await fetch(`/${path}`)).arrayBuffer()),
          ),
        ),
      ),
  };
  const rules = catalog.rules;
  const scenario = JSON.stringify({
    map: map.definition,
    rules,
    units: [
      // Keep the publication's units inside the frozen lab camera so the
      // comparison exercises authored model pixels instead of an off-screen
      // transport record.
      { side: "blue", kind: "test_tank", position: [0, 0], yaw: 0.25 },
      { side: "red", kind: "test_tank", position: [10, 4], yaw: -2.4 },
    ],
    events: [],
    scripts: [],
  });
  const sim = await loadSimModule();
  const battle = sim.createBattle(scenario, 7);
  const ack = JSON.parse(
    battle.accept(
      JSON.stringify({
        side: "blue",
        seq: 1,
        order: { kind: "attack", units: [0], target: { kind: "ground", point: [10, 4, 0] } },
        queued: false,
      }),
    ),
  ) as { error?: string | null; refused?: unknown[] };
  if (ack.error) throw new Error(`one-renderer attack fixture rejected: ${ack.error}`);
  if (ack.refused?.length)
    throw new Error(`one-renderer attack fixture refused: ${JSON.stringify(ack)}`);
  const presentation = new ScenePresentation(battle.observation_layout(), rules.tick_hz);
  battle.resync_observation();
  for (let i = 0; i < 60; i++) battle.step();
  const length = battle.publish("blue");
  const packed = new Uint32Array(length);
  packed.set(new Uint32Array(sim.memory.buffer, battle.publication_ptr(), length));
  battle.free();
  presentation.decode(packed, 0);
  const battleInput = await fieldedBattle({
    rules: JSON.stringify(rules),
    catalog: catalog.units.view,
    kinds: ["test_tank"],
    runtime,
    side: "blue",
    tickHz: rules.tick_hz,
    sun: { azimuth: gameLight.sun_azimuth, elevation: gameLight.sun_elevation },
    soldier: rules.physics,
    xray: { own: [0.2, 0.6, 1, 0.8], selected: [1, 0.8, 0.1, 0.9], ally: [0.2, 1, 0.5, 0.8] },
    blocked: [1, 0.1, 0.1, 0.8],
  });
  presentation.setBattle(battleInput);
  // Reuse the lab's frozen static world and overlay workload so the real
  // publication row is compared on the same visible scene as BattleFrame.
  // Only the model/fog records below come from the simulation publication.
  const visual = battleFixture();
  visual.presentation.free();
  const world = visual.world;
  presentation.setGround(world.terrain.input, world.scenery);
  // No stand-in eye records: fog derives the blue unit's sight from the real
  // publication decoded above, exactly as the live battle presentation does.
  presentation.setFog({ sensors: rules.sensors });
  presentation.setShadow(gameLight, world.terrain.mesh);
  const axes = screenAxes(FOUNDATION_CAMERA);
  const eye = eyePosition(new Float32Array(3) as never, FOUNDATION_CAMERA);
  const camera = presentation.frame(
    20 / rules.tick_hz,
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
  return {
    terrain: visual.terrain,
    camera: structuredClone(camera),
    instances: visual.instances,
    world,
    presentation,
    overlay: visual.overlay,
  };
}

const capturedParisFixtures = new Map<string, Promise<BattleFixture>>();

/** Build one real native parity plate through the browser presentation seam.
 * The shared comparison invokes this row once per timing repeat; retain the
 * decoded publication and resident art generation across those repeats. */
export function capturedParisFixture(
  plate = new URLSearchParams(window.location.search).get("plate") ?? "plate-660.png",
  changingFrames = 0,
): Promise<BattleFixture> {
  // A changing run consumes authority publications and must start fresh for
  // each backend/repeat. The frozen control retains its original cache.
  if (changingFrames) return buildCapturedParisFixture(plate, changingFrames);
  let fixture = capturedParisFixtures.get(plate);
  if (!fixture) {
    fixture = buildCapturedParisFixture(plate);
    capturedParisFixtures.set(plate, fixture);
  }
  return fixture;
}

async function buildCapturedParisFixture(
  plate: string,
  changingFrames = 0,
): Promise<BattleFixture> {
  const capture = decodePresentationCapture(JSON.stringify(parisCapture));
  const frame = capture.frames.find((row) => row.plate === plate);
  if (!frame) throw new Error(`paris-corner capture has no ${plate} frame`);
  if (changingFrames > frame.index)
    throw new Error(`${plate} has fewer than ${changingFrames} preceding captured frames`);
  const sequence = capture.frames.slice(frame.index - changingFrames, frame.index + 1);
  const initial = sequence[0];
  const sample = [...capture.samples].reverse().find((row) => row.tick <= frame.tick);
  if (!sample) throw new Error(`paris-corner capture has no publication for tick ${frame.tick}`);
  const [catalog, map, wasm] = await Promise.all([
    catalogSet("capture"),
    loadMap("paris-corner"),
    loadWasm(),
  ]);
  const rules = catalog.rules;
  let presentation = new ScenePresentation(capture.layout, capture.tickHz);
  let observation = null;
  const observedKinds = new Set<string>();
  for (const row of capture.samples) {
    if (row.tick > frame.tick) break;
    observation = presentation.decode(
      Uint32Array.from(row.publication),
      (row.tick * 1000) / capture.tickHz,
    );
    if (changingFrames && observation)
      for (const unit of [...observation.own, ...observation.identified])
        observedKinds.add(unit.kind);
  }
  if (!observation) throw new Error(`capture publication at tick ${sample.tick} was stale`);
  const kinds = changingFrames
    ? [...observedKinds]
    : [...new Set([...observation.own, ...observation.identified].map((unit) => unit.kind))];
  if (changingFrames) {
    presentation.free();
    presentation = new ScenePresentation(capture.layout, capture.tickHz);
    for (const row of capture.samples) {
      if (row.tick > initial.tick) break;
      presentation.decode(Uint32Array.from(row.publication), (row.tick * 1000) / capture.tickHz);
    }
  }
  const worldView = new wasm.WorldView(JSON.stringify(map.definition), JSON.stringify(rules));
  const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules))) as WorldLayout;
  const exports = readWorldExports(worldView);
  worldView.free();
  const buildings = indexBuildings(exports.buildings, mapProps(exports, layout));
  // `kinds` are simulation type ids, while the loader's second argument is
  // reserved for map-specific appearance names (kits and regional looks).
  // Resolve the unit ids to the art they wear explicitly; passing the ids as
  // appearance requests happens to work for `test_tank` because the fixture
  // has an appearance with that name, but fails for capture-only ids.
  const wearing = catalog.units.appearancesFor(kinds);
  const installed = await reloadGameAppearances(catalog.units, wearing, null);
  const complete = await gameAppearances(
    catalog.units,
    mapDownloads(installed, buildings.placed, exports.buildings.regionalFamily, true),
    wearing,
    null,
  );
  const modelAppearances = mapAppearances(
    complete,
    { exports, layout },
    buildings.placed,
    true,
    true,
  );
  // The scenery and grass passes need the complete generation. The model
  // layer receives the filtered view above, matching the live battle route.
  const layers = buildWorldLayers(exports, layout, gameBiome, "surface", complete, true);
  const battle = await fieldedBattle({
    rules: JSON.stringify(rules),
    catalog: catalog.units.view,
    kinds,
    runtime: gameRuntime,
    side: capture.side,
    tickHz: capture.tickHz,
    sun: { azimuth: gameLight.sun_azimuth, elevation: gameLight.sun_elevation },
    soldier: rules.physics,
    xray: { own: [0.2, 0.6, 1, 0.8], selected: [1, 0.8, 0.1, 0.9], ally: [0.2, 1, 0.5, 0.8] },
    blocked: [1, 0.1, 0.1, 0.8],
  });
  presentation.setBattle(battle);
  presentation.setBuildings(
    { placed: buildings.placed, partBuilding: buildings.partBuilding, shells: new Set() },
    modelAppearances,
    gameBuildingStyle,
    {
      azimuth: gameLight.sun_azimuth,
      elevation: gameLight.sun_elevation,
      farM: gameLight.cascades.max_far_m,
    },
  );
  presentation.setGround(layers.terrain.input, layers.scenery);
  presentation.setFog({ sensors: rules.sensors });
  presentation.setShadow(gameLight, layers.terrain.mesh);
  const draw = (record: PresentationFrame, generation: number) =>
    presentation.frame(
      record.simTime,
      {
        eye: record.eye,
        target: record.target,
        up: record.up,
        fovY: 0.8,
        near: 1,
        width: 640,
        height: 360,
      },
      { selected: [], xray: false, ghosts: [] },
      generation,
    );
  const camera = draw(initial, 0);
  let drawn = 0;
  let publications = 0;
  let sampleIndex = capture.samples.findIndex((row) => row.tick > initial.tick);
  if (sampleIndex < 0) sampleIndex = capture.samples.length;
  const changing = changingFrames
    ? {
        next(generation: number) {
          const record = sequence[++drawn];
          if (!record) throw new Error("changing capture sequence exhausted");
          while (
            sampleIndex < capture.samples.length &&
            capture.samples[sampleIndex].tick <= record.tick
          ) {
            const row = capture.samples[sampleIndex++];
            presentation.decode(
              Uint32Array.from(row.publication),
              (row.tick * 1000) / capture.tickHz,
            );
            publications++;
          }
          return draw(record, generation);
        },
        result() {
          const state = presentation.probe();
          const records = presentation.lastFrame();
          return {
            inputs: {
              fingerprint: capture.workload.fingerprint,
              seed: capture.workload.seed,
              first: initial,
              last: frame,
            },
            frames: drawn,
            publications,
            state,
            records: {
              camera: Array.from(records.camera.f32),
              view: Array.from(records.view.f32),
              shadow: Array.from(records.sun_shadow.f32),
              casters: Array.from(records.casters.f32),
            },
          };
        },
      }
    : undefined;
  return {
    terrain: layers.terrain.mesh,
    appearances: modelAppearances,
    camera: structuredClone(camera),
    instances: [],
    world: layers,
    presentation,
    changing,
    overlay: {
      opaque: new MeshBuilder().build(),
      translucent: new MeshBuilder().build(),
      painted: new MeshBuilder().build(),
      paintedMarching: new MeshBuilder().build(),
    },
  };
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
  // Keep the rendered mesh frozen while giving presentation/fog a real
  // non-flat page payload. This isolates the height-page transport gate from
  // terrain-mesh generation and makes recovery replay observable.
  world.terrain.input.exports.terrain = {
    nx: 2,
    ny: 2,
    spacing: 2,
    pageSize: 2,
    minHeight: 0,
    pageIds: new Uint32Array([0]),
    heights: new Float32Array([0, 0.35, 0.2, 0.8]),
  };
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
  canvas.dataset.stage = "TypeGPU device setup";
  await loadPresentation();
  const started = performance.now();
  canvas.width = BATTLE_WIDTH;
  canvas.height = BATTLE_HEIGHT;
  const context = canvas.getContext("webgpu");
  if (!context) throw new Error("Canvas refused a WebGPU context.");
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
  if (!adapter) throw new Error("WebGPU adapter unavailable for BattleFrame control.");
  const timing = new URLSearchParams(window.location.search).get("timing") === "1";
  const device = await adapter.requestDevice({
    requiredFeatures: timing && adapter.features.has("timestamp-query") ? ["timestamp-query"] : [],
  });
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: "opaque" });
  const capture = new URLSearchParams(window.location.search).get("capture") === "1";
  const dynamic = capture && new URLSearchParams(window.location.search).get("dynamic") === "1";
  const fixture = capture
    ? await capturedParisFixture(undefined, dynamic ? frames : 0)
    : battleFixture();
  canvas.dataset.stage = "TypeGPU frame construction";
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
  // Captured frames use the exact appearance generation that built their
  // presentation rows; the small control keeps its authored vehicle fixture.
  const installed = fixture.appearances ?? (await loadSpikeAppearances());
  canvas.dataset.stage = "TypeGPU appearance admission";
  await frame.setAppearances(installed);
  frame.setPresentationWorld(fixture.presentation.appearanceRows());
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
  if (!capture)
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
  frame.setFrame(fixture.camera, { models: true, effects: true });
  frame.setCamera(fixture.camera);
  frame.setStaticFrame(fixture.camera, {
    buildings: fixture.presentation.buildingWalk(),
    scenery: fixture.presentation.sceneryWalk(),
  });
  const setupMs = performance.now() - started;
  canvas.dataset.stage = "TypeGPU warm frame";
  frame.render(context.getCurrentTexture().createView());
  await device.queue.onSubmittedWorkDone();
  frame.resetGpuTiming();
  let encodedFrames = 0;
  const measured = await measureFrames(
    frames,
    () => {
      canvas.dataset.stage = `TypeGPU frame ${++encodedFrames}`;
      if (fixture.changing) {
        const current = fixture.changing.next(frame.staticGeneration);
        frame.setFrame(current, { models: true, effects: true });
        frame.setCamera(current);
        const buildings = fixture.presentation.buildingWalk();
        const scenery = fixture.presentation.sceneryWalk();
        frame.setStaticFrame(fixture.presentation.lastFrame(), { buildings, scenery });
      }
      frame.render(context.getCurrentTexture().createView());
    },
    () => device.queue.onSubmittedWorkDone(),
  );
  const stats = frame.stats();
  const changing = fixture.changing?.result();
  if (capture) canvas.dataset.finalFrame = canvas.toDataURL();
  canvas.dataset.stage = "TypeGPU complete";
  frame.dispose();
  if (!capture || dynamic) fixture.presentation.free();
  device.destroy();
  return {
    setupMs,
    encodeMs: measured.encodeMs,
    submitMs: measured.submitMs,
    frames,
    workload: JSON.stringify({
      passes: ["shadows", "depth", "world", "effects", "fog", "post", "overlays"],
      models: stats.models.instances,
      worldVertices: stats.worldVertices,
      effects: stats.effects.instances,
      shadowCascades: stats.shadow.cascades,
      fogEyes: stats.fog.eyes,
      allocations: stats.memory,
      timing: measured.timing,
      gpu: stats.gpu,
      ...(changing ? { changing } : {}),
    }),
  };
}
