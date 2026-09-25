import { useEffect, useRef, useState } from "react";
import { requestGpuDevice, gpuFailureMessage } from "@packages/renderer-core/src/device";
import {
  trackGpuAllocations,
  type GpuAllocationCounts,
} from "@packages/renderer-core/src/gpuAllocations";
import {
  projectPoint,
  screenRay,
  type Camera3DParams,
  type WorldRay,
} from "@packages/renderer-core/src/camera3d";
import { liveCamera, type CameraSnapshot } from "@packages/renderer-core/src/cameraUniform";
import { orbitCamera, panCamera, zoomCamera } from "@packages/renderer-core/src/orbitRig";
import {
  createScene,
  type BattleScene,
  type FogField,
  type SceneInstance,
  type WorldMeshes,
} from "@packages/battle-renderer/src/scene";
import { pickInstance } from "@packages/battle-renderer/src/picking";

export interface LabViewportProps {
  fixture: string;
  world: WorldMeshes;
  /** Dynamic presentation geometry drawn over the world. */
  overlay?: WorldMeshes;
  /** The observing side's ground visibility; omitted or null draws no fog. */
  fog?: FogField | null;
  instances: readonly SceneInstance[];
  initialCamera: Camera3DParams;
  /** Left/right click: the picked instance index (−1 for none) and the camera ray. */
  onPick?: (pick: LabPick) => void;
  /** Left-drag rectangle in page CSS pixels, with the projection to test against it. */
  onBox?: (box: LabBox) => void;
  /** Called every animation frame; returning instances redraws with them
   *  (presentation interpolation between completed ticks). */
  frameInstances?: (now: number) => readonly SceneInstance[] | null;
  /** The first frame is on screen (the loading cover can lift). */
  onReady?: () => void;
  /** Called every animation frame with the live world → page projection and
   *  camera distance, for DOM readouts anchored to world points. */
  onFrame?: (project: WorldToPage, distance: number) => void;
  /** Route-specific diagnostics published on `window.__lab.route`. */
  diagnostics?: Record<string, unknown>;
}

/** World point → page CSS pixel, or null when behind the eye. */
export type WorldToPage = (x: number, y: number, z: number) => [number, number] | null;

export interface LabPick {
  instance: number;
  ray: WorldRay;
  button: "left" | "right";
  shift: boolean;
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
/** Screen-edge band that pans the camera, and its speed (view heights per second). */
const EDGE_PAN_PX = 14;
const EDGE_PAN_RATE = 0.6;
const KEY_PAN_STEP = 0.05;

/** Diagnostic hooks the scene harness reads; lab-only, never on a player route. */
export interface LabHandle {
  ready: boolean;
  fixture: string;
  error: string | null;
  adapter?: { vendor: string; architecture: string; description: string; format: string };
  stats?: () => ReturnType<BattleScene["stats"]>;
  allocations?: () => GpuAllocationCounts;
  camera?: () => Camera3DParams;
  setCamera?: (camera: Camera3DParams) => void;
  reset?: () => void;
  /** Rebuild the scene from scratch (reset/dispose cycle) and resolve when drawn. */
  rebuild?: () => Promise<void>;
  pickAt?: (cssX: number, cssY: number) => number;
  rayAt?: (cssX: number, cssY: number) => WorldRay;
  route?: Record<string, unknown>;
  instances?: () => readonly SceneInstance[];
  /** World point → CSS pixel in the page, or null when behind the eye. */
  projectToCss?: (x: number, y: number, z: number) => [number, number] | null;
  frame?: () => Promise<void>;
}

declare global {
  interface Window {
    __lab?: LabHandle;
  }
}

export function LabViewport({
  fixture,
  world,
  overlay,
  fog,
  instances,
  initialCamera,
  onPick,
  onBox,
  frameInstances,
  onFrame,
  onReady,
  diagnostics,
}: LabViewportProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [box, setBox] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const onBoxRef = useRef(onBox);
  onBoxRef.current = onBox;
  const frameInstancesRef = useRef(frameInstances);
  frameInstancesRef.current = frameInstances;
  const onFrameRef = useRef(onFrame);
  onFrameRef.current = onFrame;
  const instancesRef = useRef(instances);
  const worldRef = useRef(world);
  worldRef.current = world;
  const sceneRef = useRef<BattleScene | null>(null);
  instancesRef.current = instances;

  const redrawRef = useRef<() => void>(() => {});
  const onPickRef = useRef(onPick);
  onPickRef.current = onPick;
  const onReadyRef = useRef(onReady);
  onReadyRef.current = onReady;
  const diagnosticsRef = useRef(diagnostics);
  diagnosticsRef.current = diagnostics;
  const initialCameraRef = useRef(initialCamera);
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
  const fogRef = useRef(fog);
  fogRef.current = fog;
  useEffect(() => {
    sceneRef.current?.setFog(fog ?? null);
    redrawRef.current();
  }, [fog]);
  useEffect(() => {
    if (overlay) sceneRef.current?.setOverlay(overlay);
    redrawRef.current();
  }, [overlay]);

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
    let dirty = true;
    const cleanup: (() => void)[] = [];

    const snapshot = (): CameraSnapshot => ({
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
        const build = () =>
          createScene(device!, info.format, worldRef.current, instancesRef.current);
        let scene = await build();
        sceneRef.current = scene;
        if (overlayRef.current) scene.setOverlay(overlayRef.current);
        scene.setFog(fogRef.current ?? null);

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
          if (pointer) {
            // Screen-edge panning while the pointer rests at the canvas border.
            const rect = canvas.getBoundingClientRect();
            const ex =
              pointer.x - rect.left < EDGE_PAN_PX
                ? -1
                : rect.right - pointer.x < EDGE_PAN_PX
                  ? 1
                  : 0;
            const ey =
              pointer.y - rect.top < EDGE_PAN_PX
                ? 1
                : rect.bottom - pointer.y < EDGE_PAN_PX
                  ? -1
                  : 0;
            if (ex || ey) {
              camera = panCamera(camera, ex * EDGE_PAN_RATE * dt, ey * EDGE_PAN_RATE * dt);
              dirty = true;
            }
          }
          const animated = frameInstancesRef.current?.(now);
          if (animated) {
            instancesRef.current = animated;
            scene.setInstances(animated);
            dirty = true;
          }
          if (dirty) draw();
          onFrameRef.current?.(projector(), camera.distance);
          raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);

        /** World → page projection for the current camera and canvas box,
         *  read once so a whole frame of anchors costs one layout read. */
        const projector = (): WorldToPage => {
          const view = liveCamera(snapshot());
          const rect = canvas.getBoundingClientRect();
          return (x, y, z) => {
            const { ndc, clipW } = projectPoint(view, [x, y, z]);
            if (clipW <= 0) return null;
            return [
              rect.left + (ndc[0] * 0.5 + 0.5) * rect.width,
              rect.top + (0.5 - ndc[1] * 0.5) * rect.height,
            ];
          };
        };

        const nextFrame = () =>
          new Promise<void>((resolve) => {
            dirty = true;
            requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
          });

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
            sceneRef.current = scene;
            if (overlayRef.current) scene.setOverlay(overlayRef.current);
            scene.setFog(fogRef.current ?? null);
            await nextFrame();
          },
          rayAt(cssX: number, cssY: number) {
            const rect = canvas.getBoundingClientRect();
            const ndcX = ((cssX - rect.left) / rect.width) * 2 - 1;
            const ndcY = 1 - ((cssY - rect.top) / rect.height) * 2;
            return screenRay(liveCamera(snapshot()), ndcX, ndcY);
          },
          pickAt(cssX: number, cssY: number) {
            return pickInstance(handle.rayAt!(cssX, cssY), instancesRef.current);
          },
          instances: () => instancesRef.current,
          projectToCss(x: number, y: number, z: number) {
            return projector()(x, y, z);
          },
          frame: nextFrame,
        } satisfies Partial<LabHandle>);

        // Input (contracts.md controls): left click selects, left drag box-
        // selects, right click orders, middle drag orbits, arrow keys and the
        // screen edge pan, wheel zooms.
        let orbit: { x: number; y: number } | null = null;
        let press: { x: number; y: number; shift: boolean } | null = null;
        const pick = (e: PointerEvent, button: "left" | "right") => {
          const ray = handle.rayAt!(e.clientX, e.clientY);
          onPickRef.current?.({
            instance: pickInstance(ray, instancesRef.current),
            ray,
            button,
            shift: e.shiftKey,
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
          camera = orbitCamera(camera, (-dx / h) * 3, (dy / h) * 2);
          dirty = true;
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
          camera = zoomCamera(camera, Math.exp(e.deltaY * 0.001));
          dirty = true;
        };
        const onKey = (e: KeyboardEvent) => {
          if (e.target instanceof HTMLInputElement) return;
          const moves: Record<string, [number, number]> = {
            ArrowUp: [0, KEY_PAN_STEP],
            ArrowDown: [0, -KEY_PAN_STEP],
            ArrowLeft: [-KEY_PAN_STEP, 0],
            ArrowRight: [KEY_PAN_STEP, 0],
          };
          const m = moves[e.key];
          if (m) {
            e.preventDefault();
            camera = panCamera(camera, m[0], m[1]);
            dirty = true;
          }
        };
        const onResize = () => (dirty = true);
        canvas.addEventListener("pointerdown", onDown);
        canvas.addEventListener("pointermove", onMove);
        canvas.addEventListener("pointerup", onUp);
        canvas.addEventListener("pointerleave", onLeave);
        canvas.addEventListener("wheel", onWheel, { passive: false });
        canvas.addEventListener("contextmenu", (e) => e.preventDefault());
        window.addEventListener("keydown", onKey);
        window.addEventListener("resize", onResize);
        cleanup.push(() => {
          canvas.removeEventListener("pointerdown", onDown);
          canvas.removeEventListener("pointermove", onMove);
          canvas.removeEventListener("pointerup", onUp);
          canvas.removeEventListener("pointerleave", onLeave);
          canvas.removeEventListener("wheel", onWheel);
          window.removeEventListener("keydown", onKey);
          window.removeEventListener("resize", onResize);
          scene.dispose();
        });
        handle.ready = true;
        requestAnimationFrame(() => onReadyRef.current?.());
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
