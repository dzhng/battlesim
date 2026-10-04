import { act, cleanup, fireEvent, render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, expect, test, vi } from "vitest";
import type { LabViewport } from "@apps/battle-lab/src/LabViewport";
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
async function mount(overrides: Partial<ComponentProps<typeof LabViewport>> = {}) {
  const { LabViewport } = await import("@apps/battle-lab/src/LabViewport");
  gpu.resolve = null;
  gpu.destroyed = false;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    configure() {},
    getCurrentTexture: () => ({ createView: () => ({}) }),
    unconfigure: vi.fn(),
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
  return { view: render(<LabViewport {...props} />), props, frames, Viewport: LabViewport };
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetModules();
});

test("a viewport disposed during its build releases the late frame and never becomes ready", async () => {
  const listeners = vi.spyOn(window, "addEventListener");
  let ready = false;
  const { view, frames } = await mount({
    onReady: () => {
      ready = true;
    },
  });
  await act(async () => {});
  expect(gpu.resolve).not.toBeNull();
  view.unmount();
  expect(gpu.destroyed).toBe(false);
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
  const { view, props, Viewport } = await mount();
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
  view.rerender(<Viewport {...props} models={[]} />);
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
  const { view, props, Viewport } = await mount({ appearances: first });
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
  view.rerender(<Viewport {...props} appearances={latest} />);
  expect(requested).toBe(latest);
  view.unmount();
  expect(disposed).toBe(true);
  await act(async () => {
    release();
  });
});

test("a later viewport borrows the same device and releases its canvas without destroying admission", async () => {
  const first = await mount();
  await act(async () => {});
  const native = HTMLCanvasElement.prototype.getContext as ReturnType<typeof vi.fn>;
  const context = native.mock.results.at(-1)!.value as GPUCanvasContext;
  first.view.unmount();
  expect(context.unconfigure).toHaveBeenCalledTimes(1);
  expect(gpu.destroyed).toBe(false);
  const second = await mount();
  await act(async () => {});
  expect(gpu.resolve).not.toBeNull();
  second.view.unmount();
  expect(gpu.destroyed).toBe(false);
});

test("a covered viewport ignores camera keys and resumes steering when uncovered", async () => {
  const { view, frames, props, Viewport } = await mount({ inputEnabled: false });
  await act(async () => {});
  await act(async () =>
    gpu.resolve!({
      setFog() {},
      setClock() {},
      setCorpses() {},
      setGround: () => false,
      render() {},
      dispose() {},
    } as unknown as BattleFrame),
  );
  const advance = (now: number) => {
    const scheduled = frames.splice(0);
    for (const frame of scheduled) frame(now);
  };
  act(() => advance(performance.now() + 100));
  const before = [...window.__lab!.camera!().target];
  fireEvent.keyDown(window, { code: "KeyW" });
  act(() => advance(performance.now() + 200));
  expect(window.__lab!.camera!().target).toEqual(before);
  fireEvent.keyUp(window, { code: "KeyW" });
  view.rerender(<Viewport {...props} inputEnabled />);
  fireEvent.keyDown(window, { code: "KeyW" });
  act(() => advance(performance.now() + 300));
  expect(window.__lab!.camera!().target).not.toEqual(before);
  fireEvent.keyUp(window, { code: "KeyW" });
  view.unmount();
});
