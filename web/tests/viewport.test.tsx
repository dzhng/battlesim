import { act, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { LabViewport } from "@apps/battle-lab/src/LabViewport";
import { villageCamera } from "@apps/battle-lab/src/villageCamera";
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
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

test("a viewport disposed during its build releases the late frame and never becomes ready", async () => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    configure() {},
  } as unknown as GPUCanvasContext);
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (frame: FrameRequestCallback) => frames.push(frame));
  vi.stubGlobal("cancelAnimationFrame", () => {});
  const listeners = vi.spyOn(window, "addEventListener");
  let ready = false;
  const view = render(
    <LabViewport
      fixture="lifetime"
      world={{ current: {} as WorldLayers, subscribe: () => () => {} }}
      initialCamera={villageCamera.opening()}
      onReady={() => {
        ready = true;
      }}
    />,
  );
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
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    configure() {},
  } as unknown as GPUCanvasContext);
  const view = render(
    <LabViewport
      fixture="failed-build"
      world={{ current: {} as WorldLayers, subscribe: () => () => {} }}
      initialCamera={villageCamera.opening()}
    />,
  );
  await act(async () => {});
  let disposed = false;
  await act(async () =>
    gpu.resolve!({
      setFog() {
        throw new Error("frame setup failed");
      },
      dispose() {
        disposed = true;
      },
    } as unknown as BattleFrame),
  );
  expect(view.getByRole("alert").textContent).toContain("frame setup failed");
  expect(disposed).toBe(true);
  view.unmount();
});
