import { writeFile } from "node:fs/promises";

/** The terrain material's field lookup on the GPU against the same lookup on
 *  the CPU (`surfaceField.ts`, which the unit test holds to the all-primitives
 *  distance): every shape the export has, and a town of thousands of records,
 *  at pixels from the play camera's to one as wide as the map. */
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
      JSON.stringify(rules.VILLAGE_RULES),
    );
    const grown = [];
    try {
      const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules.VILLAGE_RULES)));
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
        const queries = new Float32Array(count * 4);
        for (let i = 0; i < POINTS; i++) {
          const x = minX - 40 + next() * (maxX - minX + 80),
            y = minY - 40 + next() * (maxY - minY + 80);
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
        try {
          const values = new Float32Array(read.getMappedRange());
          for (let q = 0; q < count; q++) {
            const [x, y, footprint] = queries.subarray(q * 4, q * 4 + 3);
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
    "the GPU's surface field lookup equals the CPU's on every shape and on a dense town",
    result.grounds.length === 2 &&
      result.grounds.every((g) => g.wrong.length === 0 && g.finite > g.compared / 20) &&
      // The town is dense enough to stop its ladder; the curated ground is not.
      result.grounds[0].exactToM === "any" &&
      result.grounds[1].exactToM !== "any" &&
      result.validation.length === 0 &&
      Object.values(result.final).every((value) => value === 0),
    JSON.stringify(result),
  );
  await page.close();
}
