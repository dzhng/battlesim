import { act, cleanup, render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { LabViewport } from "@apps/battle-lab/src/LabViewport";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import type { BattleFrame, WorldLayers } from "@packages/battle-renderer/src/scene";

const gpu = vi.hoisted(() => ({
  resolve: null as null | ((frame: BattleFrame) => void),
  destroyed: false,
}));
vi.mock("@packages/renderer-core/src/device", () => ({
  requestGpuDevice: async () => ({
    device: {
      destroy: () => {
        gpu.destroyed = true;
      },
      createBuffer() {},
      createTexture() {},
    },
    format: "bgra8unorm",
  }),
  gpuFailureMessage: String,
}));
vi.mock("@packages/battle-renderer/src/frame/battleFrame", () => ({
  createBattleFrame: () =>
    new Promise<BattleFrame>((resolve) => {
      gpu.resolve = resolve;
    }),
}));
function mount(overrides: Partial<ComponentProps<typeof LabViewport>> = {}) {
  gpu.resolve = null;
  gpu.destroyed = false;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    configure() {},
  } as unknown as GPUCanvasContext);
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (frame: FrameRequestCallback) => frames.push(frame));
  vi.stubGlobal("cancelAnimationFrame", () => {});
  const props = {
    fixture: "lifetime",
    world: { current: {} as WorldLayers, subscribe: () => () => {} },
    initialCamera: gameCamera.opening(),
    ...overrides,
  };
  return { view: render(<LabViewport {...props} />), props, frames };
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

test("a viewport disposed during its build releases the late frame and never becomes ready", async () => {
  const listeners = vi.spyOn(window, "addEventListener");
  let ready = false;
  const { view, frames } = mount({
    onReady: () => {
      ready = true;
    },
  });
  await act(async () => {});
  expect(gpu.resolve).not.toBeNull();
  view.unmount();
  expect(gpu.destroyed).toBe(true);
  listeners.mockClear();
  let disposed = false;
  await act(async () =>
    gpu.resolve!({
      setFog() {
        throw new Error("late frame used after unmount");
      },
      setClock() {},
      setCorpses() {},
      setGround() {},
      dispose() {
        disposed = true;
      },
    } as unknown as BattleFrame),
  );
  expect(listeners.mock.calls.map(([name]) => name)).not.toContain("resize");
  expect(disposed).toBe(true);
  for (const frame of frames) frame(1);
  expect(ready).toBe(false);
});

test("failed frame initialization releases the frame before showing its error", async () => {
  const { view, props } = mount();
  await act(async () => {});
  let disposed = false;
  await act(async () =>
    gpu.resolve!({
      setFog() {
        throw new Error("frame setup failed");
      },
      setModels() {
        throw new Error("failed frame still receives props");
      },
      dispose() {
        disposed = true;
      },
    } as unknown as BattleFrame),
  );
  expect(view.getByRole("alert").textContent).toContain("frame setup failed");
  expect(disposed).toBe(true);
  view.rerender(<LabViewport {...props} models={[]} />);
  view.unmount();
});

test("appearance updates reach the installing frame and unmount releases it immediately", async () => {
  const appearance = (
    generation: number,
  ): NonNullable<ComponentProps<typeof LabViewport>["appearances"]> => ({
    generation,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    appearances: new Map(),
    onRequest: new Map(),
    skeletons: new Map(),
  });
  const first = appearance(1),
    latest = appearance(2);
  const { view, props } = mount({ appearances: first });
  await act(async () => {});
  let release!: () => void;
  const installing = new Promise<void>((resolve) => {
    release = resolve;
  });
  let requested = first;
  let disposed = false;
  const frame = {
    setFog() {},
    setModels() {},
    setClock() {},
    setCorpses() {},
    setGround() {},
    dispose() {
      disposed = true;
    },
    setAppearances(next: typeof first) {
      requested = next;
      return installing;
    },
  } as unknown as BattleFrame;
  await act(async () => {
    gpu.resolve!(frame);
  });
  view.rerender(<LabViewport {...props} appearances={latest} />);
  expect(requested).toBe(latest);
  view.unmount();
  expect(disposed).toBe(true);
  await act(async () => {
    release();
  });
});
