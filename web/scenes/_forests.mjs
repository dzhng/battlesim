import { createHash } from "node:crypto";
import { writeFile } from "node:fs/promises";

/** Query the production terrain material at the public forest-export seam. */
export async function forestExportAgreement(ctx) {
  const page = await ctx.newPage();
  await page.goto(new URL("/", ctx.url).href);
  const repo = new URL("../../", import.meta.url).pathname;
  const result = await page.evaluate(async (repo) => {
    const file = (p) => `/@fs/${repo}${p}`;
    const terrainPath = file("packages/battle-renderer/src/frame/terrainMaterial.ts");
    const terrainText = await (await fetch(terrainPath)).text();
    const gpuUrl = terrainText.match(/from ["']([^"']*typegpu[^"']*)["']/)[1];
    const [{ tgpu, d }, { GpuRegistry }, terrain, field, mesh, rules, biome, wasm, allocations] =
      await Promise.all([
        import(gpuUrl),
        import(file("packages/battle-renderer/src/frame/registry.ts")),
        import(terrainPath),
        import(file("packages/battle-renderer/src/terrain/surfaceField.ts")),
        import(file("packages/battle-renderer/src/worldMesh.ts")),
        import(file("apps/battle-lab/src/scenarios.ts")),
        import(file("fixtures/biomes/summer.json")),
        import("/src/wasm/game_wasm.js"),
        import(file("packages/renderer-core/src/gpuAllocations.ts")),
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
    const view = new wasm.WorldView(JSON.stringify(map), JSON.stringify(rules.VILLAGE_RULES));
    const adapter = await navigator.gpu.requestAdapter();
    const device = await adapter.requestDevice();
    const live = allocations.trackGpuAllocations(device);
    const registry = new GpuRegistry(device);
    const root = tgpu.initFromDevice({ device });
    registry.adopt(() => root.destroy());
    const validation = [];
    device.addEventListener("uncapturederror", (event) => validation.push(event.error.message));
    const compiled = [],
      shader = device.createShaderModule.bind(device);
    device.createShaderModule = (descriptor) => {
      compiled.push(descriptor.code);
      return shader(descriptor);
    };
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
        xy: [90, 28],
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
    const reach = terrain.groundReach(biome.default, field.SURFACE_FOOTPRINT_M).forest;
    let rows;
    try {
      const exported = mesh.readWorldExports(view);
      const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules.VILLAGE_RULES)));
      const world = mesh.buildWorldLayers(exported, layout, biome.default, "surface");
      const source = terrain.createTerrainSource(root, registry);
      await source.ready();
      source.set(world.terrain);
      const queryLayout = tgpu.bindGroupLayout({
        points: { storage: (n) => d.arrayOf(d.vec4f, n), access: "readonly" },
        output: { storage: (n) => d.arrayOf(d.vec4f, n), access: "mutable" },
      });
      const groundSite = terrain.groundSite;
      const groundCell = terrain.groundCell;
      const kernel = tgpu
        .computeFn({ in: { gid: d.builtin.globalInvocationId }, workgroupSize: [1] })(`{
        let query=queryLayout.$.points[gid.x];let xy=query.xy;let site=groundSite(xy,groundCell(xy,query.z));queryLayout.$.output[gid.x]=vec4f(site.w,site.z,0.0,0.0);
      }`)
        .$uses({ queryLayout, groundSite, groundCell });
      const pipeline = root.createComputePipeline({ compute: kernel });
      await pipeline.initAsync();
      const input = registry.buffer({
        size: queries.length * 16,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      });
      const output = registry.buffer({
        size: queries.length * 16,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
      });
      const read = registry.buffer({
        size: queries.length * 16,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
      });
      device.queue.writeBuffer(
        input,
        0,
        Float32Array.from(queries.flatMap((c) => [...c.xy, c.footprint, 0])),
      );
      const encoder = root["~unstable"].createCommandEncoder();
      pipeline
        .with(source.group)
        .with(root.createBindGroup(queryLayout, { points: input, output }))
        .with(encoder)
        .dispatchWorkgroups(queries.length);
      root.unwrap(encoder).copyBufferToBuffer(output, 0, read, 0, queries.length * 16);
      encoder.submit();
      await read.mapAsync(GPUMapMode.READ);
      try {
        const values = new Float32Array(read.getMappedRange());
        rows = queries.map((c, i) => ({
          ...c,
          actual: values[i * 4],
          nativeForest: view.surface_at(c.xy[0], c.xy[1])[6],
        }));
      } finally {
        read.unmap();
      }
    } finally {
      registry.release();
      view.free();
      device.createShaderModule = shader;
    }
    const final = live();
    device.destroy();
    return { rows, reach, validation, final, compiled };
  }, repo);
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
