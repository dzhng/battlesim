import { useEffect, useRef, useState } from "react";
import { requestGpuDevice, gpuFailureMessage } from "@packages/renderer-core/src/device";
import {
  trackGpuAllocations,
  type GpuAllocationCounts,
} from "@packages/renderer-core/src/gpuAllocations";
import { projectPoint, screenRay, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { liveCamera, type CameraSnapshot } from "@packages/renderer-core/src/cameraUniform";
import { orbitCamera, panCamera, zoomCamera } from "@packages/renderer-core/src/orbitRig";
import {
  createScene,
  type BattleScene,
  type SceneInstance,
} from "@packages/battle-renderer/src/scene";
import { pickInstance } from "@packages/battle-renderer/src/picking";
import type { MeshData } from "@packages/battle-renderer/src/proxies";

export interface LabViewportProps {
  fixture: string;
  mesh: MeshData;
  instances: readonly SceneInstance[];
  initialCamera: Camera3DParams;
  /** Called with the picked instance index (−1 for none). */
  onPick?: (index: number) => void;
}

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

export function LabViewport({ fixture, mesh, instances, initialCamera, onPick }: LabViewportProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState<string | null>(null);
  const instancesRef = useRef(instances);
  const sceneRef = useRef<BattleScene | null>(null);
  instancesRef.current = instances;

  useEffect(() => {
    sceneRef.current?.setInstances(instances);
  }, [instances]);

  useEffect(() => {
    const canvas = canvasRef.current!;
    const handle: LabHandle = { ready: false, fixture, error: null };
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
        const build = () => createScene(device!, info.format, mesh, instancesRef.current);
        let scene = await build();
        sceneRef.current = scene;

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
        const loop = () => {
          if (disposed) return;
          if (dirty) draw();
          raf = requestAnimationFrame(loop);
        };
        loop();

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
            await nextFrame();
          },
          pickAt(cssX: number, cssY: number) {
            const rect = canvas.getBoundingClientRect();
            const ndcX = ((cssX - rect.left) / rect.width) * 2 - 1;
            const ndcY = 1 - ((cssY - rect.top) / rect.height) * 2;
            return pickInstance(
              screenRay(liveCamera(snapshot()), ndcX, ndcY),
              instancesRef.current,
            );
          },
          instances: () => instancesRef.current,
          projectToCss(x: number, y: number, z: number) {
            const { ndc, clipW } = projectPoint(liveCamera(snapshot()), [x, y, z]);
            if (clipW <= 0) return null;
            const rect = canvas.getBoundingClientRect();
            return [
              rect.left + (ndc[0] * 0.5 + 0.5) * rect.width,
              rect.top + (0.5 - ndc[1] * 0.5) * rect.height,
            ];
          },
          frame: nextFrame,
        } satisfies Partial<LabHandle>);

        // Input (contracts.md controls): left click picks, middle drag orbits,
        // WASD pans, wheel zooms. Right button is reserved for orders.
        let drag: { x: number; y: number } | null = null;
        const onDown = (e: PointerEvent) => {
          if (e.button === 0) {
            onPick?.(handle.pickAt!(e.clientX, e.clientY));
          } else if (e.button === 1) {
            e.preventDefault();
            drag = { x: e.clientX, y: e.clientY };
            canvas.setPointerCapture(e.pointerId);
          }
        };
        const onMove = (e: PointerEvent) => {
          if (!drag) return;
          const dx = e.clientX - drag.x,
            dy = e.clientY - drag.y;
          drag.x = e.clientX;
          drag.y = e.clientY;
          const h = canvas.clientHeight || 1;
          camera = orbitCamera(camera, (-dx / h) * 3, (dy / h) * 2);
          dirty = true;
        };
        const onUp = () => (drag = null);
        const onWheel = (e: WheelEvent) => {
          e.preventDefault();
          camera = zoomCamera(camera, Math.exp(e.deltaY * 0.001));
          dirty = true;
        };
        const onKey = (e: KeyboardEvent) => {
          if (e.target instanceof HTMLInputElement) return;
          const step = 0.05;
          const moves: Record<string, [number, number]> = {
            w: [0, step],
            s: [0, -step],
            a: [-step, 0],
            d: [step, 0],
          };
          const m = moves[e.key.toLowerCase()];
          if (m) {
            camera = panCamera(camera, m[0], m[1]);
            dirty = true;
          }
        };
        const onResize = () => (dirty = true);
        canvas.addEventListener("pointerdown", onDown);
        canvas.addEventListener("pointermove", onMove);
        canvas.addEventListener("pointerup", onUp);
        canvas.addEventListener("wheel", onWheel, { passive: false });
        canvas.addEventListener("contextmenu", (e) => e.preventDefault());
        window.addEventListener("keydown", onKey);
        window.addEventListener("resize", onResize);
        cleanup.push(() => {
          canvas.removeEventListener("pointerdown", onDown);
          canvas.removeEventListener("pointermove", onMove);
          canvas.removeEventListener("pointerup", onUp);
          canvas.removeEventListener("wheel", onWheel);
          window.removeEventListener("keydown", onKey);
          window.removeEventListener("resize", onResize);
          scene.dispose();
        });
        handle.ready = true;
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
