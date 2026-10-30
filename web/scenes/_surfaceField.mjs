import { createHash } from "node:crypto";
import { openTerrainQuery, finishTerrainQuery } from "./_terrainQuery.mjs";
import { writeFile } from "node:fs/promises";

/** The terrain material's field lookup on the GPU against the same lookup on
 *  the CPU (the ground's own, `ground::field`, held to the all-primitives
 *  distance by its tests): every shape the export has, and a town of thousands of records,
 *  at pixels from the play camera's to one as wide as the map. Points stand
 *  round every stroke's ends too, where the stroke is cut square: past the
 *  end, beside it and off its corners. */
export async function surfaceFieldAgreement(ctx) {
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
    const [fields, surface, grounds] = await Promise.all([
      import(file("packages/battle-renderer/src/records.gen.ts")),
      import(file("packages/battle-renderer/src/terrain/terrainSurface.ts")),
      import(file("web/tests/surfaceGrounds.ts")),
    ]);
    // A stroke's stretch: `ax, ay, bx, by, half width, kind, cut ends`.
    const STROKE_FLOATS = 7;
    const STROKE_CUTS = 6;
    const FOOTPRINTS = [0.1, 3, 12, 40, 1e9];
    const POINTS = 4000;
    const view = new wasm.WorldView(
      JSON.stringify(grounds.CURATED_GROUND),
      JSON.stringify(rules.rules),
    );
    const grown = [];
    try {
      const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules.rules)));
      const exported = mesh.readWorldExports(view);
      const dense = grounds.denseGround(1600, 8, 40);
      const cases = [
        [
          "every shape",
          mesh.buildWorldLayers(exported, layout, biome.default, "surface").terrain,
          exported,
          [0, 0, ...grounds.CURATED_GROUND.size],
        ],
        [
          "dense town",
          surface.bareTerrain(new Float32Array(0), [0, 0, 1600, 1600], 4, biome.default, dense),
          dense,
          [0, 0, 1600, 1600],
        ],
      ];
      const count = POINTS * FOOTPRINTS.length;
      for (const [name, surfaceOf, streams, map] of cases) {
        const ground = surface.presentGround(surfaceOf.input);
        // Seeded points over the map and a margin past it, as f32.
        let seed = 63;
        const next = () => {
          seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
          return seed / 4294967296;
        };
        const [minX, minY, maxX, maxY] = map;
        // Round each end a paved or forest stroke is cut at.
        const ends = [];
        const around = (records, stride) => {
          for (let o = 0; o < records.length; o += stride)
            for (const [bit, from, to] of [
              [fields.CUT_A, o, o + 2],
              [fields.CUT_B, o + 2, o],
            ]) {
              if (!(records[o + STROKE_CUTS] & bit)) continue;
              const half = records[o + 4];
              const run = Math.hypot(
                records[from] - records[to],
                records[from + 1] - records[to + 1],
              );
              const out = [
                (records[from] - records[to]) / run,
                (records[from + 1] - records[to + 1]) / run,
              ];
              for (const past of [-0.5, 0.3, 0.7 * half, 1.5 * half])
                for (const aside of [-1.2, -0.8, 0, 0.8, 1.2])
                  ends.push([
                    records[from] + out[0] * past - out[1] * aside * half,
                    records[from + 1] + out[1] * past + out[0] * aside * half,
                  ]);
            }
        };
        around(streams.surfaceStrokes, STROKE_FLOATS);
        around(streams.forestStrokes, STROKE_FLOATS);
        // As many of them as a fifth of the points, evenly through the list.
        const endPoints = Math.min(ends.length, POINTS / 5);
        const queries = new Float32Array(count * 4);
        // Ends whose CPU distance a round cap would have given differently.
        let cutValues = 0;
        for (let i = 0; i < POINTS; i++) {
          const end = i < endPoints ? ends[Math.floor((i * ends.length) / endPoints)] : null;
          const x = end ? end[0] : minX - 40 + next() * (maxX - minX + 80),
            y = end ? end[1] : minY - 40 + next() * (maxY - minY + 80);
          FOOTPRINTS.forEach((footprint, k) =>
            queries.set([x, y, footprint, 0], (i * FOOTPRINTS.length + k) * 4),
          );
        }
        const points = new Float64Array(count * 3);
        for (let q = 0; q < count; q++) points.set(queries.subarray(q * 4, q * 4 + 3), q * 3);
        const cpu = ground.ground_distances(points);
        const values = await window.__terrainQuery.query(ground, "distances", queries);
        const wrong = [];
        let finite = 0,
          worst = 0;
        // What a stroke read as a round-ended capsule would give at a point.
        const capsule = (records, stride, x, y) => {
          let inside = -1e9;
          for (let o = 0; o < records.length; o += stride) {
            const ax = records[o],
              ay = records[o + 1],
              dx = records[o + 2] - ax,
              dy = records[o + 3] - ay;
            // The dense ground pins a few stretches against the map's edge.
            if (dx === 0 && dy === 0) continue;
            const t = Math.min(
              1,
              Math.max(0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)),
            );
            inside = Math.max(
              inside,
              records[o + 4] - Math.hypot(x - ax - dx * t, y - ay - dy * t),
            );
          }
          return inside;
        };
        const { records, index } = JSON.parse(ground.ground_surface_field());
        const [, , cellM, cols, rows, levels, exactToM] = ground.ground_field_grid();
        try {
          for (let q = 0; q < count; q++) {
            const [x, y, footprint] = queries.subarray(q * 4, q * 4 + 3);
            const expected = cpu.subarray(q * 3, q * 3 + 3);
            if (q < endPoints * FOOTPRINTS.length && q % FOOTPRINTS.length === 0) {
              const round = capsule(streams.surfaceStrokes, STROKE_FLOATS, x, y);
              if (Math.abs(round) < 1e8 && Math.abs(round - expected[0]) > 0.05) cutValues++;
            }
            expected.forEach((want, k) => {
              const got = values[q * 4 + k];
              // A point no listed primitive reaches reads the sentinel on both.
              const far = Math.abs(want) >= 1e8;
              if (!far) {
                finite++;
                worst = Math.max(worst, Math.abs(got - want));
              }
              const ok = far ? got === Math.fround(want) : Math.abs(got - want) <= 2e-3;
              if (!ok && wrong.length < 5)
                wrong.push({ rule: ["paved", "forest", "water"][k], x, y, footprint, got, want });
            });
          }
        } finally {
          ground.free();
        }
        grown.push({
          name,
          records: records.length / fields.SURFACE_FLOATS,
          cells: [cols, rows],
          cellM,
          levels,
          exactToM: exactToM === Infinity ? "any" : exactToM,
          indexBytes: index.byteLength,
          recordBytes: records.byteLength,
          compared: count * 3,
          endPoints,
          cutValues,
          finite,
          worstM: worst,
          wrong,
        });
      }
    } finally {
      view.free();
    }
    return { grounds: grown, validation: window.__terrainValidation };
  }, repo);
  const { final, compiled } = await finishTerrainQuery(page);
  result.final = final;
  result.shaderHashes = compiled.map((source) => createHash("sha256").update(source).digest("hex"));
  for (let i = 0; i < compiled.length; i++)
    await writeFile(ctx.evidencePath(`surface-field-${i}.wgsl`), compiled[i]);
  await writeFile(ctx.evidencePath("surface-field.json"), JSON.stringify(result, null, 2));
  ctx.check(
    "the GPU's surface field lookup equals the CPU's on every shape and on a dense town, round the square ends of their strokes too",
    result.grounds.length === 2 &&
      result.grounds.every((g) => g.wrong.length === 0 && g.finite > g.compared / 20) &&
      // Stroke ends were among the points, and their square cut was read:
      // a round cap would have given a different distance there.
      result.grounds.every((g) => g.endPoints >= 100 && g.cutValues >= 20) &&
      // The town is dense enough to stop its ladder; the curated ground is not.
      result.grounds[0].exactToM === "any" &&
      result.grounds[1].exactToM !== "any" &&
      result.validation.length === 0 &&
      Object.values(result.final).every((value) => value === 0),
    JSON.stringify(result),
  );
  await page.close();
}
