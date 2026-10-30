export type { ViewportGpu } from "./viewportRender";
import type {
  CameraFocusRequest,
  FrameCounts,
  FramePlate,
  ViewportGpu,
  ViewportPilot,
} from "./viewportRender";
import type { ViewportPointer, LabPick, LabBox } from "./viewportInput";
export type { ViewportPointer, LabPick, LabBox } from "./viewportInput";
import type { ClearanceReading, LabHandle } from "./labDiagnostics";
import { FrameRate, type FrameRateHandle } from "@web/battle/present/frameRate";
import { useLabLoading } from "./LabLoading";
import { diagnosticsSource } from "@web/diagnostics";
import {
  ScenePresentation,
  type PropStats,
  NO_PROP_STATS,
  type PresentedUi,
} from "@web/battle/sim/presentation";
import { useCursorAction, type CursorAction } from "@web/battle/present/gameCursor";
import { useEffect, useRef, useState } from "react";
import { appResources, gpuFailureMessage } from "./appResources";
import { BrowserScene, pixelsPng } from "@web/battle/render/scene";
import type { BrowserGpuInfo, FrameView } from "@web/battle/render/sceneStats";
import {
  createWorldRay,
  screenRay,
  type Camera3DParams,
  type WorldRay,
} from "@packages/renderer-core/src/camera3d";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/viewportCamera";
import {
  CAMERA_KEYS,
  CameraController,
  type CameraIntent,
  type CameraPose,
  type CameraPresentation,
} from "@packages/renderer-core/src/cameraController";
import { createClearanceState } from "@packages/renderer-core/src/cameraClearance";
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
import { applyShadowQuality } from "@packages/renderer-core/src/lightQuality";
import { gameFogStyle } from "./gameFog";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "./gameOverlay";
import { gameBuildingStyle, gameModelDetail } from "./gameModels";
import type { FogStyle } from "@packages/battle-renderer/src/fog";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type { SceneInstance, WorldLayers, WorldMeshes } from "@packages/battle-renderer/src/scene";
import type {
  BuildingStyle,
  SceneBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { SurfaceClass } from "@packages/battle-renderer/src/models/surfaceParts";
import { PassInspector } from "./PassInspector";
import { ViewportFrames } from "./viewportFrames";
import { viewportProjector } from "./viewportProjection";
import type { FeedSource } from "./feed";
import {
  emptyEffectCounts,
  emptyModelCounts,
  presentedCamera,
  type EffectCounts,
  type ModelCounts,
  type ViewportFrame,
} from "./presentedFrame";
import { type FogRequest } from "@web/battle/sim/presentation";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import { pickBox, proxyPickBox, type PickBox } from "@packages/battle-renderer/src/picking";
import { testQualityProfile, type GraphicsQualityProfile } from "./graphicsSettings";

const NO_MODELS: readonly ModelInstance[] = [];
const NO_INSTANCES: readonly SceneInstance[] = [];
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
   *  captured world ray of a held right press. The final argument is the actual
   *  canvas render-pixel/CSS-pixel ratio, for minimum stroke coverage. */
  onFrame?: (
    project: Project,
    camera: Camera3DParams,
    pointer: ViewportPointer,
    renderScale: number,
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

/** The viewport owns the live projection and render resolution passed to pointer paint. */
export type ViewportCursor = NonNullable<LabViewportProps["onCursor"]>;

/** Pixels a left press may travel and still count as a click. */
const CLICK_SLOP_PX = 5;
/** Screen-edge band that pans the camera like a held key. */
const EDGE_PAN_PX = 14;

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
  cameraFocus,
  groundAt,
  onPick,
  leftDragAction = false,
  onRightPress,
  onCursor,
  onBox,
  frame,
  onFrame,
  onReady,
  onDispose,
  diagnostics,
  pilot,
  appearances,
  models = NO_MODELS,
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
  const effectiveQuality = quality ?? testQualityProfile(window.location.search);
  const qualityRef = useRef(effectiveQuality);
  const qualityKey = effectiveQuality ? JSON.stringify(effectiveQuality) : "";
  const previousQualityKey = useRef(qualityKey);
  const rebuildRef = useRef<() => void>(() => {});
  qualityRef.current = effectiveQuality;
  useEffect(() => {
    if (previousQualityKey.current === qualityKey) return;
    previousQualityKey.current = qualityKey;
    rebuildRef.current();
  }, [qualityKey]);
  const appearancesRef = useRef(appearances?.current ?? null);
  const modelsRef = useRef(models);
  modelsRef.current = models;
  const countsRef = useRef<ModelCounts>(emptyModelCounts());
  const effectCountsRef = useRef<EffectCounts>(emptyEffectCounts());
  /** The lab's scenery switches: the dressing and the tree lines' shrubs. */
  const sceneryShown = useRef({ dressing: true, understorey: true });
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
  const sceneRef = useRef<BrowserScene | null>(null);
  const [inspecting, setInspecting] = useState<BrowserScene | null>(null);
  instancesRef.current = instances;

  const redrawRef = useRef<() => void>(() => {});
  const onPickRef = useRef(onPick);
  onPickRef.current = onPick;
  const onReadyRef = useRef(onReady);
  onReadyRef.current = onReady;
  const onDisposeRef = useRef(onDispose);
  onDisposeRef.current = onDispose;
  const diagnosticsRef = useRef(diagnostics);
  diagnosticsRef.current = diagnostics;
  const initialCameraRef = useRef(initialCamera);
  const focusTargetsRef = useRef(focusTargets);
  focusTargetsRef.current = focusTargets;
  const cameraFocusRef = useRef(cameraFocus);
  cameraFocusRef.current = cameraFocus;
  const groundAtRef = useRef(groundAt);
  groundAtRef.current = groundAt;
  useEffect(() => {
    if (window.__lab) window.__lab.route = diagnostics;
  }, [diagnostics]);
  useEffect(() => {
    redrawRef.current();
  }, [instances, models]);
  useEffect(() => world.subscribe(() => redrawRef.current()), [world]);
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
    sceneRef.current?.setFogStyle(fogStyle ?? gameFogStyle);
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
    const changed = (next: InstalledAppearances | null) => {
      appearancesRef.current = next;
      redrawRef.current();
    };
    changed(appearances?.current ?? null);
    return appearances?.subscribe(changed);
  }, [appearances]);

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
    /** The last requested camera move applied; -1 before any. */
    let appliedFocus = -1;
    /** Put the orbit target on a ground point, keeping angle and zoom. */
    const centerOn = (target: readonly [number, number]) => {
      place(
        controller.place(asked, {
          target,
          distance: asked.distance,
          yaw: asked.yaw,
          pitch: asked.pitch,
        }),
      );
    };
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
      centerOn(target.target);
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
    let submittedView: ViewportCamera | null = null;
    let submittedPicks: readonly PickBox[] = [];
    const interactionView = () => submittedView ?? snapshot();

    (async () => {
      try {
        const gpu = await appResources.gpu();
        const info = JSON.parse(gpu.info()) as BrowserGpuInfo;
        if (signal.aborted) return;
        const plates = pilot?.plates === true;
        const syncSize = () => {
          const scale = qualityRef.current?.renderScale ?? 1;
          const w = Math.max(1, Math.round(canvas.clientWidth * scale));
          const h = Math.max(1, Math.round(canvas.clientHeight * scale));
          if (canvas.width !== w || canvas.height !== h) {
            canvas.width = w;
            canvas.height = h;
            sceneRef.current?.resize();
            dirty = true;
          }
        };
        syncSize();
        // The high-air models' canvas, drawn by the same frame: laid over this
        // one between the card planes once the page has them
        // (`HIGH_PLANE_ATTR`), so a high-air model rises above every ground
        // and low-air card. Without them, and for control plates (which a
        // second client's single picture is held to), the models draw with
        // the world.
        const high = plates ? undefined : document.createElement("canvas");
        if (high) Object.assign(high.style, { position: "absolute", pointerEvents: "none" });
        signal.addEventListener("abort", () => high?.remove());
        let highAt = "";
        const placeHigh = () => {
          if (!high) return;
          const host = document.querySelector(`[${HIGH_PLANE_ATTR}]`) ?? canvas.parentElement!;
          if (high.parentElement !== host) host.appendChild(high);
          const r = canvas.getBoundingClientRect();
          const h = host.getBoundingClientRect();
          const at = `${r.left - h.left} ${r.top - h.top} ${r.width} ${r.height}`;
          if (at !== highAt) {
            highAt = at;
            Object.assign(high.style, {
              left: `${r.left - h.left}px`,
              top: `${r.top - h.top}px`,
              width: `${r.width}px`,
              height: `${r.height}px`,
            });
          }
        };
        let clock = 0;
        // A route's own light, else the fixture's (a lab URL's `?sun=` sets it lower).
        const light = () => lightRef.current ?? gameLightFor(window.location.search);
        /** Presentation for a scene without a battle. */
        let own: ScenePresentation | null = null;
        signal.addEventListener("abort", () => own?.free());
        /** The scene's one presentation: the battle's, else the viewport's own. */
        const scenePresentation = (): ScenePresentation | null => {
          const battle = presentationRef.current;
          if (battle) return battle.current;
          if (!own) own = new ScenePresentation();
          return own;
        };
        const propStats = (): PropStats => scenePresentation()?.propStats() ?? NO_PROP_STATS;
        /** The buildings' style and sun, and the light the cascades are
         *  fitted under, taken when the frame is built. */
        let staticStyle = gameBuildingStyle;
        let staticSun = { azimuth: 0, elevation: 1, farM: 0 };
        let shadowLight: LightPresentation | null = null;
        const receiverStats = () => scenePresentation()?.shadowStats() ?? null;
        /** The presentation and quality the detail scale was last given to. */
        let detailed: [
          ScenePresentation | null,
          GraphicsQualityProfile | undefined,
          boolean,
          boolean,
        ] = [null, undefined, true, true];
        const frameRequests = new ViewportFrames(signal, () => {
          dirty = true;
        });
        const reportFailure = (error: unknown) => {
          if (signal.aborted) return;
          frameRequests.fail(error);
          const message = gpuFailureMessage(error);
          handle.error = message;
          handle.ready = false;
          setReady(false);
          setError(message);
        };
        const admissionFrame = () =>
          new Promise<void>((resolve) => {
            const done = () => {
              signal.removeEventListener("abort", abort);
              resolve();
            };
            const frame = requestAnimationFrame(done);
            const abort = () => {
              cancelAnimationFrame(frame);
              done();
            };
            signal.addEventListener("abort", abort, { once: true });
          });
        let rebuilding: Promise<void> | null = null;
        const readyPresentation = async (): Promise<ScenePresentation> => {
          while (!signal.aborted) {
            const decoder = scenePresentation();
            const waitingArt =
              appearances &&
              !appearancesRef.current &&
              (presentationRef.current ||
                modelsRef.current?.length ||
                buildingsRef.current?.current);
            if (decoder && !waitingArt) return decoder;
            await admissionFrame();
          }
          throw new Error("Viewport departed during renderer admission");
        };
        const build = async (): Promise<BrowserScene> => {
          const decoder = await readyPresentation();
          const quality = qualityRef.current;
          const qualityKey = JSON.stringify(quality);
          const lit = light();
          staticSun = {
            azimuth: lit.sun_azimuth,
            elevation: lit.sun_elevation,
            farM: lit.cascades.max_far_m,
          };
          staticStyle = buildingStyleRef.current ?? gameBuildingStyle;
          shadowLight = applyShadowQuality(lit, quality);
          const world = worldRef.current.current!;
          const installed = appearancesRef.current;
          const current = () =>
            decoder === scenePresentation() &&
            installed === appearancesRef.current &&
            world === worldRef.current.current &&
            qualityKey === JSON.stringify(qualityRef.current);
          decoder.setBuildings(
            buildingsRef.current?.current ?? null,
            installed,
            staticStyle,
            staticSun,
          );
          decoder.setGround(world.terrain.input, world.scenery);
          decoder.setShadow(shadowLight, world.terrain.mesh);
          decoder.setFog(fogSuppressed.current ? null : (fogRef.current?.current ?? null));
          let first = frameRef.current?.(performance.now(), snapshot(), 0);
          // Publications start only after GPU readiness. Admit the complete
          // staged battle topology before its first observed frame exists.
          while (presentationRef.current && !decoder.hasBattle() && !signal.aborted) {
            await admissionFrame();
            if (!current()) return build();
            first = frameRef.current?.(performance.now(), snapshot(), 0);
          }
          if (signal.aborted) throw new Error("Viewport departed during renderer admission");
          if (first?.clock !== undefined) clock = first.clock;
          if (!first?.presented) decoder.frame(clock, presentedCamera(snapshot()), NO_UI, 0);
          let next: BrowserScene;
          try {
            next = await BrowserScene.create({
              gpu,
              canvas,
              highCanvas: high,
              presentation: decoder,
              world,
              settings: {
                light: shadowLight,
                fog: fogStyleRef.current ?? gameFogStyle,
                grassDensity: world.grass ? (quality?.grassDensity ?? 1) : undefined,
                paint: gamePaint,
                xrayMinHiddenFraction: gameXrayMinHiddenFragmentFraction,
                waterDetail: quality?.waterDetail,
              },
              detail: JSON.parse(gameModelDetail),
              overlayGlow: gameOverlayGlow,
              appearances: installed,
              models: modelsRef.current,
              instances: instancesRef.current,
              redraw: () => (dirty = true),
              failed: reportFailure,
            });
          } catch (error) {
            if (!current()) return build();
            throw error;
          }
          if (signal.aborted) {
            next.dispose();
            return next;
          }
          if (!current() || !next.worldCurrent()) {
            next.dispose();
            return build();
          }
          next.setModels(modelsRef.current);
          next.setInstances(instancesRef.current);
          const meshes = overlayRef.current?.current;
          if (meshes) next.setOverlay(meshes);
          const marks = pointerMarksRef.current?.current;
          if (marks) next.setPointerMarks(marks);
          return next;
        };
        let scene = await build();
        let staged: ViewportFrame | null = null;
        if (signal.aborted) return;
        sceneRef.current = scene;
        const allocations = () => scene.stats().memory;
        const adapter = info.name || info.backend;
        pilot?.attach?.({
          adapter,
          stats: () => ({ ...scene.stats(), receiverFit: receiverStats() }),
          resetGpuTiming: () => scene.resetGpuTiming(),
        });
        // What the battle draws, for a player's diagnostics report.
        signal.addEventListener(
          "abort",
          diagnosticsSource("battle view", () => ({
            adapter,
            canvas: [canvas.width, canvas.height],
            gpu: scene.stats().gpu,
            gpuAllocations: allocations(),
          })),
        );
        const draw = () => {
          // Cleared first: a frame may ask to be drawn again (`requestRedraw`).
          dirty = false;
          placeHigh();
          if (!scene.render()) {
            dirty = true;
            return false;
          }
          return true;
        };
        /** Whether the lab took the grass away (`suppressGrass`). */
        let grassSuppressed = false;
        /** Read back without grass, then restore the scene's grass setting. */
        const readPlain = () => {
          scene.grassProbes.suppress(true);
          scene.render();
          const pixels = scene.readPixels();
          scene.grassProbes.suppress(grassSuppressed);
          // Restore the visible canvas as well as the live controls.
          scene.render();
          return pixelsPng(pixels);
        };
        const counts = (): FrameCounts => ({
          drawn: scene.drawnInstances(),
          models: countsRef.current,
          effects: effectCountsRef.current,
        });
        // What the frame drew is read first: the grass-less draw changes the
        // frame, and may lay more dressing or expand more buildings.
        const plate = (): FramePlate => ({
          png: pixelsPng(scene.readPixels()),
          clumps: scene.grassProbes.clumps(),
          ...counts(),
          plain: readPlain(),
        });
        redrawRef.current = () => (dirty = true);
        let pointer: { x: number; y: number } | null = null;
        let modifiers = { ctrl: false, shift: false };
        const pointerRay = () => (pointer ? handle.rayAt!(pointer.x, pointer.y) : null);
        let lastFrame = performance.now();
        // eslint-disable-next-line complexity -- debt: its stages read and write the effect's mutable locals (scene, dirty, pointer, grassSuppressed, lastFrame); split needs those in one frame-state object the stages take, not more closures.
        const loop = (now: number) => {
          if (signal.aborted || handle.error) return;
          if (rebuilding) {
            raf = requestAnimationFrame(loop);
            return;
          }
          if (scene.presentation !== scenePresentation()) {
            void handle.rebuild?.().catch(reportFailure);
            raf = requestAnimationFrame(loop);
            return;
          }
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
          const focus = cameraFocusRef.current;
          if (focus && focus.token !== appliedFocus) {
            appliedFocus = focus.token;
            if (!piloted && inputEnabledRef.current) centerOn(focus.target);
          }
          steer({ held: keys.held, edge }, dt);
          const started = performance.now();
          if (framing) {
            place(controller.place(asked, framing));
            dirty = true; // a piloted frame is always drawn: it is measured
          }
          see(dt);
          // The scene's presentation is staged with its buildings before the
          // battle frames it (or, without one, the viewport frames it).
          const decoder = scene.presentation;
          const terrain = scene.world().terrain.mesh;
          let animated: ViewportFrame | null | undefined = null;
          try {
            if (scene.flush()) dirty = true;
            if (decoder) {
              decoder.setBuildings(
                buildingsRef.current?.current ?? null,
                scene.appearances(),
                staticStyle,
                staticSun,
              );
              const layers = scene.world();
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
            reportFailure(err);
            return;
          }
          if (animated) {
            staged = { ...staged, ...animated };
            if (animated.clock !== undefined) clock = animated.clock;
            dirty = true;
          }
          try {
            if (!animated?.presented)
              decoder.frame(
                clock,
                presentedCamera(snapshot()),
                { ...NO_UI, known: buildingsRef.current?.current?.known },
                scene.staticGeneration,
              );
            if (decoder.sceneryWalk().pending || decoder.sceneryWalk().falling) dirty = true;
          } catch (error) {
            reportFailure(error);
            return;
          }
          if (!scene.worldCurrent()) {
            void handle.rebuild?.().catch(reportFailure);
            raf = requestAnimationFrame(loop);
            return;
          }
          let drawn = false;
          try {
            const desiredWorld = worldRef.current.current;
            const updated =
              desiredWorld &&
              scene.updateSources(
                desiredWorld,
                appearancesRef.current,
                modelsRef.current,
                staged?.instances ?? instancesRef.current,
              );
            if (updated === "rebuild") {
              void handle.rebuild?.().catch(reportFailure);
              raf = requestAnimationFrame(loop);
              return;
            }
            if (updated === "reframe") {
              const reframed = frameRef.current?.(now, snapshot(), scene.staticGeneration);
              if (reframed) {
                animated = reframed;
                staged = { ...staged, ...reframed };
              }
              if (!reframed?.presented)
                decoder.frame(
                  clock,
                  presentedCamera(snapshot()),
                  { ...NO_UI, known: buildingsRef.current?.current?.known },
                  scene.staticGeneration,
                );
            }
            if (dirty && draw()) {
              drawn = true;
              if (staged?.instances) instancesRef.current = staged.instances;
              if (staged?.picks) picksRef.current = staged.picks;
              if (staged?.counts) countsRef.current = staged.counts;
              if (staged?.effectCounts) effectCountsRef.current = staged.effectCounts;
              submittedPicks =
                picksRef.current ?? fixedPicksRef.current ?? instancesRef.current.map(proxyPickBox);
              submittedView = snapshot();
              staged?.submitted?.();
              staged = null;
              if (scene.current()) frameRequests.submitted();
            }
          } catch (error) {
            reportFailure(error);
            return;
          }
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
          const renderScale = canvas.height / Math.max(1, canvas.clientHeight);
          const shownCamera = interactionView().camera3d;
          onFrameRef.current?.(project, shownCamera, pointerState, renderScale);
          const action =
            onCursorRef.current?.(pointerState, shownCamera, project, renderScale) ?? null;
          // The app cursor always shows; the battle only chooses its action.
          const active = surface && !orbit && !pilot;
          setCursorAction((active && action) || "default");
          pilot?.frame?.({
            now,
            cpuMs: performance.now() - started,
            receiverFit: receiverStats(),
            camera,
            simTime: clock,
            canvas: [canvas.width, canvas.height],
            counts,
            plate: plates ? plate : undefined,
          });
          raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);

        /** Project anchors with one canvas-layout read per frame. */
        const projector = (): Project =>
          viewportProjector(interactionView(), canvas.getBoundingClientRect());

        /** What a click can pick: the frame's picks, else the route's, else
         *  each drawn proxy by its own box. */
        const picks = (): readonly PickBox[] => submittedPicks;
        const nextFrame = () => frameRequests.request();

        Object.assign(handle, {
          adapter: info,
          stats: () => ({ ...scene.stats(), props: propStats(), receiverFit: receiverStats() }),
          resetGpuTiming: () => scene.resetGpuTiming(),
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
            rebuilding ??= (async () => {
              try {
                const previous = scene;
                const next = await build();
                if (signal.aborted) return;
                next.inheritControls(scene);
                scene = next;
                sceneRef.current = next;
                previous.dispose();
                setInspecting((shown) => (shown ? scene : shown));
                dirty = true;
              } catch (error) {
                reportFailure(error);
                throw error;
              } finally {
                rebuilding = null;
              }
            })();
            await rebuilding;
            if (!signal.aborted && !handle.error) await nextFrame();
          },
          rayAt(cssX: number, cssY: number) {
            const rect = canvas.getBoundingClientRect();
            const ndcX = ((cssX - rect.left) / rect.width) * 2 - 1;
            const ndcY = 1 - ((cssY - rect.top) / rect.height) * 2;
            return screenRay(createWorldRay(), liveCamera(interactionView()), ndcX, ndcY);
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
            scene.setControls({ view });
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
            scene.setControls({ scars: !on });
            await nextFrame();
          },
          async suppressFieldTexture(on: boolean) {
            scene.setControls({ fieldTexture: !on });
            await nextFrame();
          },
          async suppressModels(on: boolean) {
            scene.setControls({ models: !on });
            await nextFrame();
          },
          async suppressTrees(on: boolean) {
            scene.setControls({ trees: !on });
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
            scene.setControls({ buildings: !on });
            await nextFrame();
          },
          async suppressSurface(surface: SurfaceClass, on: boolean) {
            scene.setSurfaceShown(surface, !on);
            await nextFrame();
          },
          async suppressRoadWear(on: boolean) {
            scene.setControls({ roadWear: !on });
            await nextFrame();
          },
          async suppressEffects(on: boolean) {
            scene.setControls({ effects: !on });
            await nextFrame();
          },
          async suppressCastLights(on: boolean) {
            scene.setControls({ castLights: !on });
            await nextFrame();
          },
          async suppressXrayCoverage(on: boolean) {
            scene.setControls({ xrayCoverage: !on });
            await nextFrame();
          },
          async suppressPaint(on: boolean) {
            scene.setControls({ paint: !on });
            await nextFrame();
          },
          async setOverlayGlowStrength(strength: number | null) {
            scene.setOverlayGlow(
              strength === null ? gameOverlayGlow : { ...gameOverlayGlow, strength },
            );
            await nextFrame();
          },
        } satisfies Partial<LabHandle>);

        rebuildRef.current = () => {
          void handle.rebuild?.().catch(reportFailure);
        };

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
        await nextFrame();
        if (signal.aborted || handle.error) return;
        handle.ready = true;
        if (new URLSearchParams(window.location.search).has("inspect")) {
          setInspecting(scene);
        }
        setReady(true);
        onReadyRef.current?.({ allocations, gpu, frame: () => scene });
      } catch (err) {
        if (signal.aborted) return;
        appResources.refuse(err);
        const message = gpuFailureMessage(err);
        handle.error = message;
        setError(message);
      }
    })();

    return () => {
      onDisposeRef.current?.();
      lifetime.abort();
      rebuildRef.current = () => {};
      cancelInputRef.current = null;
      cancelAnimationFrame(raf);
      keys.detach();
      setCursorAction("default");
      onRightPressRef.current?.(null);
      sceneRef.current?.dispose();
      sceneRef.current = null;
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
