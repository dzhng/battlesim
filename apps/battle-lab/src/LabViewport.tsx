import { FrameRate, type FrameRateHandle } from "@web/battle/present/frameRate";
import { useLabLoading } from "./LabLoading";
import { useCursorAction, type CursorAction } from "@web/battle/present/gameCursor";
import { useEffect, useRef, useState } from "react";
import { gpuFailureMessage } from "@packages/renderer-core/src/device";
import { appResources } from "./appResources";
import {
  trackGpuAllocations,
  type GpuAllocationTracker,
} from "@packages/renderer-core/src/gpuAllocations";
import { vec3 } from "math";
import {
  createGpuMat4,
  createProjectedPoint,
  createWorldRay,
  projectPoint,
  screenRay,
  viewProjMatrix,
  type Camera3DParams,
  type WorldRay,
} from "@packages/renderer-core/src/camera3d";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import {
  CAMERA_KEYS,
  CameraController,
  type CameraIntent,
  type CameraPose,
  type CameraPresentation,
} from "@packages/renderer-core/src/cameraController";
import {
  createClearanceState,
  type ClearanceState,
} from "@packages/renderer-core/src/cameraClearance";
import {
  createCameraObstacles,
  type CameraObstacles,
} from "@packages/renderer-core/src/cameraObstacles";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type {
  CorpseInstance,
  ModelInstance,
} from "@packages/battle-renderer/src/models/modelInstances";
import type { FelledTree } from "@packages/battle-renderer/src/scenery/felled";
import type { SideStructures } from "@packages/battle-renderer/src/models/propAppearance";
import { trackHeldKeys } from "@web/battle/input/heldKeys";
import type { Project } from "@web/battle/present/readouts";
import { gameCamera } from "./gameCamera";
import { gameLightFor } from "./gameLight";
import { gameFogGeometry, gameFogStyle } from "./gameFog";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "./gameOverlay";
import { gameBuildingStyle, gameGlass, gameModelDetail } from "./gameModels";
import type { FogInput } from "@packages/battle-renderer/src/frame/fogInputs";
import type { FogStyle } from "@packages/battle-renderer/src/frame/fogStyle";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type {
  BattleFrame,
  FrameView,
  SceneInstance,
  WorldLayers,
  WorldMeshes,
} from "@packages/battle-renderer/src/scene";
import type { GroundMarks } from "@packages/battle-renderer/src/frame/scarTexture";
import type {
  BuildingStyle,
  SideBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { SurfaceClass } from "@packages/battle-renderer/src/models/surfaceParts";
import { createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import { PassInspector } from "./PassInspector";
import type { FeedSource } from "./feed";
import {
  createEffectBatch,
  type EffectBatch,
} from "@packages/battle-renderer/src/effects/effectFrame";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import { pickBox, proxyPickBox, type PickBox } from "@packages/battle-renderer/src/picking";

const NO_INSTANCES: readonly SceneInstance[] = [];

interface LabViewportProps {
  fixture: string;
  /** A loading cover or menu owns player input while false. */
  inputEnabled?: boolean;
  /** The static world, fed like the overlay (`useFeed`); drawn once it is set. */
  world: FeedSource<WorldLayers | null>;
  /** Knowledge-drawn props as fitted appearances (standing walls and field
   *  works, remembered rubble and wrecks), lit and fogged with the world;
   *  fed like the overlay (`SideStructures`). */
  structures?: FeedSource<SideStructures>;
  /** The map's buildings, and those the side has seen fall, fed like the
   *  overlay; omitted or null draws none. */
  buildings?: FeedSource<SideBuildings | null>;
  /** How those buildings are tiered, chunked and pooled; the fixture's when
   *  omitted. The frame takes it when it is built: a change draws at the next
   *  `rebuild`. */
  buildingStyle?: BuildingStyle;
  /** What the camera keeps clear of (the buildings the side knows stand, on
   *  the ground), fed like the overlay; the ground alone when omitted or null. */
  obstacles?: FeedSource<CameraObstacles | null>;
  /** Display-space marks drawn over the finished frame, fed through one
   *  stable object (`useFeed`): never a prop that changes with them. */
  overlay?: FeedSource<WorldMeshes | undefined>;
  /** Ground paint that follows the pointer (the range ruler), fed like the
   *  overlay and uploaded apart from it (`BattleFrame.setPointerMarks`). */
  pointerMarks?: FeedSource<Mesh>;
  /** What the observing side sees from, over the static world, fed like the
   *  overlay; omitted or null draws no fog. */
  fog?: FeedSource<FogInput | null>;
  /** How unseen looks; the fixture's selected style when omitted. Live. */
  fogStyle?: FogStyle;
  /** The light, fixed for the viewport's life; the fixture's when omitted. */
  light?: LightPresentation;
  /** The proxy instances (none by default: a battle draws posed models). */
  instances?: readonly SceneInstance[];
  initialCamera: Camera3DParams;
  /** Ground height under a world point: the camera target rides it. */
  groundAt?: (x: number, y: number) => number;
  /** Left/right click: the picked instance index (−1 for none) and the camera ray. */
  onPick?: (pick: LabPick) => void;
  /** Treat a left drag as a placement gesture instead of box selection. */
  leftDragAction?: boolean;
  /** Capture the semantic target once; null cancels a press. */
  onRightPress?: (pick: LabPick | null) => void;
  /** Per-frame action for the app cursor; null shows the plain arrow. */
  onCursor?: (
    pointer: ViewportPointer,
    camera: Camera3DParams,
    project: Project,
  ) => CursorAction | null;
  /** Left-drag rectangle in page CSS pixels, with the projection to test against it. */
  onBox?: (box: LabBox) => void;
  /** Called every animation frame; what it returns redraws the frame with it
   *  (presentation interpolation between completed ticks). */
  frame?: (now: number) => ViewportFrame | null;
  /** The first frame is on screen (the loading cover can lift), with the
   *  device's live GPU allocation counts. */
  onReady?: (gpu: ViewportGpu) => void;
  /** Called every animation frame with the live world → page projection and
   *  camera, for DOM readouts anchored to world points and panned sound, and
   *  a shared pointer snapshot: position, camera ray (null over UI), and the
   *  captured world ray of a held right press. */
  onFrame?: (project: Project, camera: Camera3DParams, pointer: ViewportPointer) => void;
  /** Route-specific diagnostics published on `window.__lab.route`. */
  diagnostics?: Record<string, unknown>;
  /** A scripted driver (the benchmark, a lab's trajectory), fixed for the
   *  viewport's life: while it gives a framing it owns the camera, input no
   *  longer steers it, and every frame is drawn and reported with its cost. */
  pilot?: ViewportPilot;
  /** Appearance bundles from `AppearanceLibrary` for the models layer. */
  appearances?: InstalledAppearances | null;
  /** Posed models to draw. */
  models?: readonly ModelInstance[];
  /** What a click can pick when no frame hands picks over (the units'
   *  simulation boxes); each drawn proxy by its own box without it. */
  picks?: readonly PickBox[];
  /** The camera rig's numbers; the game's by default. */
  cameraConfig?: CameraPresentation;
}

/** One animation frame's drawn units. Anything omitted keeps its last value. */
export interface ViewportFrame {
  /** Proxies to draw (lab markers). */
  instances?: readonly SceneInstance[];
  /** What a click can pick, as boxes (every drawn soldier and vehicle, by
   *  the simulation's bodies); the drawn proxies when omitted. Picks report
   *  an index into this list. */
  picks?: readonly PickBox[];
  /** Posed models (soldiers and vehicles). */
  models?: readonly ModelInstance[];
  /** The corpses: handed to the frame only when the array changes. */
  corpses?: readonly CorpseInstance[];
  /** The trees the side knows have fallen: handed over only when the array
   *  changes. */
  felled?: readonly FelledTree[];
  /** The presentation clock, in seconds (the pose driver's and the wind's). */
  clock?: number;
  /** Combat effects at that clock (`EffectFrame.build`). */
  effects?: EffectBatch;
  /** The observing side's learned ground (the client's `GroundView`), drawn
   *  as scars; the frame uploads only what changed since it last looked. */
  ground?: GroundMarks | null;
}

/** A script's hold on the viewport's camera. */
export interface ViewportPilot {
  /** The battle frame is up: the adapter, and the frame's own statistics
   *  (its rolling GPU frame time from `timestamp-query`, and live memory). */
  attach?(frame: { adapter: string; stats: () => ReturnType<BattleFrame["stats"]> }): void;
  /** The framing for the frame at `now`; the rig places it
   *  (`CameraController.place`) and draws it clear of obstacles (`resolve`).
   *  Null leaves the camera to the player for that frame. */
  pose(now: number): CameraPose | null;
  /** A drawn frame: its main-thread cost and the camera drawn. */
  frame?(f: { now: number; cpuMs: number; camera: Camera3DParams }): void;
}

/** What the viewport's device reports once it is up. */
export interface ViewportGpu {
  allocations: GpuAllocationTracker;
  /** The device and canvas format, for work beside the viewport (model sheets). */
  device: GPUDevice;
  format: GPUTextureFormat;
  /** The live battle frame (it is rebuilt by `rebuild`). */
  frame: () => BattleFrame;
}

const _projector_world = vec3.create();
const _projector_point = createProjectedPoint();

/** One snapshot shared by hover, the ruler and the held facing preview. */
export interface ViewportPointer {
  position: { x: number; y: number } | null;
  ray: WorldRay | null;
  rightPress: WorldRay | null;
  /** Where a held left drag began, while it is an action (`leftDragAction`). */
  leftPress: WorldRay | null;
  ctrl: boolean;
  shift: boolean;
  cameraDragging: boolean;
  rightDragging: boolean;
}

export interface LabPick {
  instance: number;
  ray: WorldRay;
  button: "left" | "right";
  shift: boolean;
  ctrl: boolean;
  /** CSS pixel position and event time, for gesture recognition. */
  x: number;
  y: number;
  time: number;
  /** A right-drag's release ray (Q9 facing): the press is the pick, the
   *  release the point the units face. Absent for a plain right-click. */
  release?: WorldRay;
}

export interface LabBox {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  shift: boolean;
  project: (x: number, y: number, z: number) => [number, number] | null;
}

/** What a frame draws while effects are suppressed. */
const NO_EFFECTS: EffectBatch = createEffectBatch(0);

/** Pixels a left press may travel and still count as a click. */
const CLICK_SLOP_PX = 5;
/** Screen-edge band that pans the camera like a held key. */
const EDGE_PAN_PX = 14;

/** What holds a drawn pose off the pose asked for, and how it got there. */
type ClearanceReading = Pick<ClearanceState, "hold" | "cut" | "blocked">;

/** Diagnostic hooks the scene harness reads; lab-only, never on a player route. */
interface LabHandle {
  ready: boolean;
  fixture: string;
  error: string | null;
  adapter?: { vendor: string; architecture: string; description: string; format: string };
  stats?: () => ReturnType<BattleFrame["stats"]>;
  allocations?: GpuAllocationTracker;
  /** The camera drawn. */
  camera?: () => Camera3DParams;
  /** The harness's raw framing: drawn exactly as given, through neither the
   *  rig's limits nor camera clearance, so a check can frame its subject from
   *  anywhere. No playable or benchmark path uses it; the next input or
   *  scripted placement returns the camera to the rig. */
  setCamera?: (camera: Camera3DParams) => void;
  /** A scripted framing, as the rig places it and the next frame clears it
   *  (the benchmark tour's path): `camera()` is then the pose drawn for it. */
  placeCamera?: (pose: CameraPose) => void;
  /** Where camera clearance stands: the pose asked for, what holds the drawn
   *  pose off it, and whether the last step was a cut or found nothing clear. */
  clearance?: () => ClearanceReading & { asked: Camera3DParams; settled: boolean };
  /** Fly a scratch camera through `poses`, `dt` seconds apart, by the rig
   *  and the obstacles the viewport uses, drawing nothing: each pose drawn,
   *  and what the resolver cost (wall time and box tests, over `reps` flights). */
  flyClearance?: (
    poses: readonly CameraPose[],
    dt: number,
    reps?: number,
  ) => {
    frames: (ClearanceReading & { camera: Camera3DParams })[];
    msPerFrame: number;
    boxTestsPerFrame: number;
  };
  reset?: () => void;
  /** Rebuild the scene from scratch (reset/dispose cycle) and resolve when drawn. */
  rebuild?: () => Promise<void>;
  pickAt?: (cssX: number, cssY: number) => number;
  rayAt?: (cssX: number, cssY: number) => WorldRay;
  route?: Record<string, unknown>;
  /** What a click can pick: every drawn unit and soldier, as boxes. */
  instances?: () => readonly PickBox[];
  /** World point → CSS pixel in the page, or null when behind the eye. */
  projectToCss?: (x: number, y: number, z: number) => [number, number] | null;
  frame?: () => Promise<void>;
  /** The pass inspector's view of the frame; resolves once it is drawn. */
  setFrameView?: (view: FrameView) => Promise<void>;
  /** The sight lights' lab probes. */
  fog?: () => BattleFrame["fogProbes"];
  /** Draw without fog while on (a paired cost measure); the route's fog
   *  returns when it goes off. */
  suppressFog?: (on: boolean) => Promise<void>;
  /** The grass field's lab probes. */
  grass?: () => BattleFrame["grassProbes"];
  /** Draw without grass while on (a paired cost measure). */
  suppressGrass?: (on: boolean) => Promise<void>;
  /** Draw the plots without their own texture while on: their plain rows
   *  alone (paired frames, and a paired cost measure). */
  suppressFieldTexture?: (on: boolean) => Promise<void>;
  /** Draw no models or corpses while on (a paired cost measure). */
  suppressModels?: (on: boolean) => Promise<void>;
  /** Draw no trees and none of their shadows while on (they are scenery, not
   *  models): the ground under a wood, and a paired cost measure. */
  suppressTrees?: (on: boolean) => Promise<void>;
  /** Draw no forest-floor dressing while on (a paired cost measure). */
  suppressDressing?: (on: boolean) => Promise<void>;
  /** Draw no shrubs under tree lines while on (a paired cost measure). */
  suppressUnderstorey?: (on: boolean) => Promise<void>;
  /** Draw no template-art buildings and none of their shadows while on (a
   *  paired cost measure). */
  suppressBuildings?: (on: boolean) => Promise<void>;
  /** Draw one kind of model surface (a cutout, say), its depth and its
   *  shadow, or none of it while on (paired frames and cost). */
  suppressSurface?: (surface: SurfaceClass, on: boolean) => Promise<void>;
  /** Draw the roads plain while on, with no surface detail or shoulder (a
   *  paired cost measure). */
  suppressRoadWear?: (on: boolean) => Promise<void>;
  /** Draw no combat effects while on (a paired cost measure). */
  suppressEffects?: (on: boolean) => Promise<void>;
  /** Draw the effects but light nothing by them while on (paired frames, cost). */
  suppressCastLights?: (on: boolean) => Promise<void>;
  /** The ground marks' halo at `strength` (0 draws none: a paired cost
   *  measure), or the fixture's with null. */
  setOverlayGlowStrength?: (strength: number | null) => Promise<void>;
  /** Draw the painted ground marks or not (paired frames isolate them). */
  suppressPaint?: (on: boolean) => Promise<void>;
  /** Disable the per-model x-ray coverage gate for paired verification/cost. */
  suppressXrayCoverage?: (on: boolean) => Promise<void>;
  /** Draw the ground unmarked while on (paired frames and cost). */
  suppressScars?: (on: boolean) => Promise<void>;
  /** GPU time of one pose-kernel dispatch over the posed bodies drawn now. */
  timePoseKernel?: (
    reps: number,
    bodies?: number,
  ) => Promise<{ bodies: number; ms: number } | null>;
}

declare global {
  interface Window {
    __lab?: LabHandle;
  }
}

export function LabViewport({
  fixture,
  inputEnabled = true,
  world,
  structures,
  buildings,
  buildingStyle,
  obstacles,
  overlay,
  pointerMarks,
  fog,
  fogStyle,
  light,
  instances = NO_INSTANCES,
  initialCamera,
  groundAt,
  onPick,
  leftDragAction = false,
  onRightPress,
  onCursor,
  onBox,
  frame,
  onFrame,
  onReady,
  diagnostics,
  pilot,
  appearances,
  models,
  picks: fixedPicks,
  cameraConfig,
}: LabViewportProps) {
  const inputEnabledRef = useRef(inputEnabled);
  inputEnabledRef.current = inputEnabled;
  const leftDragActionRef = useRef(leftDragAction);
  leftDragActionRef.current = leftDragAction;
  const cancelInputRef = useRef<(() => void) | null>(null);
  useEffect(() => {
    if (!inputEnabled) cancelInputRef.current?.();
  }, [inputEnabled]);
  const pilotRef = useRef(pilot);
  const cameraConfigRef = useRef(cameraConfig);
  const appearancesRef = useRef(appearances);
  appearancesRef.current = appearances;
  const modelsRef = useRef(models);
  modelsRef.current = models;
  const corpsesRef = useRef<readonly CorpseInstance[]>([]);
  const felledRef = useRef<readonly FelledTree[]>([]);
  const modelsSuppressed = useRef(false);
  const effectsSuppressed = useRef(false);
  const scarsSuppressed = useRef(false);
  /** The side's learned ground as the last frame reported it. */
  const groundRef = useRef<GroundMarks | null>(null);
  /** The ground the battle frame draws: none while suppressed. */
  const groundNow = () => (scarsSuppressed.current ? null : groundRef.current);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const setCursorActionRef = useRef(useCursorAction());
  const frameRate = useRef<FrameRateHandle>(null);
  const onCursorRef = useRef(onCursor);
  onCursorRef.current = onCursor;
  const onRightPressRef = useRef(onRightPress);
  onRightPressRef.current = onRightPress;
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useLabLoading("renderer", ready, error);
  const [box, setBox] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const onBoxRef = useRef(onBox);
  onBoxRef.current = onBox;
  const frameRef = useRef(frame);
  frameRef.current = frame;
  const onFrameRef = useRef(onFrame);
  onFrameRef.current = onFrame;
  const instancesRef = useRef(instances);
  const picksRef = useRef<readonly PickBox[] | null>(null);
  const fixedPicksRef = useRef(fixedPicks);
  fixedPicksRef.current = fixedPicks;
  const worldRef = useRef(world);
  worldRef.current = world;
  const sceneRef = useRef<BattleFrame | null>(null);
  const [inspecting, setInspecting] = useState<BattleFrame | null>(null);
  instancesRef.current = instances;

  const redrawRef = useRef<() => void>(() => {});
  const onPickRef = useRef(onPick);
  onPickRef.current = onPick;
  const onReadyRef = useRef(onReady);
  onReadyRef.current = onReady;
  const diagnosticsRef = useRef(diagnostics);
  diagnosticsRef.current = diagnostics;
  const initialCameraRef = useRef(initialCamera);
  const groundAtRef = useRef(groundAt);
  groundAtRef.current = groundAt;
  useEffect(() => {
    if (window.__lab) window.__lab.route = diagnostics;
  }, [diagnostics]);
  useEffect(() => {
    sceneRef.current?.setInstances(instances);
    redrawRef.current();
  }, [instances]);
  useEffect(
    () =>
      world.subscribe((next) => {
        if (next) sceneRef.current?.setWorld(next);
        redrawRef.current();
      }),
    [world],
  );
  const overlayRef = useRef(overlay);
  overlayRef.current = overlay;
  const pointerMarksRef = useRef(pointerMarks);
  pointerMarksRef.current = pointerMarks;
  useEffect(
    () =>
      pointerMarks?.subscribe((marks) => {
        sceneRef.current?.setPointerMarks(marks);
        redrawRef.current();
      }),
    [pointerMarks],
  );
  const structuresRef = useRef(structures);
  structuresRef.current = structures;
  const buildingsRef = useRef(buildings);
  buildingsRef.current = buildings;
  const buildingStyleRef = useRef(buildingStyle);
  buildingStyleRef.current = buildingStyle;
  useEffect(
    () =>
      buildings?.subscribe((next) => {
        sceneRef.current?.setBuildings(next);
        redrawRef.current();
      }),
    [buildings],
  );
  const obstaclesRef = useRef(obstacles);
  obstaclesRef.current = obstacles;
  const fogRef = useRef(fog);
  fogRef.current = fog;
  const fogStyleRef = useRef(fogStyle);
  fogStyleRef.current = fogStyle;
  const lightRef = useRef(light);
  useEffect(() => {
    sceneRef.current?.setFogStyle(fogStyle ?? gameFogStyle);
    redrawRef.current();
  }, [fogStyle]);
  const fogSuppressed = useRef(false);
  useEffect(() => {
    const draw = (input: FogInput | null) => {
      sceneRef.current?.setFog(fogSuppressed.current ? null : input);
      redrawRef.current();
    };
    draw(fog?.current ?? null);
    return fog?.subscribe(draw);
  }, [fog]);
  useEffect(
    () =>
      overlay?.subscribe((meshes: WorldMeshes | undefined) => {
        if (meshes) sceneRef.current?.setOverlay(meshes);
        redrawRef.current();
      }),
    [overlay],
  );
  useEffect(() => {
    const draw = (next: SideStructures) => {
      sceneRef.current?.setStructures(next);
      redrawRef.current();
    };
    if (!structures) return;
    draw(structures.current);
    return structures.subscribe(draw);
  }, [structures]);
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    void scene.setAppearances(appearances ?? null).then(() => {
      if (modelsRef.current) scene.setModels(modelsRef.current);
      redrawRef.current();
    });
  }, [appearances]);
  useEffect(() => {
    if (models) sceneRef.current?.setModels(models);
    redrawRef.current();
  }, [models]);

  useEffect(() => {
    setReady(false);
    setError(null);
    const canvas = canvasRef.current!;
    const handle: LabHandle = {
      ready: false,
      fixture,
      error: null,
      route: diagnosticsRef.current,
    };
    const initialCamera = initialCameraRef.current;
    window.__lab = handle;
    const lifetime = new AbortController();
    const { signal } = lifetime;
    let device: GPUDevice | null = null;
    let canvasContext: GPUCanvasContext | null = null;
    let raf = 0;
    const setCursorAction = setCursorActionRef.current;
    const interactionSurface = (target: EventTarget | null) =>
      target === canvas
        ? canvas
        : target instanceof Element
          ? target.closest<HTMLElement>(".ro-layer .ro-unit, .hud")
          : null;
    // The pose input and scripts ask for, and the pose drawn for it: the
    // same one unless camera clearance holds it off an obstacle.
    let asked = initialCamera;
    let camera = initialCamera;
    /** A harness framing (`setCamera`) is drawn as given until the rig moves. */
    let framedRaw = false;
    const clearance = createClearanceState();
    // Without a ground, the target keeps its height.
    const ground = (x: number, y: number) =>
      groundAtRef.current ? groundAtRef.current(x, y) : asked.target[2];
    const controller = new CameraController(cameraConfigRef.current ?? gameCamera.config, ground);
    const openGround = createCameraObstacles([], ground);
    const obstaclesNow = () => obstaclesRef.current?.current ?? openGround;
    let resolvedOver: CameraObstacles | null = null;
    /** Draw the pose asked for, clear of obstacles, `dt` seconds on from the
     *  last call. Nothing to do once the camera rests on an unchanged ask over
     *  unchanged obstacles. */
    const see = (dt: number) => {
      if (framedRaw) return;
      const aspect = canvas.width / Math.max(1, canvas.height);
      if (asked.aspect !== aspect) asked = { ...asked, aspect };
      const over = obstaclesNow();
      if (clearance.settled && clearance.desired === asked && over === resolvedOver) return;
      resolvedOver = over;
      const next = controller.resolve(clearance, asked, dt, over);
      if (next !== camera) {
        camera = next;
        dirty = true;
      }
    };
    /** The rig takes the camera: from input, a script, or back from a raw framing. */
    const place = (next: Camera3DParams) => {
      asked = next;
      framedRaw = false;
    };
    const keys = trackHeldKeys(
      window,
      (code) =>
        inputEnabledRef.current &&
        (code in CAMERA_KEYS || code === "ShiftLeft" || code === "ShiftRight"),
    );
    const pilot = pilotRef.current;
    /** The pilot gave the last frame its framing. */
    let piloted = pilot !== undefined;
    /** Apply one intent to the pose asked for; the frame loop draws it. A
     *  piloted camera takes no input. */
    const steer = (intent: CameraIntent, dt: number) => {
      if (piloted || !inputEnabledRef.current) return;
      const next = controller.step(asked, intent, dt);
      if (next !== asked) place(next);
    };
    let dirty = true;

    const snapshot = (): ViewportCamera => ({
      camera3d: camera,
      width: canvas.width,
      height: canvas.height,
    });

    (async () => {
      try {
        const info = await appResources.gpu();
        device = info.device;
        if (signal.aborted) return;
        const allocations = trackGpuAllocations(device);
        const context = canvas.getContext("webgpu");
        if (!context) throw new Error("Canvas refused a WebGPU context.");
        canvasContext = context;
        context.configure({ device, format: info.format, alphaMode: "opaque" });
        const syncSize = () => {
          const dpr = window.devicePixelRatio || 1;
          const w = Math.max(1, Math.round(canvas.clientWidth * dpr));
          const h = Math.max(1, Math.round(canvas.clientHeight * dpr));
          if (canvas.width !== w || canvas.height !== h) {
            canvas.width = w;
            canvas.height = h;
            dirty = true;
          }
        };
        syncSize();
        let clock = 0;
        const build = async () => {
          const next = await createBattleFrame(device!, info.format, {
            // A route's own light, else the fixture's (a lab URL's `?sun=` sets it lower).
            light: lightRef.current ?? gameLightFor(window.location.search),
            fogGeometry: gameFogGeometry,
            fogStyle: fogStyleRef.current ?? gameFogStyle,
            overlayGlow: gameOverlayGlow,
            paint: gamePaint,
            xrayMinHiddenFragmentFraction: gameXrayMinHiddenFragmentFraction,
            models: gameModelDetail,
            glass: gameGlass,
            buildings: buildingStyleRef.current ?? gameBuildingStyle,
            world: worldRef.current.current!,
            instances: instancesRef.current,
            width: canvas.width,
            height: canvas.height,
            requestRedraw: () => (dirty = true),
          });
          if (signal.aborted) {
            next.dispose();
            return next;
          }
          // Prop effects and feeds can update this frame while appearances load.
          sceneRef.current = next;
          try {
            if (structuresRef.current) next.setStructures(structuresRef.current.current);
            if (buildingsRef.current) next.setBuildings(buildingsRef.current.current);
            const meshes = overlayRef.current?.current;
            if (meshes) next.setOverlay(meshes);
            const marks = pointerMarksRef.current?.current;
            if (marks) next.setPointerMarks(marks);
            next.setFog(fogRef.current?.current ?? null);
            if (appearancesRef.current) await next.setAppearances(appearancesRef.current);
            if (signal.aborted) {
              next.dispose();
              return next;
            }
            if (modelsRef.current) next.setModels(modelsRef.current);
            next.setClock(clock);
            next.setCorpses(corpsesRef.current);
            next.setFelled(felledRef.current);
            next.setGround(groundNow());
            return next;
          } catch (error) {
            if (sceneRef.current === next) sceneRef.current = null;
            next.dispose();
            throw error;
          }
        };
        let scene = await build();
        if (signal.aborted) return;
        pilot?.attach?.({
          adapter: info.description || `${info.vendor} ${info.architecture}`.trim(),
          stats: () => scene.stats(),
        });
        const draw = () => {
          syncSize();
          // Cleared first: a frame may ask to be drawn again (`requestRedraw`).
          dirty = false;
          scene.render(context.getCurrentTexture().createView(), snapshot());
        };
        redrawRef.current = () => (dirty = true);
        let pointer: { x: number; y: number } | null = null;
        let modifiers = { ctrl: false, shift: false };
        const pointerRay = () => (pointer ? handle.rayAt!(pointer.x, pointer.y) : null);
        let lastFrame = performance.now();
        const loop = (now: number) => {
          if (signal.aborted) return;
          const dt = Math.min(0.1, (now - lastFrame) / 1000);
          lastFrame = now;
          // Held camera keys, and the pointer resting at the canvas border.
          let edge: [number, number] = [0, 0];
          if (
            pointer &&
            (interactionSurface(document.elementFromPoint(pointer.x, pointer.y)) === canvas ||
              rightPress ||
              press ||
              orbit)
          ) {
            const rect = canvas.getBoundingClientRect();
            edge = [
              pointer.x - rect.left < EDGE_PAN_PX
                ? -1
                : rect.right - pointer.x < EDGE_PAN_PX
                  ? 1
                  : 0,
              pointer.y - rect.top < EDGE_PAN_PX
                ? 1
                : rect.bottom - pointer.y < EDGE_PAN_PX
                  ? -1
                  : 0,
            ];
          }
          const framing = pilot?.pose(now) ?? null;
          piloted = framing !== null;
          steer({ held: keys.held, edge }, dt);
          const started = performance.now();
          if (framing) {
            place(controller.place(asked, framing));
            dirty = true; // a piloted frame is always drawn: it is measured
          }
          see(dt);
          const animated = frameRef.current?.(now);
          if (animated?.ground !== undefined) groundRef.current = animated.ground;
          if (scene.setGround(groundNow())) dirty = true;
          if (animated) {
            if (animated.clock !== undefined && animated.clock !== clock) {
              clock = animated.clock;
              scene.setClock(clock);
            }
            if (animated.instances) {
              instancesRef.current = animated.instances;
              scene.setInstances(animated.instances);
            }
            if (animated.picks) picksRef.current = animated.picks;
            if (animated.effects && !effectsSuppressed.current) scene.setEffects(animated.effects);
            if (animated.models) {
              modelsRef.current = animated.models;
              if (!modelsSuppressed.current) scene.setModels(animated.models);
            }
            if (animated.corpses && animated.corpses !== corpsesRef.current) {
              corpsesRef.current = animated.corpses;
              if (!modelsSuppressed.current) scene.setCorpses(animated.corpses);
            }
            if (animated.felled && animated.felled !== felledRef.current) {
              felledRef.current = animated.felled;
              scene.setFelled(animated.felled);
            }
            dirty = true;
          }
          const drawn = dirty;
          if (drawn) draw();
          frameRate.current?.frame(now, drawn);
          const surface = pointer
            ? interactionSurface(document.elementFromPoint(pointer.x, pointer.y))
            : null;
          const pointerState: ViewportPointer = {
            position: surface || rightPress ? pointer : null,
            ray: pointerRay(),
            ...modifiers,
            cameraDragging: !!orbit,
            rightPress: rightPress?.ray ?? null,
            leftPress: press?.dragged && leftDragActionRef.current ? press.ray : null,
            rightDragging: !!(
              rightPress &&
              pointer &&
              Math.hypot(
                pointer.x - rightPress.event.clientX,
                pointer.y - rightPress.event.clientY,
              ) > CLICK_SLOP_PX
            ),
          };
          const project = projector();
          onFrameRef.current?.(project, camera, pointerState);
          const action = onCursorRef.current?.(pointerState, camera, project) ?? null;
          // The app cursor always shows; the battle only chooses its action.
          const active = pointerState.position && surface && !orbit && !pilot;
          setCursorAction((active && action) || "default");
          pilot?.frame?.({ now, cpuMs: performance.now() - started, camera });
          raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);

        /** World → page projection for the current camera and canvas box,
         *  read once so a whole frame of anchors costs one layout read. */
        const projector = (): Project => {
          const viewProj = viewProjMatrix(createGpuMat4(), liveCamera(snapshot()));
          const rect = canvas.getBoundingClientRect();
          return (x, y, z) => {
            vec3.set(_projector_world, x, y, z);
            const { ndc, clipW } = projectPoint(_projector_point, viewProj, _projector_world);
            if (clipW <= 0) return null;
            return [
              rect.left + (ndc[0] * 0.5 + 0.5) * rect.width,
              rect.top + (0.5 - ndc[1] * 0.5) * rect.height,
            ];
          };
        };

        /** What a click can pick: the frame's picks, else the route's, else
         *  each drawn proxy by its own box. */
        const picks = (): readonly PickBox[] =>
          picksRef.current ?? fixedPicksRef.current ?? instancesRef.current.map(proxyPickBox);
        const drawn = () =>
          new Promise<void>((resolve) => {
            dirty = true;
            requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
          });
        /** A frame drawn at the current size: a resize's targets build
         *  asynchronously, so wait for them and draw again. */
        const nextFrame = async () => {
          await drawn();
          if (await scene.settled()) await drawn();
        };

        Object.assign(handle, {
          adapter: {
            vendor: info.vendor,
            architecture: info.architecture,
            description: info.description,
            format: info.format,
          },
          stats: () => scene.stats(),
          allocations,
          camera: () => camera,
          setCamera(next: Camera3DParams) {
            asked = camera = next;
            framedRaw = true;
            Object.assign(clearance, createClearanceState());
            dirty = true;
          },
          placeCamera(pose: CameraPose) {
            place(controller.place(asked, pose));
          },
          clearance: () => ({
            asked,
            hold: clearance.hold,
            cut: clearance.cut,
            blocked: clearance.blocked,
            settled: clearance.settled,
          }),
          flyClearance(poses: readonly CameraPose[], dt: number, reps = 1) {
            const over = obstaclesNow();
            const tested = over.tested;
            const started = performance.now();
            let frames: (ClearanceReading & { camera: Camera3DParams })[] = [];
            for (let rep = 0; rep < reps; rep++) {
              const state = createClearanceState();
              let pose = asked;
              frames = poses.map((p) => {
                pose = controller.place(pose, p);
                const drawn = controller.resolve(state, pose, dt, over);
                return { camera: drawn, hold: state.hold, cut: state.cut, blocked: state.blocked };
              });
            }
            const flown = Math.max(1, poses.length * reps);
            return {
              frames,
              msPerFrame: (performance.now() - started) / flown,
              boxTestsPerFrame: (over.tested - tested) / flown,
            };
          },
          reset() {
            place(initialCamera);
            Object.assign(clearance, createClearanceState());
            see(0);
          },
          async rebuild() {
            scene.dispose();
            sceneRef.current = null;
            scene = await build();
            if (signal.aborted) return;
            setInspecting((shown) => (shown ? scene : shown));
            await nextFrame();
          },
          rayAt(cssX: number, cssY: number) {
            const rect = canvas.getBoundingClientRect();
            const ndcX = ((cssX - rect.left) / rect.width) * 2 - 1;
            const ndcY = 1 - ((cssY - rect.top) / rect.height) * 2;
            return screenRay(createWorldRay(), liveCamera(snapshot()), ndcX, ndcY);
          },
          pickAt(cssX: number, cssY: number) {
            return pickBox(handle.rayAt!(cssX, cssY), picks());
          },
          instances: () => picks(),
          projectToCss(x: number, y: number, z: number) {
            return projector()(x, y, z);
          },
          frame: nextFrame,
          async setFrameView(view: FrameView) {
            scene.setView(view);
            await nextFrame();
          },
          fog: () => scene.fogProbes,
          grass: () => scene.grassProbes,
          async suppressGrass(on: boolean) {
            scene.grassProbes.suppress(on);
            await nextFrame();
          },
          async suppressFog(on: boolean) {
            fogSuppressed.current = on;
            scene.setFog(on ? null : (fogRef.current?.current ?? null));
            await nextFrame();
          },
          async suppressScars(on: boolean) {
            scarsSuppressed.current = on;
            await nextFrame();
          },
          async suppressFieldTexture(on: boolean) {
            scene.setFieldTextureShown(!on);
            await nextFrame();
          },
          async suppressModels(on: boolean) {
            modelsSuppressed.current = on;
            scene.setModels(on ? [] : (modelsRef.current ?? []));
            scene.setCorpses(on ? [] : corpsesRef.current);
            await nextFrame();
          },
          async suppressTrees(on: boolean) {
            scene.setTreesShown(!on);
            await nextFrame();
          },
          async suppressDressing(on: boolean) {
            scene.setDressingShown(!on);
            await nextFrame();
          },
          async suppressUnderstorey(on: boolean) {
            scene.setUnderstoreyShown(!on);
            await nextFrame();
          },
          async suppressBuildings(on: boolean) {
            scene.setBuildingsShown(!on);
            await nextFrame();
          },
          async suppressSurface(surface: SurfaceClass, on: boolean) {
            scene.setSurfaceShown(surface, !on);
            await nextFrame();
          },
          async suppressRoadWear(on: boolean) {
            scene.setRoadWearShown(!on);
            await nextFrame();
          },
          async suppressEffects(on: boolean) {
            effectsSuppressed.current = on;
            if (on) scene.setEffects(NO_EFFECTS);
            await nextFrame();
          },
          async suppressCastLights(on: boolean) {
            scene.setCastLightsShown(!on);
            await nextFrame();
          },
          async suppressXrayCoverage(on: boolean) {
            scene.setXrayCoverageEnabled(!on);
            await nextFrame();
          },
          async suppressPaint(on: boolean) {
            scene.setPaintShown(!on);
            await nextFrame();
          },
          async setOverlayGlowStrength(strength: number | null) {
            scene.setOverlayGlow(
              strength === null ? gameOverlayGlow : { ...gameOverlayGlow, strength },
            );
            await nextFrame();
          },
          timePoseKernel: (reps: number, bodies?: number) => scene.timePoseKernel(reps, bodies),
        } satisfies Partial<LabHandle>);

        // Input: left click selects, left drag box-selects (or, as an action,
        // places at the press and faces toward the release), right click
        // orders (on release: a right-drag also sets the facing); the camera (CameraController) takes held WASD/arrows and the
        // screen edge to pan, Q/E to turn, middle drag to orbit, the wheel to zoom.
        let orbit: { x: number; y: number } | null = null;
        let press: {
          x: number;
          y: number;
          shift: boolean;
          ray: WorldRay;
          dragged: boolean;
        } | null = null;
        let rightPress: { event: PointerEvent; pick: LabPick; ray: WorldRay } | null = null;
        const makePick = (e: PointerEvent, button: "left" | "right") => {
          const ray = handle.rayAt!(e.clientX, e.clientY);
          return {
            instance: pickBox(ray, picks()),
            ray,
            button,
            shift: e.shiftKey,
            ctrl: e.ctrlKey,
            x: e.clientX,
            y: e.clientY,
            time: e.timeStamp,
          };
        };
        const onDown = (e: PointerEvent) => {
          if (!inputEnabledRef.current) return;
          // Let the HUD own its native click sequence. Capturing a HUD press
          // on the canvas drops the button's pointerup and prevents clicks.
          if (e.target instanceof Element && e.target.closest(".hud")) return;
          if (!interactionSurface(e.target)) return;
          modifiers = { ctrl: e.ctrlKey, shift: e.shiftKey };
          if (e.button === 0) {
            press = {
              x: e.clientX,
              y: e.clientY,
              shift: e.shiftKey,
              ray: handle.rayAt!(e.clientX, e.clientY),
              dragged: false,
            };
            canvas.setPointerCapture(e.pointerId);
          } else if (e.button === 2) {
            pointer = { x: e.clientX, y: e.clientY };
            const pick = makePick(e, "right");
            rightPress = { event: e, ray: pick.ray, pick };
            onRightPressRef.current?.(pick);
            canvas.setPointerCapture(e.pointerId);
          } else if (e.button === 1) {
            e.preventDefault();
            orbit = { x: e.clientX, y: e.clientY };
            canvas.setPointerCapture(e.pointerId);
          }
        };
        const onMove = (e: PointerEvent) => {
          if (!inputEnabledRef.current) return;
          modifiers = { ctrl: e.ctrlKey, shift: e.shiftKey };
          const target = e.target;
          const over =
            target === canvas ||
            (target instanceof Element && target.closest(".ro-layer .ro-unit, .hud"));
          pointer = over || rightPress || press || orbit ? { x: e.clientX, y: e.clientY } : null;
          if (press && Math.hypot(e.clientX - press.x, e.clientY - press.y) > CLICK_SLOP_PX) {
            press.dragged = true;
            // A drag that is an action (placing, then facing) draws no selection box.
            if (!leftDragActionRef.current)
              setBox({ x0: press.x, y0: press.y, x1: e.clientX, y1: e.clientY });
          }
          if (!orbit) return;
          const dx = e.clientX - orbit.x,
            dy = e.clientY - orbit.y;
          orbit.x = e.clientX;
          orbit.y = e.clientY;
          const h = canvas.clientHeight || 1;
          steer({ drag: [dx / h, dy / h] }, 0);
          see(0);
        };
        const onUp = (e: PointerEvent) => {
          if (!inputEnabledRef.current) return;
          orbit = null;
          if (e.button === 2 && rightPress) {
            const start = rightPress.event;
            const captured = rightPress.pick;
            rightPress = null;
            const dragged =
              Math.hypot(e.clientX - start.clientX, e.clientY - start.clientY) > CLICK_SLOP_PX;
            onPickRef.current?.({
              ...captured,
              release: dragged ? handle.rayAt!(e.clientX, e.clientY) : undefined,
            });
            return;
          }
          if (e.button !== 0 || !press) return;
          const start = press;
          press = null;
          setBox(null);
          // A drag that is an action ends as a click: the route aimed it while held.
          if (
            leftDragActionRef.current ||
            Math.hypot(e.clientX - start.x, e.clientY - start.y) <= CLICK_SLOP_PX
          ) {
            onPickRef.current?.(makePick(e, "left"));
          } else {
            onBoxRef.current?.({
              x0: Math.min(start.x, e.clientX),
              y0: Math.min(start.y, e.clientY),
              x1: Math.max(start.x, e.clientX),
              y1: Math.max(start.y, e.clientY),
              shift: start.shift,
              project: handle.projectToCss!,
            });
          }
        };
        const onLeave = (e: PointerEvent) => {
          const to = e.relatedTarget;
          if (!rightPress && !(to instanceof Element && to.closest(".ro-layer .ro-unit"))) {
            pointer = null;
          }
        };
        const cancelGesture = () => {
          pointer = null;
          rightPress = null;
          press = null;
          orbit = null;
          setBox(null);
          modifiers = { ctrl: false, shift: false };
          onRightPressRef.current?.(null);
        };
        cancelInputRef.current = cancelGesture;
        const onWheel = (e: WheelEvent) => {
          if (!inputEnabledRef.current) return;
          const target = e.target;
          if (
            target !== canvas &&
            !(target instanceof Element && target.closest(".ro-layer .ro-unit"))
          )
            return;
          e.preventDefault();
          steer({ wheel: e.deltaY }, 0);
          see(0);
        };
        const onResize = () => (dirty = true);
        window.addEventListener("pointerdown", onDown, { signal });
        window.addEventListener("pointermove", onMove, { signal });
        canvas.addEventListener("pointerup", onUp, { signal });
        canvas.addEventListener("pointerleave", onLeave, { signal });
        canvas.addEventListener("pointercancel", cancelGesture, { signal });
        canvas.addEventListener(
          "lostpointercapture",
          () => {
            // Normal release ends the gesture, not the stationary pointer's hover.
            if (rightPress || press || orbit) cancelGesture();
          },
          { signal },
        );
        window.addEventListener("blur", cancelGesture, { signal });
        window.addEventListener(
          "keydown",
          (e) => {
            modifiers = { ctrl: e.ctrlKey, shift: e.shiftKey };
            if (e.code === "Escape") cancelGesture();
          },
          { signal },
        );
        window.addEventListener(
          "keyup",
          (e) => {
            modifiers = { ctrl: e.ctrlKey, shift: e.shiftKey };
          },
          { signal },
        );
        window.addEventListener("wheel", onWheel, { passive: false, signal });
        canvas.addEventListener("contextmenu", (e) => e.preventDefault(), { signal });
        window.addEventListener("resize", onResize, { signal });
        handle.ready = true;
        if (new URLSearchParams(window.location.search).has("inspect")) {
          setInspecting(scene);
        }
        requestAnimationFrame(() => {
          if (signal.aborted) return;
          setReady(true);
          onReadyRef.current?.({
            allocations,
            device: info.device,
            format: info.format,
            frame: () => scene,
          });
        });
      } catch (err) {
        if (signal.aborted) return;
        appResources.refuse(err);
        const message = gpuFailureMessage(err);
        handle.error = message;
        setError(message);
      }
    })();

    return () => {
      lifetime.abort();
      cancelInputRef.current = null;
      cancelAnimationFrame(raf);
      keys.detach();
      setCursorAction("default");
      onRightPressRef.current?.(null);
      sceneRef.current?.dispose();
      sceneRef.current = null;
      canvasContext?.unconfigure();
      if (window.__lab === handle) delete window.__lab;
    };
    // The viewport is rebuilt only when the fixture identity changes.
  }, [fixture]);

  return (
    <div style={{ position: "absolute", inset: 0 }}>
      <canvas ref={canvasRef} style={{ width: "100%", height: "100%", display: "block" }} />
      {!pilot && <FrameRate handle={frameRate} />}
      {box && (
        <div
          className="lab-box"
          style={{
            left: Math.min(box.x0, box.x1),
            top: Math.min(box.y0, box.y1),
            width: Math.abs(box.x1 - box.x0),
            height: Math.abs(box.y1 - box.y0),
          }}
        />
      )}
      {inspecting && (
        <PassInspector
          frame={inspecting}
          setView={(view) => void window.__lab?.setFrameView?.(view)}
        />
      )}
    </div>
  );
}
