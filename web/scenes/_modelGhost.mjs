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
      const [gpu, frames, assets, poses, bench, views, light, fog, overlay, models] = await Promise.all([
        import(file("packages/renderer-core/src/device.ts")),
        import(file("packages/battle-renderer/src/frame/battleFrame.ts")),
        import(file("packages/scene-assets/src/loader.ts")),
        import(file("packages/battle-renderer/src/models/modelInstances.ts")),
        import(file("apps/battle-lab/src/workbench/benchWorld.ts")),
        import(file("apps/battle-lab/src/workbench/views.ts")),
        import(file("apps/battle-lab/src/gameLight.ts")),
        import(file("apps/battle-lab/src/gameFog.ts")),
        import(file("apps/battle-lab/src/gameOverlay.ts")),
        import(file("apps/battle-lab/src/gameModels.ts")),
      ]);
      // Every unit's art: the probe draws one model of each kind, whoever wears it.
      const installed = await new assets.AppearanceLibrary().load("/");
      const chosen = ["skinned", "articulated", "static"].map((kind) =>
        [...installed.appearances].find(([, entry]) => entry.bundle.kind === kind),
      );
      if (chosen.some((entry) => !entry)) throw new Error("missing placement model kind");
      // The page's device admission, as every page that draws models gets it.
      const { adapter, device } = await gpu.requestGpuDevice();
      const layers = {
        adapter: adapter.limits.maxTextureArrayLayers,
        device: device.limits.maxTextureArrayLayers,
      };
      const validation = [];
      device.addEventListener("uncapturederror", (event) => validation.push(event.error.message));
      const width = 256;
      const target = device.createTexture({
        size: [width, width],
        format: "rgba8unorm",
        usage: 0x11,
      });
      const read = device.createBuffer({ size: width * width * 4, usage: 0x09 });
      const frame = await frames.createBattleFrame(device, "rgba8unorm", {
        light: light.gameLight,
        fogGeometry: fog.gameFogGeometry,
        fogStyle: fog.gameFogStyle,
        overlayGlow: overlay.gameOverlayGlow,
        paint: overlay.gamePaint,
        xrayMinHiddenFragmentFraction: overlay.gameXrayMinHiddenFragmentFraction,
        models: models.gameModelDetail,
        glass: models.gameGlass,
        buildings: models.gameBuildingStyle,
        world: bench.benchWorld(null),
        instances: [],
        width,
        height: width,
      });
      const cases = [];
      try {
        await frame.setAppearances({
          ...installed,
          appearances: new Map(chosen),
          templates: undefined,
        });
        for (const [name, entry] of chosen) {
          const bundle = entry.bundle;
          const pose = poses.restingModelPose(bundle);
          // A later state exercises its nonzero range in the shared index buffer.
          if (pose.kind === "static") pose.state = bundle.states.at(-1).name;
          const instance = { appearance: name, x: 0, y: 0, z: 0, yaw: 0, pose, tier: 0 };
          const camera = views.viewCamera("q-front", bundle.bounds);
          async function pixels(list) {
            frame.setModels(list);
            frame.render(target.createView(), { camera3d: camera, width, height: width });
            const encoder = device.createCommandEncoder();
            encoder.copyTextureToBuffer(
              { texture: target },
              { buffer: read, bytesPerRow: width * 4 },
              [width, width],
            );
            device.queue.submit([encoder.finish()]);
            await read.mapAsync(1);
            const data = [...new Uint8Array(read.getMappedRange())];
            read.unmap();
            return data;
          }
          const background = await pixels([]);
          const zero = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0] }]);
          const half = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0.4] }]);
          const depthUntouched = await pixels([
            { ...instance, z: 0.1, ghost: [0.1, 0.7, 1, 0] },
            { ...instance, ghost: [0.1, 0.7, 1, 0.4] },
          ]);
          const full = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 1] }]);
          const buried = await pixels([{ ...instance, z: -100, ghost: [0.1, 0.7, 1, 0.4] }]);
          frame.setFog({
            world: {
              nx: 65,
              ny: 65,
              spacing: 4,
              pageSize: 16,
              minHeight: 0,
              pageIds: new Uint32Array(0),
              heights: new Float32Array(0),
              foliage: new Float32Array([1, 1, 4]),
              targetHeightM: 1,
              foliageFullBlock: 1,
            },
            sight: { eyes: [], occluders: [] },
          });
          const fogBackground = await pixels([]);
          const fogFull = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 1] }]);
          const fogHalf = await pixels([{ ...instance, ghost: [0.1, 0.7, 1, 0.4] }]);
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
          frame.setFog(null);
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
        await device.queue.onSubmittedWorkDone();
        return { cases, validation, width, layers };
      } finally {
        frame.dispose();
        target.destroy();
        read.destroy();
        device.destroy();
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
