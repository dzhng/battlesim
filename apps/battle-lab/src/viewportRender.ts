/** The viewport's render callbacks and owned GPU handle. */
import type { AllocationCounts, DrawnInstances, SceneStats } from "@web/battle/render/sceneStats";
import type { BrowserGpu } from "@wasm/renderer_browser";
import type { BrowserScene } from "@web/battle/render/scene";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPose, CameraPresentation } from "@packages/renderer-core/src/cameraController";
import type { GrassClumpRow } from "@packages/battle-renderer/src/grass";
import type { EffectCounts, ModelCounts } from "./presentedFrame";

import type { FeedSource } from "@web/feed";
import type { GraphicsQualityProfile } from "./graphicsSettings";
import type { WorldLayers, WorldMeshes, SceneInstance } from "@packages/battle-renderer/src/scene";
import type {
  SceneBuildings,
  BuildingStyle,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { CameraObstacles } from "@packages/renderer-core/src/cameraObstacles";
import type { FogRequest } from "@web/battle/sim/presentation";
import type { FogStyle } from "@packages/battle-renderer/src/fog";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type { FocusTarget } from "@web/battle/input/contactFocus";
import type { ViewportPointer, LabPick, LabBox } from "./viewportInput";
import type { CursorAction } from "@web/battle/present/gameCursor";
import type { Project } from "@web/battle/present/readouts";
import type { ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import type { ViewportFrame } from "./presentedFrame";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import type { PickBox } from "@packages/battle-renderer/src/picking";
import type { Mesh } from "@packages/battle-renderer/src/mesh";

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
  /** Renderer diagnostics and a fresh GPU measurement boundary. */
  attach?(frame: {
    adapter: string;
    stats: () => SceneStats & {
      receiverFit: ReturnType<ScenePresentation["shadowStats"]> | null;
    };
    resetGpuTiming: () => void;
  }): void;
  /** The framing for the frame at `now`; the rig places it
   *  (`CameraController.place`) and draws it clear of obstacles (`resolve`).
   *  A full camera is already resolved and drawn as recorded (human replays).
   *  Null leaves the camera to the player for that frame. */
  pose(now: number): CameraPose | Camera3DParams | null;
  /** Measurements and optional readback from the task that drew this frame. */
  frame?(f: {
    now: number;
    cpuMs: number;
    receiverFit: ReturnType<ScenePresentation["shadowStats"]> | null;
    camera: Camera3DParams;
    simTime: number;
    canvas: readonly [number, number];
    counts: () => FrameCounts;
    plate?: () => FramePlate;
  }): void;
}

/** What the viewport's device reports once it is up. */
export interface ViewportGpu {
  allocations: () => AllocationCounts;
  /** Independent workbench outputs share the page's admitted GPU. */
  gpu: BrowserGpu;
  /** The live scene (it is replaced by `rebuild`). */
  frame: () => BrowserScene;
}

/** A one-shot ask from the page: put the orbit target on a ground point,
 *  keeping the current angle and zoom. `token` changes for every new ask. */
export interface CameraFocusRequest {
  token: number;
  target: readonly [number, number];
}

export interface LabViewportProps {
  /** Resolved graphics profile; omitted keeps the authored High profile. */
  quality?: GraphicsQualityProfile;
  fixture: string;
  /** A loading cover or menu owns player input while false. */
  inputEnabled?: boolean;
  /** Canonical world inputs, published through the stable feed. */
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
   *  overlay and uploaded apart from it. */
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
  /** A one-shot move to a ground point, keeping the current angle and zoom. */
  cameraFocus?: CameraFocusRequest | null;
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
    renderScale: number,
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
  /** Release borrowed GPU callbacks before the viewport frees its renderer. */
  onDispose?: () => void;
  /** Called every animation frame with the live world → page projection and
   *  camera, for DOM readouts anchored to world points and panned sound, and
   *  a shared pointer snapshot: position, camera ray (null over UI), and the
   *  captured world ray of a held right press. The remaining arguments are the
   *  canvas render-pixel/CSS-pixel ratio and battle time in seconds. */
  onFrame?: (
    project: Project,
    camera: Camera3DParams,
    pointer: ViewportPointer,
    renderScale: number,
    simTime: number,
  ) => void;
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

export type ViewportCursor = NonNullable<LabViewportProps["onCursor"]>;
