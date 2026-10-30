// Shared browser runtime recovery and owned snapshots through production admission.
export async function rendererLifecycle(ctx) {
  const page = await ctx.newPage({ allowErrors: true });
  try {
    const repo = new URL("../../", import.meta.url).pathname;
    await page.goto(new URL(`/@fs/${repo}assets/runtime/catalog.json`, ctx.url).href);
    const result = await page.evaluate(async (repo) => {
      const file = (path) => `/@fs/${repo}${path}`;
      const [
        bindings,
        sceneModule,
        presentation,
        assets,
        bench,
        views,
        light,
        fog,
        overlay,
        models,
        camera,
        game,
      ] = await Promise.all([
        import("/src/wasm/renderer_browser/renderer_browser.js"),
        import(file("web/src/battle/render/scene.ts")),
        import(file("web/src/battle/sim/presentation.ts")),
        import(file("packages/scene-assets/src/loader.ts")),
        import(file("apps/battle-lab/src/workbench/benchWorld.ts")),
        import(file("apps/battle-lab/src/workbench/views.ts")),
        import(file("apps/battle-lab/src/gameLight.ts")),
        import(file("apps/battle-lab/src/gameFog.ts")),
        import(file("apps/battle-lab/src/gameOverlay.ts")),
        import(file("apps/battle-lab/src/gameModels.ts")),
        import(file("apps/battle-lab/src/presentedFrame.ts")),
        import(file("fixtures/rules.json")),
      ]);
      await presentation.loadPresentation();
      const installed = await new assets.AppearanceLibrary().load("/");
      const name = "test_tank";
      const entry = installed.appearances.get(name);
      if (!entry) throw new Error("missing authored runtime test tank");
      const world = bench.benchWorld(null);
      const sight = new presentation.ScenePresentation();
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 1;
      const pose = camera.presentedCamera({
        camera3d: views.viewCamera("q-front", entry.bundle.bounds),
        width: 256,
        height: 256,
      });
      const ui = { selected: [], xray: false, ghosts: [] };
      sight.setGround(world.terrain.input, false);
      sight.setShadow(light.gameLight, world.terrain.mesh);
      sight.setFog({
        sensors: game.default.sensors,
        eyes: [
          {
            key: "runtime-eye",
            position: [0, 0, 2],
            forward: 0,
            range: 30,
            shape: { front: 1, side: 1, rear: 1 },
          },
        ],
      });
      sight.frame(0, pose, ui, 0);
      const timings = {};
      let started = performance.now();
      const gpu = await bindings.BrowserGpu.new();
      let failure;
      void gpu.failure().then((reason) => {
        failure = reason;
      });
      timings.deviceAdmissionMs = performance.now() - started;
      let recoveredGpu = null;
      let scene;
      const changed = (a, b) =>
        a.length !== b.length ? Infinity : a.reduce((n, v, i) => n + Number(v !== b[i]), 0);
      try {
        started = performance.now();
        scene = await sceneModule.BrowserScene.create({
          gpu,
          canvas,
          presentation: sight,
          world,
          settings: {
            light: light.gameLight,
            fog: fog.gameFogStyle,
            paint: overlay.gamePaint,
            xrayMinHiddenFraction: overlay.gameXrayMinHiddenFragmentFraction,
          },
          detail: JSON.parse(models.gameModelDetail),
          overlayGlow: overlay.gameOverlayGlow,
          appearances: { ...installed, appearances: new Map([[name, entry]]) },
          models: [{ appearance: name, x: 0, y: 0, z: 0, yaw: 0, pose: { kind: "rest" }, tier: 0 }],
          instances: [],
          redraw: () => {},
          failed: (e) => {
            throw e;
          },
        });
        timings.sceneAdmissionMs = performance.now() - started;
        const draw = () => {
          sight.frame(0, pose, ui, scene.staticGeneration);
          scene.render();
        };
        const tinyDraw = async () => {
          const { width, height } = scene.stats();
          sight.frame(0, { ...pose, width, height }, ui, scene.staticGeneration);
          scene.render();
          const pixels = await scene.readPixels();
          return (
            pixels.width > 0 &&
            pixels.height > 0 &&
            pixels.rgba.length === pixels.width * pixels.height * 4
          );
        };
        const tinyAdmission = await tinyDraw();
        canvas.width = canvas.height = 256;
        scene.resize();
        draw();
        const before = Array.from((await scene.readPixels()).rgba);
        const queued = scene.readPixels();
        scene.setControls({ view: "fog-mask" });
        scene.render();
        const other = Array.from((await scene.readPixels()).rgba);
        const snapshot = Array.from((await queued).rgba);
        scene.setControls({ view: "final" });
        scene.render();
        const restored = Array.from((await scene.readPixels()).rgba);
        const points = [{ position: [0, 0, 1] }, { position: [100, 100, 1] }];
        const fogBefore = Array.from(await sight.probeFog(scene.renderer, points));
        canvas.width = canvas.height = 1;
        scene.resize();
        const tinyResize = await tinyDraw();
        started = performance.now();
        canvas.width = 192;
        canvas.height = 160;
        scene.resize();
        sight.frame(0, { ...pose, width: 192, height: 160 }, ui, scene.staticGeneration);
        scene.render();
        canvas.width = canvas.height = 256;
        scene.resize();
        draw();
        timings.resizeRoundTripCpuMs = performance.now() - started;
        const resized = Array.from((await scene.readPixels()).rgba);
        timings.resizeRoundTripWithReadbackMs = performance.now() - started;
        const renderer = scene.renderer;
        started = performance.now();
        renderer.recover_surface();
        draw();
        timings.surfaceReconfigureCpuMs = performance.now() - started;
        const resurfaced = Array.from((await scene.readPixels()).rgba);
        timings.surfaceReconfigureWithReadbackMs = performance.now() - started;
        const failureBeforeLoss = failure ?? null;
        renderer.destroy_device_for_test();
        const loss = await gpu.failure();
        started = performance.now();
        recoveredGpu = await bindings.BrowserGpu.new();
        await renderer.recover_device(recoveredGpu, canvas);
        sight.installRenderer(renderer);
        await renderer.prepare_assets();
        timings.deviceRecreationAdmissionMs = performance.now() - started;
        draw();
        const fogAfter = Array.from(await sight.probeFog(renderer, points));
        const recovered = Array.from((await scene.readPixels()).rgba);
        timings.deviceRecreationWithQueriesAndReadbackMs = performance.now() - started;
        return {
          timings,
          tinyAdmission,
          tinyResize,
          failureBeforeLoss,
          snapshotDifference: changed(before, snapshot),
          otherDifference: changed(before, other),
          restoredDifference: changed(before, restored),
          resizeDifference: changed(before, resized),
          surfaceDifference: changed(before, resurfaced),
          recoveryDifference: changed(before, recovered),
          loss: typeof loss === "string" && /device lost.*Destroyed/.test(loss),
          fogBefore,
          fogAfter,
          stats: scene.stats(),
        };
      } finally {
        scene?.dispose();
        sight.free();
        gpu.dispose();
        gpu.free();
        recoveredGpu?.dispose();
        recoveredGpu?.free();
      }
    }, repo);
    ctx.check(
      "collapsed targets admit and resize to a usable image before restoring the scene",
      result.tinyAdmission && result.tinyResize && result.failureBeforeLoss === null,
      JSON.stringify(result),
    );
    ctx.check(
      "owned readback preserves its submitted frame while another view renders",
      result.snapshotDifference === 0 &&
        result.otherDifference > 0 &&
        result.restoredDifference === 0,
      JSON.stringify(result),
    );
    ctx.check(
      "shared runtime resize, surface reconfiguration and real device recreation preserve current pixels",
      result.resizeDifference === 0 &&
        result.surfaceDifference === 0 &&
        result.recoveryDifference === 0 &&
        result.loss,
      JSON.stringify(result),
    );
    ctx.check(
      "recreated fog restores current canonical knowledge before queries",
      result.fogBefore.includes(0) &&
        result.fogBefore.includes(1) &&
        JSON.stringify(result.fogBefore) === JSON.stringify(result.fogAfter),
      JSON.stringify(result),
    );
    await ctx.writeEvidence("renderer-lifecycle.json", result);
  } finally {
    await page.close();
  }
}
