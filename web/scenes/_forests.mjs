import { openTerrainQuery, finishTerrainQuery } from "./_terrainQuery.mjs";
import { createHash } from "node:crypto";
import { writeFile } from "node:fs/promises";

/** Query the production terrain material at the public forest-export seam. */
export async function forestExportAgreement(ctx) {
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
    const forest = (shape) => ({ shape });
    const map = {
      size: [256, 128],
      height_grid_m: 4,
      fog_cell_m: 8,
      slope_cutoff_deg: 35,
      forests: [
        forest({
          kind: "polygon",
          ring: [
            [0, 0],
            [90, 0],
            [90, 27],
            [27, 27],
            [27, 90],
            [0, 90],
          ],
        }),
        forest({
          kind: "polygon",
          ring: [
            [180, 0],
            [240, 0],
            [240, 60],
            [180, 60],
          ],
        }),
        forest({
          kind: "stroke",
          points: [
            [100, 20],
            [160, 80],
          ],
          width_m: 18,
        }),
      ],
    };
    const view = new wasm.WorldView(JSON.stringify(map), JSON.stringify(rules.rules));
    const cases = [
      { name: "concave polygon interior", xy: [10, 50], distance: 10 },
      { name: "concave polygon notch", xy: [50, 50], distance: -23 },
      { name: "closed polygon edge", xy: [27, 70], distance: 0 },
      { name: "across polygon triangulation diagonal", xy: [13.5, 45], distance: 13.5 },
      { name: "mixed-ID exact rectangle", xy: [210, 30], distance: 30 },
      { name: "diagonal strip centre", xy: [130, 50], distance: 9 },
      // The strip is cut square across its first point: behind it the
      // distance is to that flat end, not to a round cap (which gave -1).
      { name: "behind the strip's square end", xy: [100, 10], distance: -Math.hypot(5, 5) },
      { name: "just inside the strip's square end", xy: [101, 21], distance: Math.SQRT2 },
      {
        name: "off the corner of the strip's square end",
        xy: [108, 10],
        distance: -Math.hypot(18 / Math.SQRT2 - 9, Math.SQRT2),
      },
    ];
    // Each case is read twice: by a pixel as wide as the map, which reads
    // every distance exactly, and by a play-camera pixel, which reads it
    // exactly as far as its feathers reach and keeps its side beyond.
    const queries = [
      ...cases.map((c) => ({ ...c, footprint: 1e9 })),
      ...cases.map((c) => ({ ...c, footprint: 0.1 })),
    ];
    let rows, reach;
    try {
      const exported = mesh.readWorldExports(view);
      const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules.rules)));
      const world = mesh.buildWorldLayers(exported, layout, biome.default, "surface");
      // How far the play camera's widest exact pixel reads the forest rule.
      const ground = (
        await import(file("packages/battle-renderer/src/terrain/terrainSurface.ts"))
      ).presentGround(world.terrain.input);
      reach = ground.ground_reach(2)[1];
      const values = await window.__terrainQuery.query(
        ground,
        "distances",
        Float32Array.from(queries.flatMap((c) => [...c.xy, c.footprint, 0])),
      );
      try {
        rows = queries.map((c, i) => ({
          ...c,
          actual: values[i * 4 + 1],
          nativeForest: view.surface_at(c.xy[0], c.xy[1])[6],
        }));
      } finally {
        ground.free();
      }
    } finally {
      view.free();
    }
    return { rows, reach, validation: window.__terrainValidation };
  }, repo);
  Object.assign(result, await finishTerrainQuery(page));
  const compiled = result.compiled;
  delete result.compiled;
  result.shaderHashes = compiled.map((s) => createHash("sha256").update(s).digest("hex"));
  for (let i = 0; i < compiled.length; i++)
    await writeFile(ctx.evidencePath(`forest-query-${i}.wgsl`), compiled[i]);
  await writeFile(ctx.evidencePath("forest-query.json"), JSON.stringify(result, null, 2));
  ctx.check(
    "forest floor uses native concave, square-ended strip and explicit rectangle primitives",
    // WGSL's sqrt/length are not correctly rounded, so a GPU distance may sit a
    // few f32 steps off. Membership is the contract: the GPU's side of the
    // boundary must equal the sim's, and the distance must agree to 0.1 mm.
    result.rows.every(
      (r) =>
        (r.footprint > 1 || Math.abs(r.distance) <= result.reach
          ? Math.abs(r.actual - r.distance) <= 1e-4
          : r.distance > 0
            ? r.actual >= r.distance - 1e-4
            : r.actual <= r.distance + 1e-4) &&
        r.nativeForest === (r.distance >= 0 ? 1 : 0) &&
        (r.actual >= 0 ? 1 : 0) === r.nativeForest,
    ) &&
      result.rows.some((r) => r.footprint < 1 && Math.abs(r.distance) <= result.reach) &&
      result.validation.length === 0 &&
      Object.values(result.final).every((value) => value === 0),
    JSON.stringify(result),
  );
  await page.close();
}
