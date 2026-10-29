/** Production material probes share one ordinary scene's Rust GPU/pipeline owner. */
export async function openTerrainQuery(ctx) {
  const page = await ctx.newPage();
  await page.addInitScript(() => {
    const resources = new Map();
    const queues = [];
    window.__terrainValidation = [];
    let created = 0;
    const onDestroy = (id, destroy) =>
      function (...args) {
        resources.delete(id);
        return destroy.apply(this, args);
      };
    const observe = (resource, kind) => {
      const id = created++;
      resources.set(id, { reference: new WeakRef(resource), kind });
      resource.destroy = onDestroy(id, resource.destroy);
      return resource;
    };
    window.__terrainResources = {
      counts() {
        const counts = { buffers: 0, textures: 0 };
        for (const [id, row] of resources) {
          if (!row.reference.deref()) resources.delete(id);
          else counts[row.kind]++;
        }
        return counts;
      },
      drain: () => Promise.all(queues.map((queue) => queue.onSubmittedWorkDone())),
    };
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function (...args) {
      const device = await request.apply(this, args);
      device.addEventListener("uncapturederror", (event) =>
        window.__terrainValidation.push(event.error.message),
      );
      queues.push(device.queue);
      for (const [method, kind] of [
        ["createBuffer", "buffers"],
        ["createTexture", "textures"],
      ]) {
        const create = device[method];
        device[method] = function (...descriptors) {
          return observe(create.apply(this, descriptors), kind);
        };
      }
      return device;
    };
  });
  // This diagnostic owns its scene; an unrelated menu backdrop would add live resources.
  await page.route("**/src/main.tsx*", (route) =>
    route.fulfill({ contentType: "application/javascript", body: "" }),
  );
  await page.goto(new URL("/", ctx.url).href);
  const repo = new URL("../../", import.meta.url).pathname;
  await page.evaluate(async (repo) => {
    const file = (path) => `/@fs/${repo}${path}`;
    const [
      { BrowserGpu },
      { BrowserScene },
      { ScenePresentation, loadPresentation },
      surface,
      biome,
      { gameLight },
      { gameFogStyle },
      { gameBuildingStyle, gameModelDetail },
      { gameOverlayGlow, gamePaint },
      { applyShadowQuality },
      { sceneMinimumProfile },
    ] = await Promise.all([
      import("/src/wasm/renderer_browser/renderer_browser.js"),
      import(file("web/src/battle/render/scene.ts")),
      import(file("web/src/battle/sim/presentation.ts")),
      import(file("packages/battle-renderer/src/terrain/terrainSurface.ts")),
      import(file("fixtures/biomes/summer.json")),
      import(file("apps/battle-lab/src/gameLight.ts")),
      import(file("apps/battle-lab/src/gameFog.ts")),
      import(file("apps/battle-lab/src/gameModels.ts")),
      import(file("apps/battle-lab/src/gameOverlay.ts")),
      import(file("packages/renderer-core/src/lightQuality.ts")),
      import(file("apps/battle-lab/src/graphicsSettings.ts")),
    ]);
    await loadPresentation();
    const gpu = await BrowserGpu.new();
    const presentation = new ScenePresentation();
    const terrain = surface.bareTerrain(new Float32Array(0), [0, 0, 32, 32], 4, biome.default);
    const light = applyShadowQuality(gameLight, sceneMinimumProfile);
    presentation.setGround(terrain.input, null);
    presentation.setBuildings(null, null, gameBuildingStyle, {
      azimuth: light.sun_azimuth,
      elevation: light.sun_elevation,
      farM: light.cascades.max_far_m,
    });
    presentation.setShadow(light, terrain.mesh);
    presentation.frame(
      0,
      {
        eye: [16, -20, 20],
        target: [16, 16, 0],
        up: [0, 0, 1],
        fovY: 0.8,
        near: 1,
        width: 64,
        height: 64,
      },
      { selected: [], xray: false, ghosts: [] },
      0,
    );
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 64;
    const scene = await BrowserScene.create({
      gpu,
      canvas,
      presentation,
      world: { terrain, props: new Float32Array(0), scenery: null, grass: null },
      settings: { light, fog: gameFogStyle, paint: gamePaint },
      detail: JSON.parse(gameModelDetail),
      overlayGlow: gameOverlayGlow,
      appearances: null,
      models: [],
      instances: [],
      redraw() {},
      failed(error) {
        throw error;
      },
    });
    window.__terrainQuery = { scene, presentation, gpu, compiled: new Set() };
    window.__terrainQuery.query = async (
      ground,
      mode,
      rows,
      reference = new Uint8Array(0),
      side = 0,
    ) => {
      const result = await scene.renderer.ground_query(ground, mode, rows, reference, side);
      window.__terrainQuery.compiled.add(result.source);
      return result.values;
    };
  }, repo);
  return page;
}

export async function finishTerrainQuery(page) {
  const observed = await page.evaluate(() => window.__terrainResources.counts());
  if (!observed.buffers || !observed.textures)
    throw new Error("terrain lifetime observer missed live resources");
  const compiled = await page.evaluate(async () => {
    const owner = window.__terrainQuery;
    await owner.scene.drain();
    owner.scene.dispose();
    owner.presentation.free();
    owner.gpu.dispose();
    owner.gpu.free();
    const compiled = [...owner.compiled];
    delete window.__terrainQuery;
    await window.__terrainResources.drain();
    return compiled;
  });
  const cdp = await page.context().newCDPSession(page);
  try {
    await cdp.send("HeapProfiler.collectGarbage");
  } finally {
    await cdp.detach();
  }
  return {
    compiled,
    ...(await page.evaluate(() => ({
      final: window.__terrainResources.counts(),
      validation: window.__terrainValidation,
    }))),
  };
}
