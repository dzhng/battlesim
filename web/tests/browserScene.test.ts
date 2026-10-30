import { afterEach, expect, test, vi } from "vitest";
import { proxyModels } from "@web/battle/render/proxyModels";
import { Renderer } from "@wasm/renderer_browser";
import { BrowserScene, type SceneOptions } from "@web/battle/render/scene";

vi.mock("@web/battle/render/effectAssets", () => ({ loadEffectAtlasLayers: async () => [] }));
vi.mock("@web/battle/render/sceneGeometry", () => ({ installSceneGeometry() {} }));
afterEach(() => vi.restoreAllMocks());

async function scene(installed: string[] = []) {
  const freed = () => {
    throw new Error("freed wasm handle");
  };
  const renderer = {
    load_effect_atlas() {},
    set_overlay_glow() {},
    async prepare_assets() {},
    set_gpu_timing() {},
    free() {},
    frame_stats: () => JSON.stringify({ models: { installed } }),
    drawn_instances: freed,
    static_generation: freed,
    read_rgba: freed,
    drain: freed,
    fog_shape: freed,
    fog_tile_counts: freed,
    grass_counts: freed,
  };
  vi.spyOn(Renderer, "new").mockResolvedValue(renderer as unknown as Renderer);
  const presentation = {
    canonicalAppearanceRows: () => [],
    presentedGround: () => null,
    installLabAssets() {},
    setLabModels() {},
    installRenderer() {},
    world: () => ({}),
    fogOccluders: freed,
    probeFog: freed,
    grassClumps: freed,
  };
  return BrowserScene.create({
    presentation,
    world: { terrain: { biome: { grass: {} } } },
    settings: {},
    detail: {},
    overlayGlow: {},
    appearances: null,
    models: [],
    instances: [],
    redraw() {},
    failed(error: unknown) {
      throw error;
    },
  } as unknown as SceneOptions);
}

test("disposed scene diagnostics refuse by name before touching freed WASM handles", async () => {
  const frame = await scene();
  frame.dispose();
  frame.dispose();
  expect(() => frame.stats()).toThrow(/BrowserSceneDisposed: stats/);
});

test("all scene queries and diagnostic probes reject a disposed owner by name", async () => {
  const frame = await scene();
  frame.dispose();
  const queries: Record<string, () => unknown> = {
    appearances: () => frame.appearances(),
    world: () => frame.world(),
    worldCurrent: () => frame.worldCurrent(),
    staticGeneration: () => frame.staticGeneration,
    current: () => frame.current(),
    readPixels: () => frame.readPixels(),
    bakeImpostor: () => frame.bakeImpostor("missing"),
    settled: () => frame.settled(),
    drain: () => frame.drain(),
    stats: () => frame.stats(),
    drawnInstances: () => frame.drawnInstances(),
    "fog.probe": () => frame.fogProbes.probe([]),
    "fog.probeShape": () => frame.fogProbes.probeShape(new Float32Array()),
    "fog.tileCounts": () => frame.fogProbes.tileCounts(),
    "grass.rules": () => frame.grassProbes.rules(),
    "grass.counts": () => frame.grassProbes.counts(),
    "grass.clumps": () => frame.grassProbes.clumps(),
  };
  for (const [name, query] of Object.entries(queries))
    expect(query, name).toThrow(`BrowserSceneDisposed: ${name}`);
  await expect(frame.fogProbes.wholes()).rejects.toThrow("BrowserSceneDisposed: fog.wholes");
  expect(frame.render()).toBe(false);
});

test("scene diagnostics publish proxy counts from the admitted appearance identities", async () => {
  const proxies = proxyModels([{ kind: "box", x: 0, y: 0, z: 0, yaw: 0, color: [1, 1, 1] }]);
  const frame = await scene(["authored-tank", proxies.assets[0].name]);
  try {
    expect(frame.stats().models.proxyAppearances).toBe(1);
  } finally {
    frame.dispose();
  }
});
