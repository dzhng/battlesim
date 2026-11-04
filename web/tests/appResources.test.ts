import { afterEach, expect, test, vi } from "vitest";
import { AppResources } from "@apps/battle-lab/src/appResources";

const { admit } = vi.hoisted(() => ({ admit: vi.fn() }));
vi.mock("@wasm/renderer_browser", () => ({ BrowserGpu: { new: admit } }));
vi.mock("@web/battle/sim/presentation", () => ({ loadPresentation: async () => {} }));

afterEach(() => vi.restoreAllMocks());

function hardware() {
  const admitted: object[] = [];
  let lose!: (reason: string) => void;
  let destroyed = 0;
  const device = {
    failure: () => failure,
    dispose: () => destroyed++,
    free: vi.fn(),
  };
  const failure = new Promise<string>((resolve) => {
    lose = resolve;
  });
  admit.mockImplementation(async () => {
    admitted.push(device);
    return device;
  });
  return { device, admitted, lose, destroyed: () => destroyed };
}

test("concurrent and later battle visits retain one admitted GPU until page departure", async () => {
  const native = hardware();
  const app = new AppResources();
  const [first, second] = await Promise.all([app.gpu(), app.gpu()]);
  expect(first).toBe(native.device);
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
  native.lose("WebGPU device lost: driver reset");
  await Promise.resolve();
  expect(app.error()).toContain("driver reset");
  await expect(app.gpu()).rejects.toThrow("driver reset");
  expect(native.admitted).toEqual([native.device]);
  app.dispose();
});

test("GPU admission that completes after departure disposes its owner without reopening the page", async () => {
  const native = hardware();
  let complete!: (gpu: object) => void;
  admit.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
  );
  const app = new AppResources();
  const arriving = app.gpu();
  await Promise.resolve();
  app.dispose();
  complete(native.device);
  await expect(arriving).rejects.toThrow("released");
  expect(native.destroyed()).toBe(1);
  await expect(app.gpu()).rejects.toThrow("Reload");
});
