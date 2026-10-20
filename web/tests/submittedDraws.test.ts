// @vitest-environment node
import { expect, test } from "vitest";
import {
  createBuildingLayer,
  emptyBuildingWalkStats,
} from "@packages/battle-renderer/src/models/buildingLayer";
import { surfaceFlags } from "@packages/battle-renderer/src/models/surfaceParts";
import {
  createSubmittedFrame,
  submittedDraws,
} from "@packages/battle-renderer/src/frame/submitted";
import { drawTerrainRegions } from "@packages/battle-renderer/src/frame/terrainRegions";
import { MeshSlot } from "@packages/battle-renderer/src/frame/geometry";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import {
  Batch,
  BATCH_WORDS,
  INSTANCE_WORDS,
  VIEW_PASS,
  CAST_PASS,
} from "@packages/battle-renderer/src/records.gen";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { fakeGpuDevice } from "./support/gpuDevice";

function fixture() {
  const { device } = fakeGpuDevice();
  Object.assign(device, { queue: { writeBuffer() {} } });
  const registry = new GpuRegistry(device);
  const calls: number[][] = [];
  const encoder = {
    with() {
      return this;
    },
    draw(...args: number[]) {
      calls.push(args);
    },
    drawIndexed(...args: number[]) {
      calls.push(args);
    },
  };
  const sum = () => ({
    triangles: calls.reduce((n, [v, i]) => n + Math.floor(v / 3) * i, 0),
    instances: calls.reduce((n, [, i]) => n + i, 0),
    draws: calls.length,
  });
  return { device, registry, encoder, calls, sum };
}

function batches(rows: { pass: number; count: number; appearance?: number }[]) {
  const u32 = new Uint32Array(rows.length * BATCH_WORDS);
  let first = 0;
  rows.forEach((row, i) => {
    const at = i * BATCH_WORDS;
    u32[at + Batch.appearance] = row.appearance ?? 0;
    u32[at + Batch.key] = i;
    u32[at + Batch.pass] = row.pass;
    u32[at + Batch.count] = row.count;
    u32[at + Batch.first] = first;
    first += row.count;
  });
  return {
    batches: { u32, count: rows.length },
    instances: { f32: new Float32Array(first * INSTANCE_WORDS) },
  };
}

test("building submissions include raw consecutive draws and only enabled surface ranges", () => {
  const f = fixture();
  const layer = createBuildingLayer(f.device, f.registry);
  const mesh = {
    vertices: {} as GPUBuffer,
    indices: {} as GPUBuffer,
    count: 30,
    parts: {
      cutout: { first: 0, count: 3 },
      opaque: { first: 3, count: 12 },
      room: { first: 15, count: 6 },
      blended: { first: 21, count: 9 },
      solid: { first: 3, count: 18 },
    },
  };
  layer.setWorld([{ name: "kit", state: "wall", placed: true, draws: 0 }] as never);
  layer.setSource(() => mesh);
  layer.setFrame(
    batches([
      { pass: VIEW_PASS, count: 2 },
      { pass: VIEW_PASS, count: 5 },
      { pass: CAST_PASS, count: 3 },
    ]) as never,
    emptyBuildingWalkStats(),
  );
  const frame = createSubmittedFrame();
  const hidden = surfaceFlags();
  const bind = () => f.encoder;
  layer.draw(bind, f.encoder as never, "solid", hidden, frame.depth.buildings);
  layer.draw(bind, f.encoder as never, "cutout", hidden, frame.depth.buildings);
  expect(f.calls).toEqual([
    [18, 2, 3, 0, 0],
    [18, 5, 3, 0, 2],
    [3, 2, 0, 0, 0],
    [3, 5, 0, 0, 2],
  ]);
  expect(frame.depth.buildings).toEqual(f.sum());
  f.calls.length = 0;
  hidden.room = true;
  for (const surface of ["opaque", "room", "cutout", "blended"] as const)
    layer.draw(bind, f.encoder as never, surface, hidden, frame.color.buildings);
  expect(frame.color.buildings).toEqual(f.sum());
  expect(frame.color.buildings.triangles).toBe(56);
  for (const index of [0, 1]) {
    f.calls.length = 0;
    const counter = frame.cascade(index).buildings;
    layer.drawCasters(bind, f.encoder as never, "solid", hidden, counter);
    layer.drawCasters(bind, f.encoder as never, "cutout", hidden, counter);
    expect(counter).toEqual(f.sum());
    expect(counter.triangles).toBe(15);
  }
  const prior = frame.snapshot();
  frame.reset();
  layer.setShown(false);
  f.calls.length = 0;
  layer.draw(bind, f.encoder as never, "solid", hidden, frame.depth.buildings);
  expect(frame.depth.buildings).toEqual(f.sum());
  expect(frame.snapshot().cascades).toEqual([]);
  expect(prior.cascades[0].buildings.triangles).toBe(15);
  expect(frame.cascade(0).buildings).toEqual(submittedDraws());
  f.registry.release();
});

test("terrain regions count every issued mesh, skip offscreen regions and submit before atlas rewrite", () => {
  const f = fixture();
  const events: string[] = [];
  const root = {
    createBuffer() {
      return {
        $usage() {
          return this;
        },
        write() {},
        destroy() {},
      };
    },
    "~unstable": {
      createCommandEncoder() {
        return {
          beginRenderPass() {
            return {
              setScissorRect() {},
              end() {
                events.push("end");
              },
            };
          },
          submit() {
            events.push("submit");
          },
        };
      },
    },
  };
  const mesh = new MeshSlot(root as never, f.registry, {} as never);
  mesh.set(new Float32Array(21 * VERTEX_FLOATS));
  const texture = { createView: () => ({}) };
  const targets = {
    width: 40,
    height: 30,
    hdrMsaa: texture,
    lit: texture,
    fogMaskMsaa: texture,
    fogMask: texture,
    depth: texture,
  };
  const submitted = submittedDraws();
  const first = drawTerrainRegions(
    root as never,
    targets as never,
    [
      [0, 0, 1, 1],
      [1, 0, 2, 1],
      [2, 0, 3, 1],
    ],
    (region) => (region[0] === 1 ? null : [0, 0, 40, 30]),
    () => events.push("prepare"),
    () => {
      mesh.draw(f.encoder as never, submitted, Uint32Array.of(3, 6, 15, 3));
      events.push("draw");
    },
  );
  expect(first).toBe(false);
  expect(events).toEqual(["prepare", "draw", "end", "submit", "prepare", "draw", "end", "submit"]);
  expect(submitted).toEqual(f.sum());
  expect(f.calls).toEqual([
    [6, 1, 3],
    [3, 1, 15],
    [6, 1, 3],
    [3, 1, 15],
  ]);
  expect(submitted.triangles).toBe(6);
  f.registry.release();
});

// Only the GPU edge is faked: production installation, batch selection and
// surface routing decide which draw calls reach this encoder.
import { createModelLayer } from "@packages/battle-renderer/src/models/modelLayer";
import { createSceneryLayer } from "@packages/battle-renderer/src/frame/sceneryLayer";
import {
  NONE,
  HIGH_PASS,
  GHOST_PASS,
  FOG_FACES,
  FOG_UNITS,
  JOINT_WORDS,
} from "@packages/battle-renderer/src/records.gen";
import type { StaticBundle } from "@packages/scene-assets/src/schema";

function art(): StaticBundle {
  const bounds: StaticBundle["bounds"] = { min: [0, 0, 0], max: [1, 1, 1] };
  const mesh = {
    positions: new Float32Array(9),
    normals: new Int16Array(12),
    uvs: new Float32Array(6),
    colors: new Uint8Array(12).fill(255),
    indices: new Uint16Array(12),
    draws: [
      { material: 0, first: 0, count: 9 },
      { material: 1, first: 9, count: 3 },
    ],
  };
  return {
    kind: "static",
    states: [{ name: "summer", bounds, tiers: Array.from({ length: 4 }, () => mesh) }],
    bounds,
    materials: ["opaque", "cutout"].map((kind) => ({
      name: kind,
      base_color: [1, 1, 1, 1],
      metallic: 0,
      roughness: 1,
      tint: 0,
      coverage: { kind },
    })),
    textures: [],
  } as StaticBundle;
}

function layerFixture() {
  const f = fixture();
  Object.assign(f.device, {
    createSampler: () => ({}),
    createBindGroup: () => ({}),
    limits: { maxTextureArrayLayers: 256 },
  });
  const root = {
    device: f.device,
    unwrap: (x: unknown) => x,
    createBuffer() {
      return {
        $usage() {
          return this;
        },
        write() {},
        destroy() {},
      };
    },
    createRenderPipeline() {
      return {
        ...f.encoder,
        initAsync() {},
        $name() {
          return this;
        },
        withIndexBuffer() {
          return this;
        },
      };
    },
  };
  return {
    ...f,
    root,
    bound: {
      ...f.encoder,
      withIndexBuffer() {
        return this;
      },
    },
  };
}

test("models inclusive of props count actual filtered surfaces and independent high-air/caster draws", async () => {
  const f = layerFixture();
  const models = await createModelLayer(f.root as never, f.registry, "{}");
  await models.setAppearances({
    appearances: new Map([["shape", { bundle: art() }]]),
    skeletons: new Map(),
  } as never);
  models.setWorld([
    { name: "shape", state: "summer", lies: NONE, placed: false, draws: 0 },
    { name: "shape", state: "summer", lies: NONE, placed: true, draws: 7 },
  ]);
  const frame = batches([
    { pass: VIEW_PASS, count: 2 },
    { pass: HIGH_PASS, count: 3 },
    { pass: GHOST_PASS, count: 4 },
    { pass: VIEW_PASS, count: 5, appearance: 1 },
  ]);
  [FOG_UNITS, FOG_UNITS, FOG_UNITS, FOG_FACES].forEach((fog, i) => {
    frame.batches.u32[i * BATCH_WORDS + Batch.fog] = fog;
    frame.batches.u32[i * BATCH_WORDS + Batch.caster] = i === 2 ? NONE : 0;
  });
  models.setStaticFrame(frame as never, emptyBuildingWalkStats());
  models.setFrame({
    ...frame,
    instances: { ...frame.instances, count: 14 },
    palettes: { count: 1, f32: new Float32Array(JOINT_WORDS) },
    counts: { u32: new Uint32Array(32) },
  } as never);
  models.setHighApart(true);
  const counters = createSubmittedFrame();
  models.draw(f.bound as never, "solid", undefined, false, counters.depth.modelsIncludingProps);
  models.draw(f.bound as never, "cutout", undefined, false, counters.depth.modelsIncludingProps);
  expect(counters.depth.modelsIncludingProps).toEqual(f.sum());
  expect(counters.depth.modelsIncludingProps).toEqual({ triangles: 28, instances: 14, draws: 4 });
  f.calls.length = 0;
  models.setSurfaceShown("cutout", false);
  models.draw(f.bound as never, "opaque", "units", true, counters.highColor.modelsIncludingProps);
  models.draw(f.bound as never, "cutout", "units", true, counters.highColor.modelsIncludingProps);
  expect(counters.highColor.modelsIncludingProps).toEqual(f.sum());
  expect(counters.highColor.modelsIncludingProps.triangles).toBe(9);
  f.calls.length = 0;
  const cascade = counters.cascade(2).modelsIncludingProps;
  models.drawCasters(f.bound as never, "solid", cascade);
  models.drawCasters(f.bound as never, "cutout", cascade);
  expect(cascade).toEqual(f.sum());
  expect(cascade.triangles).toBe(30);
  f.registry.release();
});

test("scenery counts retained batches, felled pairs, backdrop and hidden trees at real draws", async () => {
  const f = layerFixture();
  const scenery = await createSceneryLayer(f.root as never, f.registry, {} as never);
  scenery.set({ appearances: new Map([["shape", art()]]) } as never);
  scenery.setWorld(
    [1, 3, 4, 5].map((draws) => ({
      name: "shape",
      state: "summer",
      lies: NONE,
      placed: true,
      draws,
    })),
  );
  const frame = batches([
    { pass: VIEW_PASS, count: 2 },
    { pass: VIEW_PASS, count: 3, appearance: 1 },
    { pass: VIEW_PASS, count: 4, appearance: 2 },
    { pass: VIEW_PASS, count: 6, appearance: 3 },
    { pass: CAST_PASS, count: 5 },
  ]);
  scenery.setFrame(frame as never, {} as never);
  const counters = createSubmittedFrame();
  scenery.encodeDepth({} as never, {} as never, counters.depth.scenery);
  expect(counters.depth.scenery).toEqual(f.sum());
  expect(counters.depth.scenery).toEqual({ triangles: 48, instances: 12, draws: 4 });
  // An unchanged batch retains the uploaded count even if a new header differs.
  for (let i = 0; i < frame.batches.count; i++) {
    frame.batches.u32[i * BATCH_WORDS + Batch.first] = NONE;
    frame.batches.u32[i * BATCH_WORDS + Batch.count] = 200;
  }
  scenery.setFrame(frame as never, {} as never);
  f.calls.length = 0;
  scenery.encode({} as never, {} as never, {} as never, counters.color.scenery);
  expect(counters.color.scenery).toEqual(f.sum());
  expect(counters.color.scenery).toEqual(counters.depth.scenery);
  f.calls.length = 0;
  scenery.encodeShadows({} as never, {} as never, counters.cascade(0).scenery);
  expect(counters.cascade(0).scenery).toEqual(f.sum());
  expect(counters.cascade(0).scenery.triangles).toBe(20);
  counters.reset();
  scenery.setTreesShown(false);
  f.calls.length = 0;
  scenery.encodeDepth({} as never, {} as never, counters.depth.scenery);
  expect(counters.depth.scenery).toEqual(f.sum());
  expect(counters.depth.scenery).toEqual({ triangles: 16, instances: 4, draws: 1 });
  f.registry.release();
});

test("explicit lab card tier draws the prepared quad, and missing cards retain the mesh", async () => {
  await (await import("@web/battle/sim/presentation")).loadPresentation();
  const f = layerFixture();
  Object.assign(f.device.queue, { writeTexture() {} });
  Object.assign(f.root, {
    createTexture: () => ({
      $usage() {
        return this;
      },
      createView() {
        return {};
      },
      destroy() {},
    }),
    createSampler: () => ({}),
    createBindGroup: () => ({}),
  });
  const models = await createModelLayer(f.root as never, f.registry, "{}");
  const bundle = art();
  bundle.states[0].name = "default";
  bundle.materials = bundle.materials.map((m) => ({ ...m, coverage: { kind: "opaque" } }));
  await models.setAppearances({
    appearances: new Map([["shape", { bundle, footprint: [1, 1, 1], farCardPx: 2 }]]),
    skeletons: new Map(),
  } as never);
  const instance = {
    appearance: "shape",
    pose: { kind: "static" as const, state: "default" },
    x: 0,
    y: 0,
    z: 0,
    yaw: 0,
    tier: 4,
  };
  models.packExact([instance]);
  models.draw(f.bound as never, "opaque");
  expect(f.sum().triangles).toBe(4);
  f.calls.length = 0;
  const { CARD_SPEC } = await import("@packages/scene-assets/src/cards");
  models.setCards([
    {
      pose: instance.pose,
      atlas: {
        appearance: "shape",
        bundle: "a".repeat(64),
        state: "default",
        spec: CARD_SPEC,
        width: 512,
        height: 128,
        center: [0, 0, 0],
        radius: 1,
        albedo: new Uint8Array(512 * 128 * 4),
        normal: new Uint8Array(512 * 128 * 4),
      },
    },
  ]);
  models.packExact([instance]);
  models.drawCards(f.bound as never, "faces");
  expect(f.sum()).toEqual({ triangles: 2, instances: 1, draws: 1 });
  models.dispose();
  f.registry.release();
});
