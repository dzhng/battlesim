// Exercise initialization through the real frame factory, including cleanup
// and retry on the same device. No test hook enters the production sampler.
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
export async function groundFilterAdmission(ctx) {
  const page = await ctx.newPage();
  await page.goto(new URL("/", ctx.url).href);
  const repo = new URL("../../", import.meta.url).pathname;
  const result = await page.evaluate(async (repo) => {
    const file = (p) => `/@fs/${repo}${p}`;
    const [frameModule, mesh, light, fog, overlay, models, rules, biome, wasm, allocations] =
      await Promise.all([
        import(file("packages/battle-renderer/src/frame/battleFrame.ts")),
        import(file("packages/battle-renderer/src/worldMesh.ts")),
        import(file("apps/battle-lab/src/villageLight.ts")),
        import(file("apps/battle-lab/src/villageFog.ts")),
        import(file("apps/battle-lab/src/villageOverlay.ts")),
        import(file("apps/battle-lab/src/villageModels.ts")),
        import(file("apps/battle-lab/src/scenarios.ts")),
        import(file("fixtures/biomes/summer.json")),
        import("/src/wasm/game_wasm.js"),
        import(file("packages/renderer-core/src/gpuAllocations.ts")),
      ]);
    await wasm.default();
    const view = new wasm.WorldView(
      JSON.stringify({ size: [32, 32], height_grid_m: 4, fog_cell_m: 8, slope_cutoff_deg: 35 }),
      JSON.stringify(rules.VILLAGE_RULES),
    );
    const adapter = await navigator.gpu.requestAdapter();
    const device = await adapter.requestDevice();
    const live = allocations.trackGpuAllocations(device);
    const baseline = live();
    const options = {
      light: light.villageLight,
      fogGeometry: fog.villageFogGeometry,
      fogStyle: fog.villageFogStyle,
      overlayGlow: overlay.villageOverlayGlow,
      paint: overlay.villagePaint,
      models: models.villageModelDetail,
      world: mesh.buildWorldLayers(
        {
          terrain: {
            ...JSON.parse(view.terrain_grid()),
            pageIds: view.terrain_page_ids(),
            heights: view.terrain_heights(),
          },
          positions: view.terrain_positions(),
          indices: view.terrain_indices(),
          triangleSurfaces: view.terrain_triangle_surfaces(),
          props: view.props(),
          water: view.water(),
          forests: view.forests(),
          foliage: view.foliage(),
          surfaceStrokes: view.surface_strokes(),
          surfaceTriangles: view.surface_triangles(),
          surfaceBoundaries: view.surface_boundaries(),
        },
        JSON.parse(wasm.world_layout(JSON.stringify(rules.VILLAGE_RULES))),
        biome.default,
        "surface",
      ),
      instances: [],
      width: 64,
      height: 64,
    };
    const sampler = device.createSampler.bind(device);
    let rejected = "",
      returned = null;
    try {
      device.createSampler = (descriptor) =>
        sampler(
          descriptor?.label === "ground-scars"
            ? { ...descriptor, minFilter: "nearest", magFilter: "nearest" }
            : descriptor,
        );
      try {
        returned = await frameModule.createBattleFrame(device, "bgra8unorm", options);
      } catch (error) {
        rejected = error.message;
      }
      returned?.dispose();
      const afterFailure = live();
      device.createSampler = sampler;
      const recovered = await frameModule.createBattleFrame(device, "bgra8unorm", options);
      recovered.dispose();
      const afterRetry = live();
      // Use the production sampler and bindings. A profile-only shader can
      // miss cancellation that appears after this source is inlined.
      const terrainPath = file("packages/battle-renderer/src/frame/terrainMaterial.ts");
      const terrainText = await (await fetch(terrainPath)).text();
      const gpuUrl = terrainText.match(/from ["']([^"']*typegpu[^"']*)["']/)[1];
      const [{ tgpu, d }, { GpuRegistry }, terrain, { GroundView }] = await Promise.all([
        import(gpuUrl),
        import(file("packages/battle-renderer/src/frame/registry.ts")),
        import(terrainPath),
        import(file("web/src/battle/sim/ground.ts")),
      ]);
      const compiled = [],
        shader = device.createShaderModule.bind(device);
      device.createShaderModule = (descriptor) => {
        compiled.push(descriptor.code);
        return shader(descriptor);
      };
      const registry = new GpuRegistry(device),
        root = tgpu.initFromDevice({ device });
      registry.adopt(() => root.destroy());
      const failures = [];
      let queryCount = 0;
      let bitFailures = 0,
        linearFailures = 0,
        maxError = 0;
      try {
        const source = terrain.createTerrainSource(root, registry);
        await source.ready();
        source.set(options.world.terrain);
        const ground = new GroundView({ cellM: 1, cols: 600, rows: 600 }),
          runs = [];
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
        ground.applyRuns({
          epoch: 1,
          side: "blue",
          baseRevision: 0,
          revision: 1,
          full: true,
          runs: Float32Array.from(runs.flat()),
        });
        source.setGround(ground);
        source.prepareScars();
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
        const layout = tgpu.bindGroupLayout({
          points: { storage: (n) => d.arrayOf(d.vec2f, n), access: "readonly" },
          output: { storage: (n) => d.arrayOf(d.vec4f, n), access: "mutable" },
          reference: { texture: d.texture2d(d.f32) },
          sampler: { sampler: "filtering" },
        });
        // Frozen b538ecd coefficients, independent of the production helper.
        const original = tgpu
          .fn(
            [d.vec2f],
            d.vec4f,
          )(`(uv:vec2f)->vec4f {
          let size=vec2f(textureDimensions(layout.$.reference));let p=uv*size-0.5;let i=floor(p);let f=p-i;
          let f2=f*f;let f3=f2*f;let w0=(1.0-3.0*f+3.0*f2-f3)/6.0;let w1=(4.0-6.0*f2+3.0*f3)/6.0;let w2=(1.0+3.0*f+3.0*f2-3.0*f3)/6.0;let w3=f3/6.0;
          let g0=w0+w1;let g1=w2+w3;let h0=(i-0.5+w1/g0)/size;let h1=(i+1.5+w3/g1)/size;
          let a=textureSampleLevel(layout.$.reference,layout.$.sampler,vec2f(h0.x,h0.y),0.0);let b=textureSampleLevel(layout.$.reference,layout.$.sampler,vec2f(h1.x,h0.y),0.0);let c=textureSampleLevel(layout.$.reference,layout.$.sampler,vec2f(h0.x,h1.y),0.0);let other=textureSampleLevel(layout.$.reference,layout.$.sampler,vec2f(h1.x,h1.y),0.0);
          return g0.y*(g0.x*a+g1.x*b)+g1.y*(g0.x*c+g1.x*other);
        }`)
          .$uses({ layout });
        const sampleGroundCubic = terrain.sampleGroundCubic,
          sampleGroundLinear = terrain.sampleGroundLinear;
        const kernel = tgpu
          .computeFn({ in: { gid: d.builtin.globalInvocationId }, workgroupSize: [1] })(`{
          let uv=layout.$.points[gid.x];layout.$.output[gid.x*4u]=sampleGroundCubic(uv);layout.$.output[gid.x*4u+1u]=original(uv);layout.$.output[gid.x*4u+2u]=sampleGroundLinear(uv);layout.$.output[gid.x*4u+3u]=textureSampleLevel(layout.$.reference,layout.$.sampler,uv,0.0);
        }`)
          .$uses({ layout, sampleGroundCubic, sampleGroundLinear, original });
        const pipeline = root.createComputePipeline({ compute: kernel });
        await pipeline.initAsync();
        const reference = registry.texture({
          size: [600, 600],
          format: "rgba8unorm",
          usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
        });
        device.queue.writeTexture(
          { texture: reference },
          bytes,
          { bytesPerRow: 2400, rowsPerImage: 600 },
          [600, 600],
        );
        const input = registry.buffer({
          size: points.byteLength,
          usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        });
        const output = registry.buffer({
          size: n * 64,
          usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
        });
        const read = registry.buffer({
          size: n * 64,
          usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
        });
        device.queue.writeBuffer(input, 0, points);
        const encoder = root["~unstable"].createCommandEncoder();
        pipeline
          .with(source.group)
          .with(
            root.createBindGroup(layout, {
              points: input,
              output,
              reference: reference.createView(),
              sampler: sampler({
                minFilter: "linear",
                magFilter: "linear",
                addressModeU: "clamp-to-edge",
                addressModeV: "clamp-to-edge",
              }),
            }),
          )
          .with(encoder)
          .dispatchWorkgroups(n);
        root.unwrap(encoder).copyBufferToBuffer(output, 0, read, 0, n * 64);
        encoder.submit();
        await read.mapAsync(GPUMapMode.READ);
        try {
          const mapped = read.getMappedRange(),
            bits = new Uint32Array(mapped),
            values = new Float32Array(mapped);
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
          read.unmap();
        }
      } finally {
        registry.release();
        device.createShaderModule = shader;
      }
      return {
        rejected,
        baseline,
        afterFailure,
        afterRetry,
        queryCount,
        bitFailures,
        linearFailures,
        failures,
        maxError,
        afterQueries: live(),
        compiled,
      };
    } finally {
      device.createSampler = sampler;
      view.free();
      device.destroy();
    }
  }, repo);
  const modules = result.compiled;
  delete result.compiled;
  result.shaderHashes = modules.map((code) => createHash("sha256").update(code).digest("hex"));
  for (let i = 0; i < modules.length; i++)
    await writeFile(ctx.evidencePath(`ground-filter-query-${i}.wgsl`), modules[i]);
  result.sourceHashes = {};
  for (const name of ["scarFilter", "terrainMaterial", "scarTexture"])
    result.sourceHashes[name] = createHash("sha256")
      .update(await readFile(`${repo}packages/battle-renderer/src/frame/${name}.ts`))
      .digest("hex");
  await writeFile(ctx.evidencePath("ground-filter-query.json"), JSON.stringify(result, null, 2));
  ctx.check(
    "unsupported ground filtering rejects frame creation and releases every allocation",
    /ground.*filter.*compatib/i.test(result.rejected) &&
      JSON.stringify(result.afterFailure) === JSON.stringify(result.baseline),
    JSON.stringify(result),
  );
  ctx.check(
    "the production cubic sampler matches original dense hardware values bit for bit",
    result.bitFailures === 0 &&
      result.linearFailures === 0 &&
      JSON.stringify(result.afterQueries) === JSON.stringify(result.baseline),
    JSON.stringify({
      bitFailures: result.bitFailures,
      linearFailures: result.linearFailures,
      failures: result.failures,
      maxError: result.maxError,
      afterQueries: result.afterQueries,
      shaderHashes: result.shaderHashes,
    }),
  );
  ctx.check(
    "failed initialization does not poison a compatible retry on the same device",
    JSON.stringify(result.afterRetry) === JSON.stringify(result.baseline),
    JSON.stringify(result.afterRetry),
  );
  await page.close();
}
