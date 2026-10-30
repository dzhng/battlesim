import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { BrowserScene } from "@web/battle/render/scene";
import { loadPresentation } from "@web/battle/sim/presentation";
import { SheetRenderer } from "@apps/battle-lab/src/workbench/sheet";
import type { Bounds } from "@packages/scene-assets/src/schema";
import type { LoadedModel } from "@apps/battle-lab/src/workbench/sources";
import type { BrowserGpu } from "@wasm/renderer_browser";

await loadPresentation();
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
beforeEach(() => {
  vi.stubGlobal(
    "ImageData",
    class {
      constructor(
        readonly data: Uint8ClampedArray,
        readonly width: number,
        readonly height: number,
      ) {}
    },
  );
});

const bounds: Bounds = { min: [-1, -1, 0], max: [1, 1, 2] };
const model = {
  name: "test",
  body: {},
  installed: {
    appearances: new Map([
      ["test", { bundle: { kind: "static", states: [{ name: "rest", bounds }], bounds } }],
    ]),
  },
} as unknown as LoadedModel;

function frame(render: () => boolean) {
  return {
    setModels() {},
    setOverlay() {},
    setWorldProps() {},
    flush() {},
    dispose() {},
    staticGeneration: 0,
    render,
    readPixels: vi.fn(async (): Promise<{ rgba: Uint8Array; width: number; height: number }> => {
      throw new Error("previous tile was read");
    }),
  };
}

test("a sheet refuses an unavailable surface without reading the previous tile", async () => {
  let attempts = 0;
  const scene = frame(() => {
    if (++attempts > 8) throw new Error("surface retry did not stop");
    return false;
  });
  vi.spyOn(BrowserScene, "create").mockResolvedValue(scene as unknown as BrowserScene);
  const sheet = await SheetRenderer.create({} as BrowserGpu, model);
  try {
    await expect(
      sheet.tile(model, { kind: "static", state: "rest" }, "q-front", bounds, {
        hitBox: false,
        sockets: false,
      }),
    ).rejects.toThrow(/SheetSurfaceUnavailable/);
    expect(scene.readPixels).not.toHaveBeenCalled();
    expect(attempts).toBeGreaterThan(1);
  } finally {
    sheet.dispose();
  }
});

test("a sheet retries transient surface unavailability and returns the newly drawn view", async () => {
  let drawn = false;
  const render = vi.fn(() => {
    drawn = render.mock.calls.length > 1;
    return drawn;
  });
  const scene = frame(render);
  scene.readPixels.mockImplementation(async () => ({
    rgba: new Uint8Array(drawn ? [11, 22, 33, 255] : [99, 88, 77, 255]),
    width: 1,
    height: 1,
  }));
  vi.spyOn(BrowserScene, "create").mockResolvedValue(scene as unknown as BrowserScene);
  const sheet = await SheetRenderer.create({} as BrowserGpu, model);
  try {
    const tile = await sheet.tile(model, { kind: "static", state: "rest" }, "q-front", bounds, {
      hitBox: false,
      sockets: false,
    });
    expect([...tile.data]).toEqual([11, 22, 33, 255]);
    expect([tile.width, tile.height]).toEqual([1, 1]);
  } finally {
    sheet.dispose();
  }
});
