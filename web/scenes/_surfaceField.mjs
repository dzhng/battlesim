import { writeFile } from "node:fs/promises";

/** The terrain material's field lookup on the GPU against the same lookup on
 *  the CPU (`surfaceField.ts`, which the unit test holds to the all-primitives
 *  distance): every shape the export has, and a town of thousands of records,
 *  at pixels from the play camera's to one as wide as the map. Points stand
 *  round every stroke's ends too, where the stroke is cut square: past the
 *  end, beside it and off its corners. */
export async function surfaceFieldAgreement(ctx) {
  const page = await ctx.newPage();
  await page.goto(new URL("/", ctx.url).href);
  const repo = new URL("../../", import.meta.url).pathname;
  const result = await page.evaluate(async (repo) => {
    const file = (p) => `/@fs/${repo}${p}`;
    const terrainPath = file("packages/battle-renderer/src/frame/terrainMaterial.ts");
    const terrainText = await (await fetch(terrainPath)).text();
    const gpuUrl = terrainText.match(/from ["']([^"']*typegpu[^"']*)["']/)[1];
    const [
      { tgpu, d },
      { GpuRegistry },
      terrain,
      fields,
      surface,
      mesh,
      grounds,
      rules,
      biome,
      wasm,
      allocations,
    ] = await Promise.all([
      import(gpuUrl),
      import(file("packages/battle-renderer/src/frame/registry.ts")),
      import(terrainPath),
      import(file("packages/battle-renderer/src/terrain/surfaceField.ts")),
      import(file("packages/battle-renderer/src/terrain/terrainSurface.ts")),
      import(file("packages/battle-renderer/src/worldMesh.ts")),
      import(file("web/tests/surfaceGrounds.ts")),
      import(file("apps/battle-lab/src/scenarios.ts")),
      import(file("fixtures/biomes/summer.json")),
      import("/src/wasm/game_wasm.js"),
      import(file("packages/renderer-core/src/gpuAllocations.ts")),
    ]);
    const strokes = await import(file("packages/battle-renderer/src/terrain/strokes.ts"));
    await wasm.default();
    const adapter = await navigator.gpu.requestAdapter();
    const device = await adapter.requestDevice();
    const live = allocations.trackGpuAllocations(device);
    const registry = new GpuRegistry(device);
    const root = tgpu.initFromDevice({ device });
    registry.adopt(() => root.destroy());
    const validation = [];
    device.addEventListener("uncapturederror", (event) => validation.push(event.error.message));

    const FOOTPRINTS = [0.1, 3, 12, 40, 1e9];
    const POINTS = 4000;
    const view = new wasm.WorldView(
      JSON.stringify(grounds.CURATED_GROUND),
      JSON.stringify(rules.GAME_RULES),
    );
    const grown = [];
    try {
      const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules.GAME_RULES)));
      const cases = [
        [
          "every shape",
          mesh.buildWorldLayers(mesh.readWorldExports(view), layout, biome.default, "surface")
            .terrain,
        ],
        [
          "dense town",
          surface.terrainSurface(
            new Float32Array(0),
            grounds.denseGround(1600, 8, 40),
            biome.default,
            null,
          ),
        ],
      ];
      const source = terrain.createTerrainSource(root, registry);
      await source.ready();
      const queryLayout = tgpu.bindGroupLayout({
        points: { storage: (n) => d.arrayOf(d.vec4f, n), access: "readonly" },
        output: { storage: (n) => d.arrayOf(d.vec4f, n), access: "mutable" },
      });
      const { groundCell, groundSite, groundWater } = terrain;
      const kernel = tgpu
        .computeFn({ in: { gid: d.builtin.globalInvocationId }, workgroupSize: [1] })(`{
        let query=queryLayout.$.points[gid.x];let cell=groundCell(query.xy,query.z);let site=groundSite(query.xy,cell);
        queryLayout.$.output[gid.x]=vec4f(site.z,site.w,groundWater(query.xy,cell),0.0);
      }`)
        .$uses({ queryLayout, groundCell, groundSite, groundWater });
      const pipeline = root.createComputePipeline({ compute: kernel });
      await pipeline.initAsync();
      const count = POINTS * FOOTPRINTS.length;
      const input = registry.buffer({
        size: count * 16,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      });
      const output = registry.buffer({
        size: count * 16,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
      });
      const read = registry.buffer({
        size: count * 16,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
      });
      for (const [name, surfaceOf] of cases) {
        const site = surfaceOf.site;
        const field = fields.buildSurfaceField(site, terrain.terrainReach(surfaceOf));
        source.set(surfaceOf);
        // Seeded points over the map and a margin past it, as f32.
        let seed = 63;
        const next = () => {
          seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
          return seed / 4294967296;
        };
        const [minX, minY, maxX, maxY] = site.map;
        // Round each end a paved or forest stroke is cut at.
        const ends = [];
        const around = (records, stride) => {
          for (let o = 0; o < records.length; o += stride)
            for (const [bit, from, to] of [
              [strokes.CUT_A, o, o + 2],
              [strokes.CUT_B, o + 2, o],
            ]) {
              if (!(records[o + strokes.STROKE_CUTS] & bit)) continue;
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
        around(site.surfaceStrokes, site.surfaceStrokeStride);
        for (const shape of site.forestShapes) around(shape.strokes, strokes.STROKE_FLOATS);
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
        device.queue.writeBuffer(input, 0, queries);
        const encoder = root["~unstable"].createCommandEncoder();
        pipeline
          .with(source.group)
          .with(root.createBindGroup(queryLayout, { points: input, output }))
          .with(encoder)
          .dispatchWorkgroups(count);
        root.unwrap(encoder).copyBufferToBuffer(output, 0, read, 0, count * 16);
        encoder.submit();
        await read.mapAsync(GPUMapMode.READ);
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
        try {
          const values = new Float32Array(read.getMappedRange());
          for (let q = 0; q < count; q++) {
            const [x, y, footprint] = queries.subarray(q * 4, q * 4 + 3);
            if (q < endPoints * FOOTPRINTS.length && q % FOOTPRINTS.length === 0) {
              const round = capsule(site.surfaceStrokes, site.surfaceStrokeStride, x, y);
              const square = fields.pavedDistance(field, x, y, footprint);
              if (Math.abs(round) < 1e8 && Math.abs(round - square) > 0.05) cutValues++;
            }
            const expected = [
              fields.pavedDistance(field, x, y, footprint),
              fields.forestDistance(field, x, y, footprint),
              fields.waterDistance(field, x, y, footprint),
            ];
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
          read.unmap();
        }
        grown.push({
          name,
          records: field.records.length / fields.SURFACE_FLOATS,
          cells: [field.cols, field.rows],
          cellM: field.cellM,
          levels: field.levels,
          exactToM: field.exactToM === Infinity ? "any" : field.exactToM,
          indexBytes: field.index.byteLength,
          recordBytes: field.records.byteLength,
          compared: count * 3,
          endPoints,
          cutValues,
          finite,
          worstM: worst,
          wrong,
        });
      }
    } finally {
      registry.release();
      view.free();
    }
    const final = live();
    device.destroy();
    return { grounds: grown, validation, final };
  }, repo);
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
