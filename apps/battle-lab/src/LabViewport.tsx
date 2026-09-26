import { useEffect, useRef, useState } from "react";
import { requestGpuDevice, gpuFailureMessage } from "@packages/renderer-core/src/device";
import {
  trackGpuAllocations,
  type GpuAllocationCounts,
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
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type {
  CorpseInstance,
  ModelInstance,
} from "@packages/battle-renderer/src/models/modelInstances";
import { trackHeldKeys } from "@web/battle/input/heldKeys";
import { villageCamera } from "./villageCamera";
import { villageLight } from "./villageLight";
import { villageFogGeometry, villageFogStyle } from "./villageFog";
import { villageModelDetail } from "./villageModels";
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
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import { createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import { PassInspector } from "./PassInspector";
import { pickInstance } from "@packages/battle-renderer/src/picking";

export interface LabViewportProps {
  fixture: string;
  world: WorldLayers;
  /** Knowledge-drawn world geometry (standing buildings, remembered ruins and
   *  wrecks), lit and fogged with the world. */
  structures?: Mesh;
  /** Display-space marks drawn over the finished frame. */
  overlay?: WorldMeshes;
  /** What the observing side sees from, over the static world; omitted or
   *  null draws no fog. */
  fog?: FogInput | null;
  /** How unseen looks; the fixture's selected style when omitted. Live. */
  fogStyle?: FogStyle;
  /** The light, fixed for the viewport's life; the fixture's when omitted. */
  light?: LightPresentation;
  instances: readonly SceneInstance[];
  initialCamera: Camera3DParams;
  /** Ground height under a world point: the camera target rides it. */
  groundAt?: (x: number, y: number) => number;
  /** Left/right click: the picked instance index (−1 for none) and the camera ray. */
  onPick?: (pick: LabPick) => void;
  /** Left-drag rectangle in page CSS pixels, with the projection to test against it. */
  onBox?: (box: LabBox) => void;
  /** Called every animation frame; what it returns redraws the frame with it
   *  (presentation interpolation between completed ticks). */
  frame?: (now: number) => ViewportFrame | null;
  /** The first frame is on screen (the loading cover can lift), with the
   *  device's live GPU allocation counts. */
  onReady?: (gpu: ViewportGpu) => void;
  /** Called every animation frame with the live world → page projection and
   *  camera, for DOM readouts anchored to world points and panned sound. */
  onFrame?: (project: WorldToPage, camera: Camera3DParams) => void;
  /** Route-specific diagnostics published on `window.__lab.route`. */
  diagnostics?: Record<string, unknown>;
  /** A scripted driver (the benchmark), fixed for the viewport's life: it
   *  owns the camera, input no longer steers it, and every frame is drawn
   *  and reported with its cost. */
  pilot?: ViewportPilot;
  /** Appearance bundles from `AppearanceLibrary` for the models layer. */
  appearances?: InstalledAppearances | null;
  /** Posed models to draw. */
  models?: readonly ModelInstance[];
  /** The camera rig's numbers; the village's by default. */
  cameraConfig?: CameraPresentation;
}

/** One animation frame's drawn units. Anything omitted keeps its last value. */
export interface ViewportFrame {
  /** Proxies to draw (vehicles until their models land). */
  instances?: readonly SceneInstance[];
  /** What a click can pick, as boxes (every drawn unit and soldier); the
   *  drawn proxies when omitted. Picks report an index into this list. */
  picks?: readonly SceneInstance[];
  /** Posed models (soldiers). */
  models?: readonly ModelInstance[];
  /** The corpses: handed to the frame only when the array changes. */
  corpses?: readonly CorpseInstance[];
  /** The presentation clock, in seconds (the pose driver's and the wind's). */
  clock?: number;
}

/** The benchmark's hold on the viewport. */
export interface ViewportPilot {
  /** The battle frame is up: the adapter, and the frame's own statistics
   *  (its rolling GPU frame time from `timestamp-query`, and live memory). */
  attach(frame: { adapter: string; stats: () => ReturnType<BattleFrame["stats"]> }): void;
  /** The framing for the frame at `now`; the rig places it (`CameraController.place`). */
  pose(now: number): CameraPose;
  /** A drawn frame: its main-thread cost and the camera drawn. */
  frame(f: { now: number; cpuMs: number; camera: Camera3DParams }): void;
}

/** What the viewport's device reports once it is up. */
export interface ViewportGpu {
  allocations: () => GpuAllocationCounts;
  /** The device and canvas format, for work beside the viewport (model sheets). */
  device: GPUDevice;
  format: GPUTextureFormat;
  /** The live battle frame (it is rebuilt by `rebuild`). */
  frame: () => BattleFrame;
}

/** World point → page CSS pixel, or null when behind the eye. */
export type WorldToPage = (x: number, y: number, z: number) => [number, number] | null;

const _projector_world = vec3.create();
const _projector_point = createProjectedPoint();

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

/** Diagnostic hooks the scene harness reads; lab-only, never on a player route. */
export interface LabHandle {
  ready: boolean;
  fixture: string;
  error: string | null;
  adapter?: { vendor: string; architecture: string; description: string; format: string };
  stats?: () => ReturnType<BattleFrame["stats"]>;
  allocations?: () => GpuAllocationCounts;
  camera?: () => Camera3DParams;
  setCamera?: (camera: Camera3DParams) => void;
  reset?: () => void;
  /** Rebuild the scene from scratch (reset/dispose cycle) and resolve when drawn. */
  rebuild?: () => Promise<void>;
  pickAt?: (cssX: number, cssY: number) => number;
  rayAt?: (cssX: number, cssY: number) => WorldRay;
  route?: Record<string, unknown>;
  /** What a click can pick: every drawn unit and soldier, as boxes. */
  instances?: () => readonly SceneInstance[];
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
  /** Draw no models or corpses while on (a paired cost measure). */
  suppressModels?: (on: boolean) => Promise<void>;
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
  world,
  structures,
  overlay,
  fog,
  fogStyle,
  light,
  instances,
  initialCamera,
  groundAt,
  onPick,
  onBox,
  frame,
  onFrame,
  onReady,
  diagnostics,
  pilot,
  appearances,
  models,
  cameraConfig,
}: LabViewportProps) {
  const pilotRef = useRef(pilot);
  const cameraConfigRef = useRef(cameraConfig);
  const appearancesRef = useRef(appearances);
  appearancesRef.current = appearances;
  const modelsRef = useRef(models);
  modelsRef.current = models;
  const corpsesRef = useRef<readonly CorpseInstance[]>([]);
  const modelsSuppressed = useRef(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [box, setBox] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const onBoxRef = useRef(onBox);
  onBoxRef.current = onBox;
  const frameRef = useRef(frame);
  frameRef.current = frame;
  const onFrameRef = useRef(onFrame);
  onFrameRef.current = onFrame;
  const instancesRef = useRef(instances);
  const picksRef = useRef<readonly SceneInstance[] | null>(null);
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
  useEffect(() => {
    sceneRef.current?.setWorld(world);
    redrawRef.current();
  }, [world]);
  const overlayRef = useRef(overlay);
  overlayRef.current = overlay;
  const structuresRef = useRef(structures);
  structuresRef.current = structures;
  const fogRef = useRef(fog);
  fogRef.current = fog;
  const fogStyleRef = useRef(fogStyle);
  fogStyleRef.current = fogStyle;
  const lightRef = useRef(light);
  useEffect(() => {
    sceneRef.current?.setFogStyle(fogStyle ?? villageFogStyle);
    redrawRef.current();
  }, [fogStyle]);
  const fogSuppressed = useRef(false);
  useEffect(() => {
    sceneRef.current?.setFog(fogSuppressed.current ? null : (fog ?? null));
    redrawRef.current();
  }, [fog]);
  useEffect(() => {
    if (overlay) sceneRef.current?.setOverlay(overlay);
    redrawRef.current();
  }, [overlay]);
  useEffect(() => {
    if (structures) sceneRef.current?.setStructures(structures);
    redrawRef.current();
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
    const canvas = canvasRef.current!;
    const handle: LabHandle = {
      ready: false,
      fixture,
      error: null,
      route: diagnosticsRef.current,
    };
    const initialCamera = initialCameraRef.current;
    window.__lab = handle;
    let disposed = false;
    let device: GPUDevice | null = null;
    let raf = 0;
    let camera = initialCamera;
    // Without a ground, the target keeps its height.
    const controller = new CameraController(
      cameraConfigRef.current ?? villageCamera.config,
      (x, y) => (groundAtRef.current ? groundAtRef.current(x, y) : camera.target[2]),
    );
    const keys = trackHeldKeys(window, (code) => code in CAMERA_KEYS);
    const pilot = pilotRef.current;
    /** Apply one intent; redraw only when the camera moved. A pilot's camera
     *  takes no input. */
    const steer = (intent: CameraIntent, dt: number) => {
      if (pilot) return;
      const next = controller.step(camera, intent, dt);
      if (next !== camera) {
        camera = next;
        dirty = true;
      }
    };
    let dirty = true;
    const cleanup: (() => void)[] = [() => keys.detach()];

    const snapshot = (): ViewportCamera => ({
      camera3d: camera,
      width: canvas.width,
      height: canvas.height,
    });

    (async () => {
      try {
        const info = await requestGpuDevice();
        device = info.device;
        if (disposed) {
          device.destroy();
          return;
        }
        const allocations = trackGpuAllocations(device);
        const context = canvas.getContext("webgpu");
        if (!context) throw new Error("Canvas refused a WebGPU context.");
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
            light: lightRef.current ?? villageLight,
            fogGeometry: villageFogGeometry,
            fogStyle: fogStyleRef.current ?? villageFogStyle,
            models: villageModelDetail,
            world: worldRef.current,
            instances: instancesRef.current,
            width: canvas.width,
            height: canvas.height,
            requestRedraw: () => (dirty = true),
          });
          if (structuresRef.current) next.setStructures(structuresRef.current);
          if (overlayRef.current) next.setOverlay(overlayRef.current);
          next.setFog(fogRef.current ?? null);
          if (appearancesRef.current) await next.setAppearances(appearancesRef.current);
          if (modelsRef.current) next.setModels(modelsRef.current);
          next.setClock(clock);
          next.setCorpses(corpsesRef.current);
          sceneRef.current = next;
          return next;
        };
        let scene = await build();
        pilot?.attach({
          adapter: info.description || `${info.vendor} ${info.architecture}`.trim(),
          stats: () => scene.stats(),
        });
        const draw = () => {
          syncSize();
          scene.render(context.getCurrentTexture().createView(), snapshot());
          dirty = false;
        };
        redrawRef.current = () => (dirty = true);
        let pointer: { x: number; y: number } | null = null;
        let lastFrame = performance.now();
        const loop = (now: number) => {
          if (disposed) return;
          const dt = Math.min(0.1, (now - lastFrame) / 1000);
          lastFrame = now;
          // Held camera keys, and the pointer resting at the canvas border.
          let edge: [number, number] = [0, 0];
          if (pointer) {
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
          steer({ held: keys.held, edge }, dt);
          const started = performance.now();
          if (pilot) {
            camera = controller.place(camera, pilot.pose(now));
            dirty = true; // a piloted frame is always drawn: it is measured
          }
          const animated = frameRef.current?.(now);
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
            if (animated.models) {
              modelsRef.current = animated.models;
              if (!modelsSuppressed.current) scene.setModels(animated.models);
            }
            if (animated.corpses && animated.corpses !== corpsesRef.current) {
              corpsesRef.current = animated.corpses;
              if (!modelsSuppressed.current) scene.setCorpses(animated.corpses);
            }
            dirty = true;
          }
          if (dirty) draw();
          onFrameRef.current?.(projector(), camera);
          pilot?.frame({ now, cpuMs: performance.now() - started, camera });
          raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);

        /** World → page projection for the current camera and canvas box,
         *  read once so a whole frame of anchors costs one layout read. */
        const projector = (): WorldToPage => {
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

        /** What a click can pick: the frame's picks, else the drawn proxies. */
        const picks = () => picksRef.current ?? instancesRef.current;
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
            camera = next;
            dirty = true;
          },
          reset() {
            camera = initialCamera;
            dirty = true;
          },
          async rebuild() {
            scene.dispose();
            scene = await build();
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
            return pickInstance(handle.rayAt!(cssX, cssY), picks());
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
            scene.setFog(on ? null : (fogRef.current ?? null));
            await nextFrame();
          },
          async suppressModels(on: boolean) {
            modelsSuppressed.current = on;
            scene.setModels(on ? [] : (modelsRef.current ?? []));
            scene.setCorpses(on ? [] : corpsesRef.current);
            await nextFrame();
          },
          timePoseKernel: (reps: number, bodies?: number) => scene.timePoseKernel(reps, bodies),
        } satisfies Partial<LabHandle>);

        // Input: left click selects, left drag box-selects, right click
        // orders; the camera (CameraController) takes held WASD/arrows and the
        // screen edge to pan, Q/E to turn, middle drag to orbit, the wheel to zoom.
        let orbit: { x: number; y: number } | null = null;
        let press: { x: number; y: number; shift: boolean } | null = null;
        const pick = (e: PointerEvent, button: "left" | "right") => {
          const ray = handle.rayAt!(e.clientX, e.clientY);
          onPickRef.current?.({
            instance: pickInstance(ray, picks()),
            ray,
            button,
            shift: e.shiftKey,
            ctrl: e.ctrlKey,
            x: e.clientX,
            y: e.clientY,
            time: e.timeStamp,
          });
        };
        const onDown = (e: PointerEvent) => {
          if (e.button === 0) {
            press = { x: e.clientX, y: e.clientY, shift: e.shiftKey };
            canvas.setPointerCapture(e.pointerId);
          } else if (e.button === 2) {
            pick(e, "right");
          } else if (e.button === 1) {
            e.preventDefault();
            orbit = { x: e.clientX, y: e.clientY };
            canvas.setPointerCapture(e.pointerId);
          }
        };
        const onMove = (e: PointerEvent) => {
          pointer = { x: e.clientX, y: e.clientY };
          if (press && Math.hypot(e.clientX - press.x, e.clientY - press.y) > CLICK_SLOP_PX) {
            setBox({ x0: press.x, y0: press.y, x1: e.clientX, y1: e.clientY });
          }
          if (!orbit) return;
          const dx = e.clientX - orbit.x,
            dy = e.clientY - orbit.y;
          orbit.x = e.clientX;
          orbit.y = e.clientY;
          const h = canvas.clientHeight || 1;
          steer({ drag: [dx / h, dy / h] }, 0);
        };
        const onUp = (e: PointerEvent) => {
          orbit = null;
          if (e.button !== 0 || !press) return;
          const start = press;
          press = null;
          setBox(null);
          if (Math.hypot(e.clientX - start.x, e.clientY - start.y) <= CLICK_SLOP_PX) {
            pick(e, "left");
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
        const onLeave = () => (pointer = null);
        const onWheel = (e: WheelEvent) => {
          e.preventDefault();
          steer({ wheel: e.deltaY }, 0);
        };
        const onResize = () => (dirty = true);
        canvas.addEventListener("pointerdown", onDown);
        canvas.addEventListener("pointermove", onMove);
        canvas.addEventListener("pointerup", onUp);
        canvas.addEventListener("pointerleave", onLeave);
        canvas.addEventListener("wheel", onWheel, { passive: false });
        canvas.addEventListener("contextmenu", (e) => e.preventDefault());
        window.addEventListener("resize", onResize);
        cleanup.push(() => {
          canvas.removeEventListener("pointerdown", onDown);
          canvas.removeEventListener("pointermove", onMove);
          canvas.removeEventListener("pointerup", onUp);
          canvas.removeEventListener("pointerleave", onLeave);
          canvas.removeEventListener("wheel", onWheel);
          window.removeEventListener("resize", onResize);
          scene.dispose();
        });
        handle.ready = true;
        if (new URLSearchParams(window.location.search).has("inspect")) {
          setInspecting(scene);
        }
        requestAnimationFrame(() =>
          onReadyRef.current?.({
            allocations,
            device: info.device,
            format: info.format,
            frame: () => scene,
          }),
        );
      } catch (err) {
        const message = gpuFailureMessage(err);
        handle.error = message;
        setError(message);
      }
    })();

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      for (const fn of cleanup) fn();
      sceneRef.current = null;
      device?.destroy();
      if (window.__lab === handle) delete window.__lab;
    };
    // The viewport is rebuilt only when the fixture identity changes.
  }, [fixture]);

  return (
    <div style={{ position: "absolute", inset: 0 }}>
      <canvas ref={canvasRef} style={{ width: "100%", height: "100%", display: "block" }} />
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
      {error && (
        <div
          role="alert"
          style={{
            position: "absolute",
            inset: 0,
            display: "grid",
            placeItems: "center",
            background: "#15171c",
          }}
        >
          <div style={{ maxWidth: 520, padding: 24 }}>
            <strong>WebGPU unavailable.</strong>
            <p>{error}</p>
          </div>
        </div>
      )}
    </div>
  );
}
