// Placement presentation through the production frame, with named skinned,
// articulated and static geometry. Alpha-zero must change neither colour nor
// shadow; partial alpha must differ from both zero and full; terrain occludes it.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";

export async function modelGhostAgreement(ctx) {
  const page = await ctx.newPage();
  try {
    const repo = new URL("../../", import.meta.url).pathname;
    // Plain same-origin document: no route startup or HMR navigation in this GPU probe.
    await page.goto(new URL(`/@fs/${repo}assets/runtime/catalog.json`, ctx.url).href);
    const result = await page.evaluate(async (repo) => {
      const file = (p) => `/@fs/${repo}${p}`;
      const [
        bindings,
        scenes,
        assets,
        bench,
        views,
        light,
        fog,
        overlay,
        models,
        presentation,
        presented,
        game,
      ] = await Promise.all([
        import("/src/wasm/renderer_browser/renderer_browser.js"),
        import(file("web/src/battle/render/scene.ts")),
        import(file("packages/scene-assets/src/loader.ts")),
        import(file("apps/battle-lab/src/workbench/benchWorld.ts")),
        import(file("apps/battle-lab/src/workbench/views.ts")),
        import(file("apps/battle-lab/src/gameLight.ts")),
        import(file("apps/battle-lab/src/gameFog.ts")),
        import(file("apps/battle-lab/src/gameOverlay.ts")),
        import(file("apps/battle-lab/src/gameModels.ts")),
        import(file("web/src/battle/sim/presentation.ts")),
        import(file("apps/battle-lab/src/presentedFrame.ts")),
        import(file("fixtures/rules.json")),
      ]);
      const installed = await new assets.AppearanceLibrary().load("/");
      const chosen = ["skinned", "articulated", "static"].map((kind) =>
        [...installed.appearances].find(
          ([name, entry]) =>
            entry.bundle.kind === kind && (kind !== "static" || name === "basketball_court"),
        ),
      );
      if (chosen.some((entry) => !entry)) throw new Error("missing placement model kind");
      await presentation.loadPresentation();
      const gpu = await bindings.BrowserGpu.new();
      const info = JSON.parse(gpu.info());
      const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
      const layers = {
        adapter: adapter.limits.maxTextureArrayLayers,
        device: info.limits.maxTextureArrayLayers,
      };
      const validation = [];
      void gpu.failure().then((error) => {
        if (error) validation.push(error);
      });
      const width = 256;
      const world = bench.benchWorld(null);
      const initial = chosen.map(([name]) => ({
        appearance: name,
        x: 0,
        y: 0,
        z: 0,
        yaw: 0,
        pose: { kind: "rest" },
        tier: 0,
      }));
      const states = [];
      const make = async (settings) => {
        const sight = new presentation.ScenePresentation();
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = width;
        sight.setGround(world.terrain.input, null);
        sight.setShadow(settings, world.terrain.mesh);
        sight.frame(
          0,
          {
            eye: [0, 0, 800],
            target: [1, 1, 0],
            up: [0, 0, 1],
            fovY: 0.8,
            near: 1,
            width,
            height: width,
          },
          { selected: [], xray: false, ghosts: [] },
          0,
        );
        const scene = await scenes.BrowserScene.create({
          gpu,
          canvas,
          presentation: sight,
          world,
          settings: {
            light: settings,
            fog: fog.gameFogStyle,
            paint: overlay.gamePaint,
            xrayMinHiddenFraction: overlay.gameXrayMinHiddenFragmentFraction,
          },
          detail: JSON.parse(models.gameModelDetail),
          overlayGlow: overlay.gameOverlayGlow,
          appearances: { ...installed, appearances: new Map(chosen) },
          models: initial,
          instances: [],
          redraw: () => {},
          failed: (e) => {
            throw e;
          },
        });
        const state = { scene, sight };
        states.push(state);
        return state;
      };
      const cases = [];
      try {
        const frame = await make(light.gameLight);
        const footprint = await make({
          ...light.gameLight,
          bloom: { ...light.gameLight.bloom, strength: 0 },
        });
        for (const [name, entry] of chosen) {
          const bundle = entry.bundle;
          const pose =
            bundle.kind === "static"
              ? { kind: "static", state: bundle.states.at(-1).name }
              : { kind: "rest" };
          const instance = { appearance: name, x: 0, y: 0, z: 0, yaw: 0, pose, tier: 0 };
          const camera = views.viewCamera("q-front", bundle.bounds);
          const pixels = async (list, drawn = frame) => {
            drawn.sight.setLabModels(list);
            drawn.sight.frame(
              0,
              presented.presentedCamera({ camera3d: camera, width, height: width }),
              { selected: [], xray: false, ghosts: [] },
              drawn.scene.staticGeneration,
            );
            drawn.scene.render();
            return Array.from((await drawn.scene.readPixels()).rgba);
          };
          const background = await pixels([]);
          const zero = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0] }]);
          const half = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0.4] }]);
          const depthUntouched = await pixels([
            { ...instance, z: 0.1, ghost: [0.1, 0.7, 1, 0] },
            { ...instance, ghost: [0.1, 0.7, 1, 0.4] },
          ]);
          const full = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 1] }]);
          const buried = await pixels([{ ...instance, z: -100, ghost: [0.1, 0.7, 1, 0.4] }]);
          const solid = [{ ...instance, ghost: [0.1, 0.7, 1, 1] }];
          const drawnClear = [await pixels([], footprint), await pixels(solid, footprint)];
          for (const state of states)
            state.sight.setFog({ sensors: game.default.sensors, eyes: [] });
          const fogBackground = await pixels([]),
            fogFull = await pixels(solid),
            fogHalf = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0.4] }]);
          const drawnFogged = [await pixels([], footprint), await pixels(solid, footprint)];
          let covered = 0,
            fogStable = 0;
          for (let p = 0; p < full.length; p += 4) {
            const delta = (a, b) =>
              Math.max(...[0, 1, 2].map((k) => Math.abs(a[p + k] - b[p + k])));
            if (
              delta(drawnClear[1], drawnClear[0]) > 30 &&
              delta(drawnFogged[1], drawnFogged[0]) > 30
            ) {
              covered++;
              if (delta(full, fogFull) < 8) fogStable++;
            }
          }
          for (const state of states) state.sight.setFog(null);
          const changed = (a, b) => a.reduce((n, x, i) => n + Number(i % 4 !== 3 && x !== b[i]), 0);
          cases.push({
            name,
            kind: bundle.kind,
            zero: changed(background, zero),
            half: changed(background, half),
            full: changed(half, full),
            buried: changed(background, buried),
            depthUntouched: changed(half, depthUntouched),
            covered,
            fogStable,
            fogBackgroundChanged: changed(background, fogBackground),
            fogHalfChanged: changed(half, fogHalf),
            image: fogHalf,
          });
        }
        return { cases, validation, width, layers };
      } finally {
        for (const state of states) {
          state.scene.dispose();
          state.sight.free();
        }
        gpu.dispose();
        gpu.free();
      }
    }, repo);
    ctx.check(
      "the model page's device holds the adapter's texture-layer limit",
      result.layers.device === result.layers.adapter,
      JSON.stringify(result.layers),
    );
    for (const item of result.cases) {
      ctx.check(
        `${item.kind} named ghost alpha and depth`,
        item.zero === 0 &&
          item.half > 0 &&
          item.full > 0 &&
          item.buried === 0 &&
          item.depthUntouched === 0 &&
          item.covered > 0 &&
          item.fogStable / item.covered > 0.9 &&
          item.fogBackgroundChanged > 0 &&
          item.fogHalfChanged > 0,
        JSON.stringify({ ...item, image: undefined }),
      );
      const png = new PNG({ width: result.width, height: result.width });
      png.data = Buffer.from(item.image);
      await writeFile(
        ctx.evidencePath
          ? ctx.evidencePath(`ghost-${item.kind}.png`)
          : `${ctx.out}/ghost-${item.kind}.png`,
        PNG.sync.write(png),
      );
    }
    ctx.check(
      "ghost pipelines have no WebGPU validation errors",
      result.validation.length === 0,
      result.validation.join("; "),
    );
  } finally {
    await page.close();
  }
}
