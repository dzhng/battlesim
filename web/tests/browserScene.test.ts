import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { joinTextures } from "@packages/scene-assets/src/codec";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { LabAsset, ModelAsset } from "@web/battle/render/modelAssets";
import type { PreparedAppearances } from "@wasm/renderer_browser";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { proxyModels } from "@web/battle/render/proxyModels";
import { Renderer } from "@wasm/renderer_browser";
import { BrowserScene, type SceneOptions } from "@web/battle/render/scene";
import { loadPresentationModule } from "../src/presentationModule";

beforeAll(async () => {
  await loadPresentationModule();
});

vi.mock("@web/battle/render/effectAssets", () => ({ loadEffectAtlasLayers: async () => [] }));
afterEach(() => vi.restoreAllMocks());

async function scene(
  installed: string[] = [],
  options: Partial<SceneOptions> = {},
  seam: Partial<Renderer> = {},
) {
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
    ...seam,
  };
  vi.spyOn(Renderer, "new").mockResolvedValue(renderer as unknown as Renderer);
  const presentation = {
    canonicalAppearanceRows: () => [],
    presentedGround: () => null,
    installLabAssets(assets: Iterable<LabAsset>) {
      Array.from(assets);
    },
    setLabModels() {},
    installGeometry() {},
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
    ...options,
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

for (const first of ["art", "battle"]) {
  test(`a growth requests once when ${first} arrives a frame before the other input`, async () => {
    const generated = proxyModels([{ kind: "box", x: 0, y: 0, z: 0, yaw: 0, color: [1, 1, 1] }]);
    const bundle = joinTextures(generated.assets[0].encode().bundle, []);
    if (bundle.kind !== "articulated") throw new Error("expected articulated fixture");
    const art = (names: string[]): InstalledAppearances => ({
      generation: names.length,
      sides: { blue: [1, 1, 1], red: [1, 1, 1] },
      skeletons: new Map(),
      onRequest: new Map(),
      appearances: new Map(
        names.map((name, i) => [
          name,
          {
            contentId: { bundle: String(i + 1).repeat(64), skeleton: null },
            bundle,
            unit: "vehicle",
            scenery: null,
            footprint: null,
            mounts: null,
            regionalFamily: null,
            paints: null,
            wreck: null,
            factions: null,
          },
        ]),
      ),
    });
    const initial = art(["A"]),
      grown = art(["A", "B"]);
    const prepared: string[][] = [];
    let staged = 0;
    const worldRows = [{ name: "A" }];
    const presentation = {
      canonicalAppearanceRows: () => worldRows,
      presentedGround: () => null,
      installLabAssets(assets: Iterable<LabAsset>) {
        Array.from(assets);
      },
      setLabModels() {},
      installGeometry() {},
      installRenderer() {},
      world: () => worldRows,
      stagedBattles: () => staged,
      stagedAppearances: () => ["A", "B"],
      prepareRendererAppearances(_renderer: Renderer, assets: ModelAsset[]) {
        prepared.push(assets.map((a) => a.name));
        return Promise.resolve({ free() {} } as PreparedAppearances);
      },
      commitRendererAppearances: () => true,
      adoptRendererAppearanceSource() {},
    } as unknown as ScenePresentation;
    const world = {
      terrain: { mesh: {}, biome: { grass: {} } },
      scenery: null,
    } as unknown as SceneOptions["world"];
    const models: SceneOptions["models"] = [],
      instances: SceneOptions["instances"] = [];
    const frame = await scene(
      [],
      { presentation, appearances: initial, world, models, instances },
      { load_bundle() {} },
    );
    try {
      if (first === "battle") staged = 1;
      frame.updateSources(world, first === "art" ? grown : initial, models, instances);
      expect(prepared).toEqual([]);
      staged = 1;
      frame.updateSources(world, grown, models, instances);
      frame.updateSources(
        world,
        { ...grown, appearances: new Map(grown.appearances) },
        models,
        instances,
      );
      expect(prepared).toEqual([["A", "B"]]);
      await Promise.resolve();
      expect(frame.flush()).toBe(true);
      expect(await frame.settled()).toBe(false);
      expect(prepared).toEqual([["A", "B"]]);
    } finally {
      frame.dispose();
    }
  });
}

test("initial admission shares one packet between its readers and retires it before the next", async () => {
  const generated = proxyModels([{ kind: "box", x: 0, y: 0, z: 0, yaw: 0, color: [1, 1, 1] }]);
  const bundle = joinTextures(generated.assets[0].encode().bundle, []);
  if (bundle.kind !== "articulated") throw new Error("expected articulated fixture");
  const names = ["test-a", "test-b"];
  const installed: InstalledAppearances = {
    generation: 1,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    skeletons: new Map(),
    onRequest: new Map(),
    appearances: new Map(
      names.map((name, i) => [
        name,
        {
          contentId: { bundle: String(i).repeat(64), skeleton: null },
          bundle,
          unit: "vehicle",
          scenery: null,
          footprint: null,
          mounts: null,
          regionalFamily: null,
          paints: null,
          wreck: null,
          factions: null,
        },
      ]),
    ),
  };
  const seen = new Set<Uint8Array>();
  const rendererRead = new Map<string, { packet: Uint8Array; copy: Uint8Array }>();
  const presentationRead = new Map<string, { packet: Uint8Array; copy: Uint8Array }>();
  let peak = 0;
  const read = (packet: Uint8Array) => {
    seen.add(packet);
    peak = Math.max(
      peak,
      [...seen].reduce((sum, bytes) => sum + bytes.byteLength, 0),
    );
    return { packet, copy: new Uint8Array(packet) };
  };
  const presentation = {
    canonicalAppearanceRows: () => names.map((name) => ({ name })),
    presentedGround: () => null,
    installLabAssets(assets: Iterable<LabAsset>) {
      for (const asset of assets) presentationRead.set(asset.name, read(asset.bundle));
    },
    setLabModels() {},
    installGeometry() {},
    installRenderer() {},
    world: () => ({}),
  } as unknown as ScenePresentation;
  const frame = await scene(
    [],
    { presentation, appearances: installed },
    {
      load_bundle(name: string, _content: string, bytes: Uint8Array) {
        rendererRead.set(name, read(bytes));
      },
    },
  );
  try {
    expect([...rendererRead.keys()]).toEqual(names);
    expect([...presentationRead.keys()]).toEqual(names);
    for (const name of names) {
      const renderer = rendererRead.get(name)!;
      const posed = presentationRead.get(name)!;
      expect(renderer.packet).toBe(posed.packet);
      expect(renderer.copy).toEqual(posed.copy);
      expect(joinTextures(renderer.copy, [])).toEqual(bundle);
      expect(renderer.packet.byteLength).toBe(0);
    }
    expect(peak).toBe(Math.max(...[...rendererRead.values()].map((read) => read.copy.byteLength)));
  } finally {
    frame.dispose();
  }
});
