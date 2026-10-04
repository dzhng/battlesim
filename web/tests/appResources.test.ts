import { afterEach, expect, test, vi } from "vitest";
import { AppResources } from "@apps/battle-lab/src/appResources";

afterEach(() => vi.unstubAllGlobals());

function hardware() {
  const admitted: object[] = [];
  let lose!: (loss: GPUDeviceLostInfo) => void;
  let destroyed = 0;
  const device = {
    features: new Set(),
    limits: {},
    lost: new Promise<GPUDeviceLostInfo>((resolve) => {
      lose = resolve;
    }),
    destroy: () => destroyed++,
  };
  const adapter = {
    features: new Set(),
    limits: {},
    info: {},
    requestDevice: async () => {
      admitted.push(device);
      return device;
    },
  };
  vi.stubGlobal("navigator", {
    gpu: { requestAdapter: async () => adapter, getPreferredCanvasFormat: () => "bgra8unorm" },
  });
  return { device, admitted, lose, destroyed: () => destroyed };
}

test("concurrent and later battle visits retain one admitted GPU until page departure", async () => {
  const native = hardware();
  const app = new AppResources();
  const [first, second] = await Promise.all([app.gpu(), app.gpu()]);
  expect(first.device).toBe(native.device);
  expect(second).toBe(first);
  expect(await app.gpu()).toBe(first);
  expect(native.admitted).toEqual([native.device]);
  expect(native.destroyed()).toBe(0);
  app.dispose();
  app.dispose();
  expect(native.destroyed()).toBe(1);
  await expect(app.gpu()).rejects.toThrow("Reload");
});

test("a lost device reports refusal and is never returned to a later visit", async () => {
  const native = hardware();
  const app = new AppResources();
  await app.gpu();
  native.lose({ reason: "unknown", message: "driver reset" } as GPUDeviceLostInfo);
  await Promise.resolve();
  expect(app.error()).toContain("driver reset");
  await expect(app.gpu()).rejects.toThrow("driver reset");
  expect(native.admitted).toEqual([native.device]);
  app.dispose();
});
