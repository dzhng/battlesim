import { act, cleanup, fireEvent, render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, expect, test, vi } from "vitest";
import type { LabViewport } from "@apps/battle-lab/src/LabViewport";
import { Feed } from "@apps/battle-lab/src/feed";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import type { LoadingTasks } from "@apps/battle-lab/src/LabLoading";
import type { BattleFrame, WorldLayers } from "@packages/battle-renderer/src/scene";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";

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
// The frame is faked, so the light and fog look it would be built with need
// not be resolved through the presentation module's WASM.
vi.mock("@web/battle/sim/presentation", async (importOriginal) => ({
  ...(await importOriginal<object>()),
  resolveLight: async (light: unknown) => light,
  resolveFogLook: () => new Float32Array(0),
}));
vi.mock("@packages/battle-renderer/src/frame/battleFrame", () => ({
  createBattleFrame: () =>
    new Promise<BattleFrame>((resolve) => {
      gpu.resolve = resolve;
    }),
}));
async function mount(
  overrides: Partial<ComponentProps<typeof LabViewport>> = {},
  report: ComponentProps<typeof LoadingTasks>["report"] = () => {},
) {
  const { LabViewport } = await import("@apps/battle-lab/src/LabViewport");
  const { LoadingTasks } = await import("@apps/battle-lab/src/LabLoading");
  gpu.resolve = null;
  gpu.destroyed = false;
  const configure = vi.fn();
  vi.stubGlobal("GPUTextureUsage", { COPY_SRC: 1, RENDER_ATTACHMENT: 16 });
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    configure,
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
  // The lab loading boundary the viewport reports its readiness and refusal to.
  const view = render(<LabViewport {...props} />, {
    wrapper: ({ children }) => <LoadingTasks report={report}>{children}</LoadingTasks>,
  });
  return { view, props, frames, configure, Viewport: LabViewport };
}

/** Waits until the viewport is building its frame: it resolves the frame's
 *  light through the presentation module, which loads first. */
const building = () =>
  act(() => vi.waitFor(() => expect(gpu.resolve).not.toBeNull(), { timeout: 10_000 }));

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
  await building();
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
      setPresentationWorld() {},
      setPresentedGround() {},
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

test("failed frame initialization releases the frame and reports its error to the loading boundary", async () => {
  const reported: (string | null | undefined)[] = [];
  const { view, props, Viewport } = await mount({}, (id, task) => reported.push(task?.error));
  await building();
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
  // The boundary keeps the first refusal it hears and shows it in place of the page.
  expect(reported).toContainEqual(expect.stringContaining("frame setup failed"));
  expect(disposed).toBe(true);
  view.rerender(<Viewport {...props} models={[]} />);
  view.unmount();
});

test("appearance updates reach the installing frame and unmount releases it immediately", async () => {
  const appearance = (generation: number): InstalledAppearances => ({
    generation,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    appearances: new Map(),
    onRequest: new Map(),
    skeletons: new Map(),
  });
  const first = appearance(1),
    latest = appearance(2);
  const appearances = new Feed<InstalledAppearances | null>(first);
  const { view } = await mount({ appearances });
  await building();
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
    setPresentationWorld() {},
    setPresentedGround() {},
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
  act(() => appearances.set(latest));
  expect(requested).toBe(latest);
  view.unmount();
  expect(disposed).toBe(true);
  await act(async () => {
    release();
  });
});

test("a later viewport borrows the same device and releases its canvas without destroying admission", async () => {
  const first = await mount();
  await building();
  const native = HTMLCanvasElement.prototype.getContext as ReturnType<typeof vi.fn>;
  const context = native.mock.results.at(-1)!.value as GPUCanvasContext;
  first.view.unmount();
  expect(context.unconfigure).toHaveBeenCalledTimes(1);
  expect(gpu.destroyed).toBe(false);
  const second = await mount();
  await building();
  second.view.unmount();
  expect(gpu.destroyed).toBe(false);
});

test("a covered viewport ignores camera keys and resumes steering when uncovered", async () => {
  const { view, frames, props, Viewport } = await mount({ inputEnabled: false });
  await building();
  await act(async () =>
    gpu.resolve!({
      setFog() {},
      setClock() {},
      setPresentationWorld() {},
      setPresentedGround() {},
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

test("only a pilot that reads plates back makes the canvas copyable", async () => {
  const usage = async (pilot?: ComponentProps<typeof LabViewport>["pilot"]) => {
    const { view, configure } = await mount({ pilot });
    await act(async () => {});
    view.unmount();
    return configure.mock.calls[0][0].usage;
  };
  const pose = () => null;
  expect(await usage()).toBe(16);
  expect(await usage({ pose })).toBe(16);
  expect(await usage({ pose, plates: true })).toBe(16 | 1);
});
