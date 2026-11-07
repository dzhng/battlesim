import { act, render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { expect, vi } from "vitest";
import type { LabViewport } from "@apps/battle-lab/src/LabViewport";
import { Feed } from "@web/feed";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import type { LoadingTasks } from "@apps/battle-lab/src/LabLoading";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import type { SceneOptions } from "@web/battle/render/scene";

const gpu = vi.hoisted(() => ({
  resolve: null as null | ((overrides?: Record<string, unknown>) => void),
  reject: null as null | ((error: Error) => void),
  options: [] as SceneOptions[],
  device: { info: () => JSON.stringify({ name: "test GPU", backend: "WebGPU" }) },
  refused: [] as unknown[],
}));
vi.mock("@apps/battle-lab/src/appResources", () => ({
  gpuFailureMessage: String,
  appResources: {
    gpu: async () => gpu.device,
    refuse: (error: unknown) => gpu.refused.push(error),
  },
}));
vi.mock("@web/battle/sim/presentation", () => ({
  NO_PROP_STATS: {},
  ScenePresentation: class {
    battle = false;
    setBattle(battle: unknown) {
      this.battle = battle !== null;
    }
    hasBattle() {
      return this.battle;
    }
    setBuildings() {}
    setGround() {}
    setShadow() {}
    setFog() {}
    setDetailScale() {}
    setSceneryQuality() {}
    sceneryWalk() {
      return {};
    }
    frame() {}
    free() {}
    shadowStats() {
      return null;
    }
    propStats() {
      return {};
    }
  },
}));
// The viewport owns scene lifetime and camera/input. The GPU scene is the
// external boundary; browser scenes exercise its real Rust implementation.
vi.mock("@web/battle/render/scene", () => ({
  pixelsPng: () => Promise.resolve(new Blob()),
  BrowserScene: {
    create: (options: SceneOptions) => {
      gpu.options.push(options);
      return new Promise((resolve, reject) => {
        gpu.reject = reject;
        gpu.resolve = (overrides = {}) => {
          const scene = {
            presentation: options.presentation,
            world: () => options.world,
            worldCurrent: () => true,
            current: () => true,
            appearances: () => options.appearances,
            flush: () => false,
            inheritControls() {},
            staticGeneration: 0,
            setModels() {},
            setInstances() {},
            setOverlay() {},
            setPointerMarks() {},
            setFogStyle() {},
            resize() {},
            render: () => true,
            dispose() {},
            updateSources: () => "current",
            stats: () => ({ memory: { buffers: 0, textures: 0 } }),
            settled: async () => false,
            ...overrides,
          };
          resolve(scene);
        };
      });
    },
  },
}));
export async function mount(
  overrides: Partial<ComponentProps<typeof LabViewport>> = {},
  report: ComponentProps<typeof LoadingTasks>["report"] = () => {},
) {
  const { LabViewport } = await import("@apps/battle-lab/src/LabViewport");
  const { LoadingTasks } = await import("@apps/battle-lab/src/LabLoading");
  gpu.resolve = null;
  gpu.reject = null;
  gpu.options = [];
  gpu.refused = [];
  const frames = new Map<number, FrameRequestCallback>();
  let frameId = 0;
  vi.stubGlobal("requestAnimationFrame", (frame: FrameRequestCallback) => {
    frames.set(++frameId, frame);
    return frameId;
  });
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
  const props = {
    fixture: "lifetime",
    world: new Feed({
      terrain: { input: {}, mesh: new Float32Array(0) },
    } as unknown as WorldLayers),
    initialCamera: gameCamera.opening(),
    ...overrides,
  };
  const view = render(<LabViewport {...props} />, {
    wrapper: ({ children }) => <LoadingTasks report={report}>{children}</LoadingTasks>,
  });
  const advance = (now: number) => {
    const scheduled = [...frames.values()];
    frames.clear();
    for (const frame of scheduled) frame(now);
  };
  return { view, props, frames, advance, Viewport: LabViewport };
}
export const building = () =>
  act(() =>
    vi.waitFor(() => {
      expect(gpu.refused).toEqual([]);
      expect(gpu.resolve).not.toBeNull();
    }),
  );

export function getGpu() {
  return gpu;
}

export function pointer(type: string, init: MouseEventInit & { pointerId?: number } = {}) {
  const event = new MouseEvent(type, { bubbles: true, ...init });
  Object.defineProperty(event, "pointerId", { value: init.pointerId ?? 1 });
  return event;
}

export function canvasFor(view: ReturnType<typeof render>) {
  const canvas = view.container.querySelector("canvas")!;
  Object.defineProperty(canvas, "setPointerCapture", { value: vi.fn(), configurable: true });
  return canvas;
}
