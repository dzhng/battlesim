// Placement presentation through the production frame, with property-selected skinned,
// articulated and static geometry. Alpha-zero must change neither colour nor
// shadow; partial alpha must differ from both zero and full; terrain occludes it.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";

export async function modelGhostAgreement(ctx) {
  const page = await ctx.newPage();
  try {
    const repo = new URL("../../", import.meta.url).pathname;
    // Keep Vite's module preamble, without starting an unrelated app scene.
    await page.route("**/src/main.tsx*", (route) =>
      route.fulfill({ contentType: "application/javascript", body: "" }),
    );
    await page.goto(new URL("/", ctx.url).href);
    const result = await page.evaluate(async (repo) => {
      const file = (p) => `/@fs/${repo}${p}`;
      // The sheet renderer is the workbench's detached-scene seam: it owns
      // production world, light, fog, material admission and camera setup.
      const [
        { BrowserGpu },
        { AppearanceLibrary },
        { SheetRenderer },
        { viewCamera },
        { presentedCamera },
      ] = await Promise.all([
        import("/src/wasm/renderer_browser/renderer_browser.js"),
        import(file("packages/scene-assets/src/loader.ts")),
        import(file("apps/battle-lab/src/workbench/sheet.ts")),
        import(file("apps/battle-lab/src/workbench/views.ts")),
        import(file("apps/battle-lab/src/presentedFrame.ts")),
      ]);
      const rules = await (await fetch(file("fixtures/rules.json"))).json();
      const installed = await new AppearanceLibrary().load("/");
      const chosen = ["skinned", "articulated", "static"].map((kind) =>
        [...installed.appearances].find(
          ([, entry]) =>
            entry.bundle.kind === kind &&
            entry.bundle.bounds.max[2] > entry.bundle.bounds.min[2] &&
            (kind !== "static" ||
              entry.bundle.states.some((state) => state.tiers[0]?.indices.length > 0)),
        ),
      );
      if (chosen.some((entry) => !entry)) throw new Error("missing placement model kind");
      const gpu = await BrowserGpu.new();
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
      const cases = [];
      let sheet;
      try {
        for (const [name, entry] of chosen) {
          sheet = await SheetRenderer.create(gpu, {
            name,
            unit: entry.unit,
            type: null,
            scenery: entry.scenery,
            source: "catalog",
            findings: [],
            stats: null,
            clipStats: null,
            loadMs: 0,
            body: { edges: [], label: "ghost" },
            installed: { ...installed, appearances: new Map([[name, entry]]) },
          });
          const scene = sheet.frame;
          const sight = scene.presentation;
          const width = scene.canvas.width;
          const bundle = entry.bundle;
          const pose =
            bundle.kind === "static"
              ? {
                  kind: "static",
                  state: bundle.states.find((state) => state.tiers[0]?.indices.length > 0).name,
                }
              : { kind: "rest" };
          const instance = { appearance: name, x: 0, y: 0, z: 0, yaw: 0, pose, tier: 0 };
          const camera = viewCamera("q-front", bundle.bounds);
          const pixels = async (list) => {
            sight.setLabModels(list);
            sight.frame(
              0,
              presentedCamera({ camera3d: camera, width, height: width }),
              { selected: [], xray: false, ghosts: [] },
              scene.staticGeneration,
            );
            scene.render();
            return Array.from((await scene.readPixels()).rgba);
          };
          const background = await pixels([]);
          const zero = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0] }]);
          const half = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0.4] }]);
          const depthUntouched = await pixels([
            { ...instance, z: 0.1, ghost: [0.1, 0.7, 1, 0] },
            { ...instance, ghost: [0.1, 0.7, 1, 0.4] },
          ]);
          const full = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 1] }]);
          const buried = await pixels([
            { ...instance, z: -Math.max(100, bundle.bounds.max[2] + 1), ghost: [0.1, 0.7, 1, 0.4] },
          ]);
          const solid = [{ ...instance, ghost: [0.1, 0.7, 1, 1] }];
          sight.setFog({ sensors: rules.sensors, eyes: [] });
          const fogBackground = await pixels([]),
            fogFull = await pixels(solid),
            fogHalf = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0.4] }]);
          let covered = 0,
            fogStable = 0;
          for (let p = 0; p < full.length; p += 4) {
            const delta = (a, b) =>
              Math.max(...[0, 1, 2].map((k) => Math.abs(a[p + k] - b[p + k])));
            if (delta(full, background) > 30 && delta(fogFull, fogBackground) > 30) {
              covered++;
              if (delta(full, fogFull) < 8) fogStable++;
            }
          }
          sight.setFog(null);
          const changed = (a, b) => a.reduce((n, x, i) => n + Number(i % 4 !== 3 && x !== b[i]), 0);
          cases.push({
            name,
            width,
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
          sheet.dispose();
          sheet = null;
        }
        return { cases, validation, width: cases[0].width, layers };
      } finally {
        sheet?.dispose();
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
