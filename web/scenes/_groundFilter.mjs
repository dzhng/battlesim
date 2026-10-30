import { openTerrainQuery, finishTerrainQuery } from "./_terrainQuery.mjs";
// The scars are filtered in the shader from loaded texels, never by a
// sampler. Query the production cubic and linear lookups on a dense ground and
// compare them, bit for bit, with the hardware filter's original arithmetic.
import { createHash } from "node:crypto";
import { writeFile } from "node:fs/promises";
export async function groundFilterExact(ctx) {
  const page = await openTerrainQuery(ctx);
  const repo = new URL("../../", import.meta.url).pathname;
  const result = await page.evaluate(async (repo) => {
    const file = (p) => `/@fs/${repo}${p}`;
    const [mesh, rules, biome, wasm] = await Promise.all([
      import(file("packages/battle-renderer/src/worldMesh.ts")),
      import(file("web/src/battle/catalog/sets.ts")).then((m) => m.catalogSet("test")),
      import(file("fixtures/biomes/summer.json")),
      import("/src/wasm/game_wasm.js"),
    ]);
    await wasm.default();
    const view = new wasm.WorldView(
      JSON.stringify({ size: [32, 32], height_grid_m: 4, fog_cell_m: 8, slope_cutoff_deg: 35 }),
      JSON.stringify(rules.rules),
    );
    const world = mesh.buildWorldLayers(
      mesh.readWorldExports(view),
      JSON.parse(wasm.world_layout(JSON.stringify(rules.rules))),
      biome.default,
      "surface",
    );
    try {
      const [{ ScenePresentation }, { groundRecords }] = await Promise.all([
        import(file("web/src/battle/sim/presentation.ts")),
        import(file("web/tests/groundRuns.ts")),
      ]);
      const failures = [];
      let queryCount = 0;
      let bitFailures = 0,
        linearFailures = 0,
        maxError = 0;
      let decoder;
      try {
        const runs = [];
        for (let y = 0; y < 38; y++)
          for (let x = 0; x < 38; x++) {
            const width = Math.min(16, 600 - x * 16),
              height = Math.min(16, 600 - y * 16),
              tile = y * 38 + x;
            if (tile === 1403 || tile === 1404) continue;
            if (width === 16) runs.push([tile, height * 16 * 256, 37 + 73 * 256, 255 + 113 * 256]);
            else
              for (let row = 0; row < height; row++)
                runs.push([tile, row * 16 + width * 256, 37 + 73 * 256, 255 + 113 * 256]);
          }
        const value = (c) => [
          (c * 11 + 7) % 256,
          (c * 17 + 13) % 256,
          (c * 23 + 29) % 256,
          (c * 31 + 37) % 256,
        ];
        runs.push([1403, 8 * 256, 53 + 137 * 256, 251 + 17 * 256]);
        runs.push([1403, 8 + 248 * 256, 37 + 73 * 256, 255 + 113 * 256]);
        for (let c = 0; c < 256; c++) {
          const v = value(c);
          runs.push([1404, c + 256, v[0] + v[1] * 256, v[2] + v[3] * 256]);
        }
        runs.sort((a, b) => a[0] - b[0] || (a[1] % 256) - (b[1] % 256));
        // The presentation learns the ground from a publication carrying it.
        const battle = new wasm.Battle(
          JSON.stringify({
            map: { size: [32, 32], height_grid_m: 4, fog_cell_m: 8, slope_cutoff_deg: 35 },
            rules: rules.rules,
            units: [],
            events: [],
            scripts: [],
          }),
          1,
        );
        const published = JSON.parse(battle.observation_layout());
        battle.free();
        published.ground = { ...published.ground, cellM: 1, cols: 600, rows: 600 };
        decoder = new ScenePresentation(published);
        decoder.decode(
          groundRecords(published)({
            epoch: 1,
            side: published.ground.sides[0],
            baseRevision: 0,
            revision: 1,
            full: true,
            runs: Float32Array.from(runs.flat()),
          }),
        );
        decoder.setGround(world.terrain.input, null);
        decoder.frame(
          0,
          {
            eye: [0, -20, 20],
            target: [0, 0, 0],
            up: [0, 0, 1],
            fovY: 0.8,
            near: 1,
            width: 64,
            height: 64,
          },
          { selected: [], xray: false, ghosts: [] },
          0,
        );
        const bytes = new Uint8Array(600 * 600 * 4);
        for (let y = 0; y < 600; y++)
          for (let x = 0; x < 600; x++)
            bytes.set(
              x >= 576 && x < 592 && y >= 576 && y < 592
                ? value((y - 576) * 16 + x - 576)
                : x >= 560 && x < 568 && y === 576
                  ? [53, 137, 251, 17]
                  : [37, 73, 255, 113],
              (y * 600 + x) * 4,
            );
        const pointValues = [
          // Frozen first failure: the old /size then *size path removed
          // normalized-f32 rounding inside the actual cubic sampler.
          0.97439044713974,
          0.9743750095367432,
          0.5 / 600,
          0.5 / 600,
          584.5 / 600,
          584.5 / 600,
          591.999 / 600,
          591.001 / 600,
          -0,
          -0,
          -0.25,
          -0.25,
          1e-42,
          1e-42,
          1.25,
          1.25,
        ];
        for (let y = 0; y < 8; y++)
          for (let x = 0; x < 8; x++)
            pointValues.push((576.125 + x * 2) / 600, (576.375 + y * 2) / 600);
        for (let i = 0; i <= 2048; i++)
          pointValues.push((582.5 + i / 2048) / 600, (583.125 + i / 8192) / 600);
        for (const y of [575.999, 576, 576.5, 576.999, 577.001])
          for (const x of [559.999, 560.5, 567.5, 567.999, 568.001, 575.999, 576.001])
            pointValues.push(x / 600, y / 600);
        const points = Float32Array.from(pointValues),
          n = points.length / 2;
        queryCount = n;
        const rows = Float32Array.from(
          Array.from({ length: n }, (_, i) => [points[i * 2], points[i * 2 + 1], 0, 0]).flat(),
        );
        const result = await decoder.queryGround(
          window.__terrainQuery.scene.renderer,
          "filters",
          rows,
          bytes,
          600,
        );
        window.__terrainQuery.compiled.add(result.source);
        const values = result.values;
        const bits = new Uint32Array(values.buffer, values.byteOffset, values.length);
        for (let i = 0; i < n; i++)
          for (let c = 0; c < 4; c++) {
            if (bits[i * 16 + c] !== bits[i * 16 + 4 + c]) {
              bitFailures++;
              failures.push({
                point: i,
                channel: c,
                uv: [points[i * 2], points[i * 2 + 1]],
                actual: values[i * 16 + c],
                original: values[i * 16 + 4 + c],
                ulp: Math.abs(bits[i * 16 + c] - bits[i * 16 + 4 + c]),
              });
            }
            if (bits[i * 16 + 8 + c] !== bits[i * 16 + 12 + c]) linearFailures++;
            maxError = Math.max(maxError, Math.abs(values[i * 16 + c] - values[i * 16 + 4 + c]));
          }
      } finally {
        decoder?.free();
      }
      return {
        queryCount,
        bitFailures,
        linearFailures,
        failures,
        maxError,
        validation: window.__gpuResources.validation,
      };
    } finally {
      view.free();
    }
  }, repo);
  const { final, compiled: modules } = await finishTerrainQuery(page);
  result.afterQueries = final;
  result.shaderHashes = modules.map((code) => createHash("sha256").update(code).digest("hex"));
  for (let i = 0; i < modules.length; i++)
    await writeFile(ctx.evidencePath(`ground-filter-query-${i}.wgsl`), modules[i]);
  // Hash the actual composed production material together with the diagnostic entry.
  result.sourceHashes = result.shaderHashes;
  await writeFile(ctx.evidencePath("ground-filter-query.json"), JSON.stringify(result, null, 2));
  ctx.check(
    "the production cubic sampler matches original dense hardware values bit for bit",
    result.bitFailures === 0 &&
      result.linearFailures === 0 &&
      Object.values(result.afterQueries).every((count) => count === 0) &&
      result.validation.length === 0,
    JSON.stringify({
      bitFailures: result.bitFailures,
      linearFailures: result.linearFailures,
      failures: result.failures,
      maxError: result.maxError,
      afterQueries: result.afterQueries,
      shaderHashes: result.shaderHashes,
    }),
  );
  await page.close();
}
