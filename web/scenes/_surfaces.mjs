import { createHash } from "node:crypto";
import { writeFile } from "node:fs/promises";

/** Query the production terrain material at the public surface-export seam. */
export async function surfaceExportAgreement(ctx) {
  const page = await ctx.newPage();
  await page.goto(new URL("/", ctx.url).href);
  const repo = new URL("../../", import.meta.url).pathname;
  const result = await page.evaluate(async (repo) => {
    const file = (p) => `/@fs/${repo}${p}`;
    const terrainPath = file("packages/battle-renderer/src/frame/terrainMaterial.ts");
    const terrainText = await (await fetch(terrainPath)).text();
    const gpuUrl = terrainText.match(/from ["']([^"']*typegpu[^"']*)["']/)[1];
    const [{ tgpu, d }, { GpuRegistry }, terrain, mesh, rules, biome, wasm, allocations] =
      await Promise.all([
        import(gpuUrl),
        import(file("packages/battle-renderer/src/frame/registry.ts")),
        import(terrainPath),
        import(file("packages/battle-renderer/src/worldMesh.ts")),
        import(file("apps/battle-lab/src/scenarios.ts")),
        import(file("fixtures/biomes/summer.json")),
        import("/src/wasm/game_wasm.js"),
        import(file("packages/renderer-core/src/gpuAllocations.ts")),
      ]);
    await wasm.default();
    const polygon = (ring) => ({ kind: "road", shape: { kind: "polygon", ring } });
    const map = {
      size: [32, 64],
      height_grid_m: 4,
      fog_cell_m: 8,
      slope_cutoff_deg: 35,
      surfaces: [
        polygon([
          [2, 2],
          [12, 2],
          [12, 12],
          [2, 12],
        ]),
        polygon([
          [2, 20],
          [12, 20],
          [12, 30],
          [2, 30],
        ]),
        polygon([
          [7, 20],
          [17, 20],
          [17, 30],
          [7, 30],
        ]),
        polygon([
          [2, 40],
          [12, 40],
          [12, 50],
          [2, 50],
        ]),
        polygon([
          [12, 40],
          [22, 40],
          [22, 50],
          [12, 50],
        ]),
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
      { name: "single polygon diagonal centre", xy: [7, 7], distance: 5 },
      { name: "single polygon across diagonal", xy: [7.25, 7], distance: 4.75 },
      { name: "single polygon inside outer edge", xy: [2.25, 7], distance: 0.25 },
      { name: "single polygon outside outer edge", xy: [1.75, 7], distance: -0.25 },
      { name: "single polygon on outer edge", xy: [7, 2], distance: 0 },
      { name: "overlap covers second polygon boundary", xy: [7, 25], distance: 5 },
      { name: "overlap covers first polygon boundary", xy: [12, 25], distance: 5 },
      { name: "overlap distance reaches exposed union boundary", xy: [9.5, 25], distance: 5 },
      { name: "touching polygon shared edge", xy: [12, 45], distance: 5 },
      { name: "left of touching polygon edge", xy: [11.75, 45], distance: 5 },
      { name: "right of touching polygon edge", xy: [12.25, 45], distance: 5 },
    ];
    let rows;
    try {
      const exported = {
        terrain: {
          ...JSON.parse(view.terrain_grid()),
          pageIds: view.terrain_page_ids(),
          heights: view.terrain_heights(),
        },
        positions: view.terrain_positions(),
        indices: view.terrain_indices(),
        triangleSurfaces: view.terrain_triangle_surfaces(),
        props: view.props(),
        buildings: JSON.parse(view.buildings()),
        water: view.water(),
        forests: view.forests(),
        foliage: view.foliage(),
        surfaceStrokes: view.surface_strokes(),
        surfaceTriangles: view.surface_triangles(),
        surfaceBoundaries: view.surface_boundaries(),
      };
      const layout = JSON.parse(wasm.world_layout(JSON.stringify(rules.VILLAGE_RULES)));
      const world = mesh.buildWorldLayers(exported, layout, biome.default, "surface");
      const source = terrain.createTerrainSource(root, registry);
      await source.ready();
      source.set(world.terrain);
      const queryLayout = tgpu.bindGroupLayout({
        points: { storage: (n) => d.arrayOf(d.vec2f, n), access: "readonly" },
        output: { storage: (n) => d.arrayOf(d.vec4f, n), access: "mutable" },
      });
      const groundSite = terrain.groundSite;
      const groundColour = terrain.groundColour;
      const kernel = tgpu
        .computeFn({ in: { gid: d.builtin.globalInvocationId }, workgroupSize: [1] })(`{
        let xy=queryLayout.$.points[gid.x];let site=groundSite(xy);let actual=groundColour(xy,0.1,site,-1e9);let interior=groundColour(xy,0.1,vec4f(site.xy,5.0,site.w),-1e9);queryLayout.$.output[gid.x]=vec4f(site.z,actual.w,interior.w,site.w);
      }`)
        .$uses({ queryLayout, groundSite, groundColour });
      const pipeline = root.createComputePipeline({ compute: kernel });
      await pipeline.initAsync();
      const input = registry.buffer({
        size: cases.length * 8,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      });
      const output = registry.buffer({
        size: cases.length * 16,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
      });
      const read = registry.buffer({
        size: cases.length * 16,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
      });
      device.queue.writeBuffer(input, 0, Float32Array.from(cases.flatMap((c) => c.xy)));
      const encoder = root["~unstable"].createCommandEncoder();
      pipeline
        .with(source.group)
        .with(root.createBindGroup(queryLayout, { points: input, output }))
        .with(encoder)
        .dispatchWorkgroups(cases.length);
      root.unwrap(encoder).copyBufferToBuffer(output, 0, read, 0, cases.length * 16);
      encoder.submit();
      await read.mapAsync(GPUMapMode.READ);
      try {
        const values = new Float32Array(read.getMappedRange());
        rows = cases.map((c, i) => ({
          ...c,
          actual: values[i * 4],
          roughness: values[i * 4 + 1],
          interiorRoughness: values[i * 4 + 2],
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
    return { rows, validation, final, compiled };
  }, repo);
  const compiled = result.compiled;
  delete result.compiled;
  result.shaderHashes = compiled.map((s) => createHash("sha256").update(s).digest("hex"));
  for (let i = 0; i < compiled.length; i++)
    await writeFile(ctx.evidencePath(`surface-query-${i}.wgsl`), compiled[i]);
  await writeFile(ctx.evidencePath("surface-query.json"), JSON.stringify(result, null, 2));
  ctx.check(
    "polygon paving uses exposed union boundaries across diagonals, overlaps and touching edges",
    result.rows.every(
      (r) => r.actual === r.distance && (r.distance !== 5 || r.roughness === r.interiorRoughness),
    ) &&
      result.validation.length === 0 &&
      Object.values(result.final).every((value) => value === 0),
    JSON.stringify(result),
  );
  await page.close();
}
