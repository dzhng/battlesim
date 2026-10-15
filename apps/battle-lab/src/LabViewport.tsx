import { FrameRate, type FrameRateHandle } from "@web/battle/present/frameRate";
import { useLabLoading } from "./LabLoading";
import { diagnosticsSource } from "@web/diagnostics";
import {
  loadPresentation,
  ScenePresentation,
  type PropStats,
  type PresentedUi,
} from "@web/battle/sim/presentation";
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
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import { readTexturePng } from "@packages/renderer-core/src/textureReadback";
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
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import { trackHeldKeys } from "@web/battle/input/heldKeys";
import { focusStep, type FocusTarget } from "@web/battle/input/contactFocus";
import { isGameKey } from "@web/battle/input/keyGuard";
import { HIGH_PLANE_ATTR, type Project } from "@web/battle/present/readouts";
import { gameCamera } from "./gameCamera";
import { gameLightFor } from "./gameLight";
import { gameFogGeometry, gameFogStyle } from "./gameFog";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "./gameOverlay";
import { gameBuildingStyle, gameGlass, gameModelDetail } from "./gameModels";
import type { FogStyle } from "@packages/battle-renderer/src/frame/fogStyle";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type {
  BattleFrame,
  DrawnInstances,
  FrameView,
  SceneInstance,
  WorldLayers,
  WorldMeshes,
} from "@packages/battle-renderer/src/scene";
import type { GroundMarks } from "@packages/battle-renderer/src/frame/scarTexture";
import type {
  BuildingStyle,
  SceneBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { SurfaceClass } from "@packages/battle-renderer/src/models/surfaceParts";
import { cascadeLight, createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import type { GrassClumpRow } from "@packages/battle-renderer/src/frame/grassPass";
import { PassInspector } from "./PassInspector";
import type { FeedSource } from "./feed";
import {
  emptyEffectCounts,
  emptyModelCounts,
  presentedCamera,
  type EffectCounts,
  type ModelCounts,
} from "./presentedFrame";
import { emptyBuildingWalkStats } from "@packages/battle-renderer/src/models/buildingLayer";
import { NO_SCENERY_WALK } from "@packages/battle-renderer/src/frame/sceneryLayer";
import {
  resolveFogLook,
  resolveLight,
  type FogRequest,
  type PresentedAppearance,
  type PresentedFrame,
} from "@web/battle/sim/presentation";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import { pickBox, proxyPickBox, type PickBox } from "@packages/battle-renderer/src/picking";
import {
  graphicsProfiles,
  sceneMinimumProfile,
  type GraphicsQualityProfile,
} from "./graphicsSettings";

const NO_INSTANCES: readonly SceneInstance[] = [];
const NO_WALK = emptyBuildingWalkStats();
/** A frame of a world no battle publishes into: no units, no selection. */
const NO_UI: PresentedUi = { selected: [], xray: false, ghosts: [] };

interface LabViewportProps {
  /** Resolved graphics profile; omitted keeps the authored High profile. */
  quality?: GraphicsQualityProfile;
  fixture: string;
  /** A loading cover or menu owns player input while false. */
  inputEnabled?: boolean;
  /** The static world, fed like the overlay (`useFeed`); drawn once it is set. */
  world: FeedSource<WorldLayers | null>;
  /** The map's buildings, fed like the overlay; omitted or null draws none.
   *  They are drawn by the scene's one presentation: the battle's
   *  (`presentation`), else one the viewport makes with no layout, whose
   *  side knows what the buildings' `known` says. */
  buildings?: FeedSource<SceneBuildings | null>;
  /** The battle's presentation (its sim client's), read each frame: null
   *  until the battle has one. Omitted for a scene with no battle. */
  presentation?: { readonly current: ScenePresentation | null };
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
  /** The side's fog of war, which its presentation decides (its sensor
   *  rules, foliage, and a lab's stand-ins), fed like the overlay; omitted
   *  or null draws no fog. */
  fog?: FeedSource<FogRequest | null>;
  /** How unseen looks; the fixture's selected style when omitted. Live. */
  fogStyle?: FogStyle;
  /** The light, fixed for the viewport's life; the fixture's when omitted. */
  light?: LightPresentation;
  /** The proxy instances (none by default: a battle draws posed models). */
  instances?: readonly SceneInstance[];
  initialCamera: Camera3DParams;
  /** Contacts the camera steps through, newest first; empty leaves N inert. */
  focusTargets?: readonly FocusTarget[];
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
   *  (presentation interpolation between completed ticks). `generation` is
   *  the GPU frame's static generation, for the presentation's frame. */
  frame?: (now: number, camera: ViewportCamera, generation: number) => ViewportFrame | null;
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
  /** Appearance bundles from `AppearanceLibrary` for the models layer, fed
   *  like the overlay: the bundles and the template library's bytes are
   *  megabytes a changing prop would copy into React's measures. */
  appearances?: FeedSource<InstalledAppearances | null>;
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
  /** The presentation's frame (`crates/presentation`): the units, corpses,
   *  ghosts and effects it draws, and the static world's batches, read
   *  before the next call into the module. */
  presented?: PresentedFrame;
  /** How many models of each kind the frame poses. */
  counts?: ModelCounts;
  /** The presentation clock, in seconds (the pose driver's and the wind's). */
  clock?: number;
  /** What the frame's effects hold, by kind and shape. */
  effectCounts?: EffectCounts;
  /** The observing side's learned ground (the presentation's `LearnedGround`), drawn
   *  as scars; the frame uploads only what changed since it last looked. */
  ground?: GroundMarks | null;
}

/** What a drawn frame drew, counted, for a second client to count against:
 *  the building modules, scenery and props, the models it posed and the
 *  combat effects. */
export interface FrameCounts {
  drawn: DrawnInstances;
  /** The soldiers, vehicles and corpses the frame posed. */
  models: ModelCounts;
  /** The combat effects the frame drew. */
  effects: EffectCounts;
}

/** A drawn frame read back for a second client to draw against: the canvas
 *  as the player saw it, the grass clumps the frame grew, its counts, and
 *  the same frame drawn again offscreen lit without its grass. The canvas
 *  and the next frame are untouched. */
export interface FramePlate extends FrameCounts {
  png: Promise<Blob>;
  clumps: Promise<readonly Pick<GrassClumpRow, "root">[]>;
  plain: Promise<Blob>;
}

/** A script's hold on the viewport's camera. */
export interface ViewportPilot {
  /** The pilot reads drawn frames back (`frame`'s `plate`). Only then may
   *  the canvas be copied from, so no other frame pays for it. */
  plates?: boolean;
  /** Show the in-game frame-rate counter, which a piloted view otherwise
   *  hides (the graphics test shows it; the menu's reel does not). */
  frameRate?: boolean;
  /** The battle frame is up: the adapter and the frame's own statistics (its rolling GPU frame time from
   *  `timestamp-query`, and live memory). */
  attach?(frame: { adapter: string; stats: () => ReturnType<BattleFrame["stats"]> }): void;
  /** The framing for the frame at `now`; the rig places it
   *  (`CameraController.place`) and draws it clear of obstacles (`resolve`).
   *  Null leaves the camera to the player for that frame. */
  pose(now: number): CameraPose | null;
  /** A drawn frame: its main-thread cost, the camera drawn, the sim time
   *  it drew (the presentation clock) and the canvas size in pixels.
   *  `counts` counts what this frame drew; with `plates`, `plate` reads it
   *  back. Called during `frame`, the task that drew it, both read exactly
   *  that frame. */
  frame?(f: {
    now: number;
    cpuMs: number;
    camera: Camera3DParams;
    simTime: number;
    canvas: readonly [number, number];
    counts: () => FrameCounts;
    plate?: () => FramePlate;
  }): void;
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

/** Pixels a left press may travel and still count as a click. */
const CLICK_SLOP_PX = 5;
/** Screen-edge band that pans the camera like a held key. */
const EDGE_PAN_PX = 14;

/** What holds a drawn pose off the pose asked for, and how it got there. */
type ClearanceReading = Pick<ClearanceState, "hold" | "cut" | "blocked">;

/** Diagnostic hooks the scene harness reads; lab-only, never on a player route. */
interface LabHandle {
  ready: boolean;
  error: string | null;
  adapter?: { vendor: string; architecture: string; description: string; format: string };
  /** The frame's statistics, and what the scene's props draw. */
  stats?: () => ReturnType<BattleFrame["stats"]> & { props: PropStats };
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
  buildings,
  presentation,
  buildingStyle,
  obstacles,
  overlay,
  pointerMarks,
  fog,
  fogStyle,
  light,
  instances = NO_INSTANCES,
  initialCamera,
  focusTargets,
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
  quality,
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
  // Verification can request a test-only renderer without adding a player-facing
  // setting. Production builds ignore the query; the runner chooses minimum or
  // player low for picture scenes.
  const requestedTestQuality = new URLSearchParams(window.location.search).get("test-quality");
  const testQuality =
    import.meta.env.DEV &&
    (requestedTestQuality === "minimum" ||
      requestedTestQuality === "low" ||
      requestedTestQuality === "medium" ||
      requestedTestQuality === "high")
      ? requestedTestQuality === "minimum"
        ? sceneMinimumProfile
        : graphicsProfiles[requestedTestQuality]
      : undefined;
  const effectiveQuality = quality ?? testQuality;
  const qualityRef = useRef(effectiveQuality);
  const qualityKey = effectiveQuality ? JSON.stringify(effectiveQuality) : "";
  const previousQualityKey = useRef(qualityKey);
  qualityRef.current = effectiveQuality;
  useEffect(() => {
    if (previousQualityKey.current === qualityKey) return;
    previousQualityKey.current = qualityKey;
    void window.__lab?.rebuild?.();
  }, [qualityKey]);
  const appearancesRef = useRef(appearances?.current ?? null);
  const modelsRef = useRef(models);
  modelsRef.current = models;
  const presentedWorldRef = useRef<readonly PresentedAppearance[]>([]);
  const countsRef = useRef<ModelCounts>(emptyModelCounts());
  const effectCountsRef = useRef<EffectCounts>(emptyEffectCounts());
  /** The lab's scenery switches: the dressing and the tree lines' shrubs. */
  const sceneryShown = useRef({ dressing: true, understorey: true });
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
  const focusTargetsRef = useRef(focusTargets);
  focusTargetsRef.current = focusTargets;
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
  const buildingsRef = useRef(buildings);
  buildingsRef.current = buildings;
  const presentationRef = useRef(presentation);
  presentationRef.current = presentation;
  const buildingStyleRef = useRef(buildingStyle);
  buildingStyleRef.current = buildingStyle;
  // The frame loop reads the buildings: a change only asks for a frame.
  useEffect(() => buildings?.subscribe(() => redrawRef.current()), [buildings]);
  const obstaclesRef = useRef(obstacles);
  obstaclesRef.current = obstacles;
  const fogRef = useRef(fog);
  fogRef.current = fog;
  const fogStyleRef = useRef(fogStyle);
  fogStyleRef.current = fogStyle;
  const lightRef = useRef(light);
  useEffect(() => {
    sceneRef.current?.setFogLook(resolveFogLook(fogStyle ?? gameFogStyle));
    redrawRef.current();
  }, [fogStyle]);
  const fogSuppressed = useRef(false);
  // The scene's presentation takes the fog each frame (`setFog` below); a
  // change only asks for one.
  useEffect(() => fog?.subscribe(() => redrawRef.current()), [fog]);
  useEffect(
    () =>
      overlay?.subscribe((meshes: WorldMeshes | undefined) => {
        if (meshes) sceneRef.current?.setOverlay(meshes);
        redrawRef.current();
      }),
    [overlay],
  );
  useEffect(() => {
    const install = (next: InstalledAppearances | null) => {
      appearancesRef.current = next;
      const scene = sceneRef.current;
      if (!scene) return;
      void scene.setAppearances(next).then(() => {
        if (modelsRef.current) scene.setModels(modelsRef.current);
        redrawRef.current();
      });
    };
    install(appearances?.current ?? null);
    return appearances?.subscribe(install);
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
    // The battlefield the pointer acts on: the canvas, or a callout over it.
    // The HUD's controls are not: over them the ground neither hovers nor
    // measures, and the app's cursor (AppCursor) shows there regardless.
    const interactionSurface = (target: EventTarget | null) =>
      target === canvas
        ? canvas
        : target instanceof Element
          ? target.closest<HTMLElement>(".ro-layer .ro-unit")
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
    /** The contact last visited; null before any. */
    let focused: Pick<FocusTarget, "id" | "evidenceTick"> | null = null;
    /** Jump to the next (older) or previous (newer) contact; angle and zoom kept. */
    const jumpToEvidence = (newer: boolean) => {
      if (piloted || !inputEnabledRef.current) return;
      const targets = focusTargetsRef.current ?? [];
      const index = focusStep(targets, focused, newer);
      const target = index < 0 ? undefined : targets[index];
      if (!target) return;
      focused = target;
      place(
        controller.place(asked, {
          target: target.target,
          distance: asked.distance,
          yaw: asked.yaw,
          pitch: asked.pitch,
        }),
      );
    };
    window.addEventListener(
      "keydown",
      (e) => {
        if (e.code !== "KeyN" || !isGameKey(e)) return;
        jumpToEvidence(e.shiftKey);
      },
      { signal },
    );
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
        const plates = pilot?.plates === true;
        context.configure({
          device,
          format: info.format,
          alphaMode: "opaque",
          usage: GPUTextureUsage.RENDER_ATTACHMENT | (plates ? GPUTextureUsage.COPY_SRC : 0),
        });
        // One pixel per CSS pixel, on any display: a high-density screen
        // scales the battle up rather than drawing it at its own density.
        // The page's HUD keeps the display's density.
        const syncSize = () => {
          const w = Math.max(1, canvas.clientWidth);
          const h = Math.max(1, canvas.clientHeight);
          if (canvas.width !== w || canvas.height !== h) {
            canvas.width = w;
            canvas.height = h;
            dirty = true;
          }
        };
        syncSize();
        // The high-air models' canvas, drawn by the same frame: laid over this
        // one between the card planes when the page has them
        // (`HIGH_PLANE_ATTR`), so a high-air model rises above every ground
        // and low-air card. Without them, and for control plates (which a
        // second client's single picture is held to), the models draw with
        // the world.
        const highCanvas = document.createElement("canvas");
        const highContext = plates ? null : highCanvas.getContext("webgpu");
        highContext?.configure({ device, format: info.format, alphaMode: "premultiplied" });
        signal.addEventListener("abort", () => {
          highContext?.unconfigure();
          highCanvas.remove();
        });
        let highAt = "";
        const highTarget = (): GPUTextureView | null => {
          const host = highContext && document.querySelector(`[${HIGH_PLANE_ATTR}]`);
          if (!host) {
            highCanvas.remove();
            return null;
          }
          if (highCanvas.parentElement !== host) host.appendChild(highCanvas);
          if (highCanvas.width !== canvas.width || highCanvas.height !== canvas.height) {
            highCanvas.width = canvas.width;
            highCanvas.height = canvas.height;
          }
          const r = canvas.getBoundingClientRect();
          const h = host.getBoundingClientRect();
          const at = `${r.left - h.left} ${r.top - h.top} ${r.width} ${r.height}`;
          if (at !== highAt) {
            highAt = at;
            Object.assign(highCanvas.style, {
              left: `${r.left - h.left}px`,
              top: `${r.top - h.top}px`,
              width: `${r.width}px`,
              height: `${r.height}px`,
            });
          }
          return highContext!.getCurrentTexture().createView();
        };
        let clock = 0;
        // A route's own light, else the fixture's (a lab URL's `?sun=` sets it lower).
        const light = () => lightRef.current ?? gameLightFor(window.location.search);
        /** The presentation the viewport makes for a scene with no battle,
         *  once the module is loaded and there are buildings to draw. */
        let own: ScenePresentation | null = null;
        let moduleLoaded = false;
        void loadPresentation().then(() => (moduleLoaded = true));
        signal.addEventListener("abort", () => own?.free());
        /** The scene's one presentation: the battle's, else the viewport's own. */
        const scenePresentation = (): ScenePresentation | null => {
          const battle = presentationRef.current;
          if (battle) return battle.current;
          if (!own && moduleLoaded) own = new ScenePresentation();
          return own;
        };
        const propStats = (): PropStats =>
          scenePresentation()?.propStats() ?? {
            bodies: 0,
            parts: 0,
            unmodelled: 0,
            models: 0,
            known: 0,
          };
        /** The buildings' style and sun, and the light the cascades are
         *  fitted under, taken when the frame is built. */
        let staticStyle = gameBuildingStyle;
        let staticSun = { azimuth: 0, elevation: 1, farM: 0 };
        let shadowLight: LightPresentation | null = null;
        /** A presentation that frames the camera alone, while the battle's
         *  is not there yet. */
        let bare: ScenePresentation | null = null;
        signal.addEventListener("abort", () => bare?.free());
        /** The presentation and quality the detail scale was last given to. */
        let detailed: [
          ScenePresentation | null,
          GraphicsQualityProfile | undefined,
          boolean,
          boolean,
        ] = [null, undefined, true, true];
        const build = async () => {
          const lit = light();
          staticSun = {
            azimuth: lit.sun_azimuth,
            elevation: lit.sun_elevation,
            farM: lit.cascades.max_far_m,
          };
          staticStyle = buildingStyleRef.current ?? gameBuildingStyle;
          shadowLight = cascadeLight(lit, qualityRef.current);
          const next = await createBattleFrame(device!, info.format, {
            light: await resolveLight(lit),
            fogGeometry: gameFogGeometry,
            fogLook: resolveFogLook(fogStyleRef.current ?? gameFogStyle),
            overlayGlow: gameOverlayGlow,
            paint: gamePaint,
            xrayMinHiddenFragmentFraction: gameXrayMinHiddenFragmentFraction,
            models: gameModelDetail,
            glass: gameGlass,
            quality: qualityRef.current,
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
            const meshes = overlayRef.current?.current;
            if (meshes) next.setOverlay(meshes);
            const marks = pointerMarksRef.current?.current;
            if (marks) next.setPointerMarks(marks);
            if (appearancesRef.current) await next.setAppearances(appearancesRef.current);
            if (signal.aborted) {
              next.dispose();
              return next;
            }
            if (modelsRef.current) next.setModels(modelsRef.current);
            next.setPresentationWorld(presentedWorldRef.current);
            next.setPresentedGround(scenePresentation()?.presentedGround() ?? null);
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
        const adapter = info.description || `${info.vendor} ${info.architecture}`.trim();
        pilot?.attach?.({
          adapter,
          stats: () => scene.stats(),
        });
        // What the battle draws, for a player's diagnostics report.
        signal.addEventListener(
          "abort",
          diagnosticsSource("battle view", () => ({
            adapter,
            format: info.format,
            canvas: [canvas.width, canvas.height],
            gpu: scene.stats().gpu,
            gpuAllocations: allocations(),
          })),
        );
        const draw = () => {
          // Cleared first: a frame may ask to be drawn again (`requestRedraw`).
          dirty = false;
          scene.render(context.getCurrentTexture().createView(), highTarget());
        };
        /** Whether the lab took the grass away (`suppressGrass`). */
        let grassSuppressed = false;
        /** The frame just drawn again without its grass, into a texture of
         *  the canvas's size and format, read back; the grass is put back,
         *  so the canvas and the next frame are as they were. */
        const readPlain = () => {
          const texture = device!.createTexture({
            label: "plate-readback",
            size: [canvas.width, canvas.height],
            format: info.format,
            usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC,
          });
          scene.grassProbes.suppress(true);
          scene.render(texture.createView());
          scene.grassProbes.suppress(grassSuppressed);
          return readTexturePng(device!, texture).finally(() => texture.destroy());
        };
        const counts = (): FrameCounts => ({
          drawn: scene.drawnInstances(),
          models: countsRef.current,
          effects: effectCountsRef.current,
        });
        // What the frame drew is read first: the grass-less draw changes the
        // frame, and may lay more dressing or expand more buildings.
        const plate = (): FramePlate => ({
          png: readTexturePng(device!, context.getCurrentTexture()),
          clumps: scene.grassProbes.clumps(),
          ...counts(),
          plain: readPlain(),
        });
        redrawRef.current = () => (dirty = true);
        let pointer: { x: number; y: number } | null = null;
        let modifiers = { ctrl: false, shift: false };
        const pointerRay = () => (pointer ? handle.rayAt!(pointer.x, pointer.y) : null);
        let lastFrame = performance.now();
        const loop = (now: number) => {
          if (signal.aborted) return;
          const dt = Math.min(0.1, (now - lastFrame) / 1000);
          lastFrame = now;
          // Sized before the camera is framed: the frame draws at the size
          // its camera was framed for.
          syncSize();
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
          // The scene's presentation is staged with its buildings before the
          // battle frames it (or, without one, the viewport frames it).
          const decoder = scenePresentation();
          const terrain = worldRef.current.current?.terrain?.mesh ?? null;
          let animated: ViewportFrame | null | undefined = null;
          try {
            if (decoder) {
              decoder.setBuildings(
                buildingsRef.current?.current ?? null,
                appearancesRef.current ?? null,
                staticStyle,
                staticSun,
              );
              const layers = worldRef.current.current;
              decoder.setGround(layers?.terrain.input ?? null, layers?.scenery ?? null);
              decoder.setShadow(shadowLight!, terrain);
              decoder.setFog(fogSuppressed.current ? null : (fogRef.current?.current ?? null));
              const quality = qualityRef.current;
              const shown = sceneryShown.current;
              if (
                detailed[0] !== decoder ||
                detailed[1] !== quality ||
                detailed[2] !== shown.dressing ||
                detailed[3] !== shown.understorey
              ) {
                detailed = [decoder, quality, shown.dressing, shown.understorey];
                decoder.setDetailScale(quality?.unitLodScale ?? 1, quality?.buildingLodScale ?? 1);
                decoder.setSceneryQuality(
                  quality?.treeLodScale ?? 1,
                  (quality?.forestDressing ?? true) && shown.dressing,
                  (quality?.understorey ?? true) && shown.understorey,
                );
              }
            }
            animated = frameRef.current?.(now, snapshot(), scene.staticGeneration);
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            handle.error = message;
            setError(message);
          }
          if (animated?.ground !== undefined) groundRef.current = animated.ground;
          if (scene.setGround(groundNow())) dirty = true;
          if (animated) {
            if (animated.clock !== undefined) clock = animated.clock;
            if (animated.instances) {
              instancesRef.current = animated.instances;
              scene.setInstances(animated.instances);
            }
            if (animated.picks) picksRef.current = animated.picks;
            const world = decoder?.world();
            if (world && world !== presentedWorldRef.current) {
              presentedWorldRef.current = world;
              scene.setPresentationWorld(world);
            }
            if (animated.presented)
              scene.setFrame(animated.presented, {
                models: !modelsSuppressed.current,
                effects: !effectsSuppressed.current,
              });
            if (animated.counts) countsRef.current = animated.counts;
            if (animated.effectCounts) effectCountsRef.current = animated.effectCounts;
            dirty = true;
          }
          // The camera drawn and the static world for it: walked in Rust,
          // its batches written only when what it draws changed. A battle's
          // frame carries them; a scene without one (or before its first
          // publication) is framed here for them alone, and before the
          // battle's presentation is there, the camera alone.
          try {
            let views: PresentedFrame | null = null;
            let walk = NO_WALK;
            let scenery = NO_SCENERY_WALK;
            if (!decoder) {
              bare ??= new ScenePresentation();
              bare.setShadow(shadowLight!, terrain);
              views = bare.frame(clock, presentedCamera(snapshot()), NO_UI, 0);
            } else {
              if (!animated?.presented) {
                decoder.frame(
                  clock,
                  presentedCamera(snapshot()),
                  { ...NO_UI, known: buildingsRef.current?.current?.known },
                  scene.staticGeneration,
                );
                const world = decoder.world();
                if (world && world !== presentedWorldRef.current) {
                  presentedWorldRef.current = world;
                  scene.setPresentationWorld(world);
                }
              }
              // A new ground's tables and the stats calls may grow memory:
              // the views are taken after them.
              scene.setPresentedGround(decoder.presentedGround());
              walk = decoder.buildingWalk();
              scenery = decoder.sceneryWalk();
              views = decoder.lastFrame();
            }
            if (views) scene.setCamera(views);
            if (scene.setStaticFrame(decoder ? views : null, { buildings: walk, scenery }))
              dirty = true;
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            handle.error = message;
            setError(message);
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
          const active = surface && !orbit && !pilot;
          setCursorAction((active && action) || "default");
          pilot?.frame?.({
            now,
            cpuMs: performance.now() - started,
            camera,
            simTime: clock,
            canvas: [canvas.width, canvas.height],
            counts,
            plate: plates ? plate : undefined,
          });
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
          stats: () => ({ ...scene.stats(), props: propStats() }),
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
            // Build the replacement while the current generation remains live;
            // only swap after every GPU resource has initialized successfully.
            const previous = scene;
            const next = await build();
            scene = next;
            previous.dispose();
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
            grassSuppressed = on;
            scene.grassProbes.suppress(on);
            await nextFrame();
          },
          async suppressFog(on: boolean) {
            fogSuppressed.current = on;
            dirty = true;
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
            await nextFrame();
          },
          async suppressTrees(on: boolean) {
            scene.setTreesShown(!on);
            await nextFrame();
          },
          async suppressDressing(on: boolean) {
            sceneryShown.current = { ...sceneryShown.current, dressing: !on };
            await nextFrame();
          },
          async suppressUnderstorey(on: boolean) {
            sceneryShown.current = { ...sceneryShown.current, understorey: !on };
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
        } satisfies Partial<LabHandle>);

        // Input: left click selects, left drag box-selects (or, as an action,
        // places at the press and faces toward the release), right click
        // orders (on release: a right-drag also sets the facing); the camera (CameraController) takes held WASD/arrows and the
        // screen edge to pan, Q/E to turn, Z/X to tilt, middle drag to orbit, the wheel to zoom.
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
          const over = interactionSurface(e.target);
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
      {(!pilot || pilot.frameRate) && <FrameRate handle={frameRate} />}
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
