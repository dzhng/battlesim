// @vitest-environment node
import { expect, test } from "vitest";
import {
  createPropBatches,
  setPropBatches,
  clearPropBatches,
  drawnProps,
} from "@packages/battle-renderer/src/models/propBatches";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import {
  frameViews,
  Batch,
  BATCH_WORDS,
  INSTANCE_WORDS,
  VIEW_PASS,
  CAST_PASS,
  NONE,
} from "@packages/battle-renderer/src/records.gen";

test("held props preserve independent camera and caster batches through reuse and removal", () => {
  const uploads = new Map<GPUBuffer, number[]>();
  const device = {
    createBuffer: (d: GPUBufferDescriptor) => ({ size: d.size, destroy() {} }),
    createTexture: () => {
      throw new Error("no textures needed");
    },
    queue: {
      writeBuffer: (buffer: GPUBuffer, _offset: number, data: Float32Array) =>
        uploads.set(buffer, [...data]),
    },
  } as unknown as GPUDevice;
  const registry = new GpuRegistry(device);
  const state = createPropBatches();
  const world = [{ name: "test_prop", state: "default", lies: NONE, placed: true, draws: 7 }];
  const memory = new ArrayBuffer(1024);
  const header = new Uint32Array(memory, 0, 28);
  header.set([128, 2, 256, 2], 4);
  const frame = frameViews(memory, 0);
  for (const [i, pass, caster, x] of [
    [0, VIEW_PASS, NONE, 0],
    [1, CAST_PASS, 2, -80],
  ]) {
    const o = i * BATCH_WORDS;
    frame.batches.u32[o + Batch.key] = (2 << 4) | pass;
    frame.batches.u32[o + Batch.tier] = 2;
    frame.batches.u32[o + Batch.pass] = pass;
    frame.batches.u32[o + Batch.caster] = caster;
    frame.batches.u32[o + Batch.cascade_mask] = pass === CAST_PASS ? 2 : 0;
    frame.batches.u32[o + Batch.first] = i;
    frame.batches.u32[o + Batch.count] = 1;
    frame.instances.f32[i * INSTANCE_WORDS] = x;
  }
  expect(setPropBatches(state, frame, world, registry, device)).toBe(true);
  expect(state.listed).toMatchObject([
    { pass: VIEW_PASS, caster: NONE },
    { pass: CAST_PASS, caster: 2, cascadeMask: 2 },
  ]);
  expect(drawnProps(state, world)).toEqual([{ appearance: "test_prop", levels: [0, 0, 1, 0] }]);
  const buffers = state.listed.map((h) => h.buffer!);
  expect(buffers.map((b) => uploads.get(b)![0])).toEqual([0, -80]);
  frame.batches.u32[Batch.first] = NONE;
  frame.batches.u32[BATCH_WORDS + Batch.first] = NONE;
  expect(setPropBatches(state, frame, world, registry, device)).toBe(false);
  expect(state.listed.map((h) => h.buffer)).toEqual(buffers);
  header[5] = 1;
  expect(setPropBatches(state, frameViews(memory, 0), world, registry, device)).toBe(true);
  expect(state.listed).toMatchObject([{ pass: VIEW_PASS, caster: NONE }]);
  expect(registry.stats().buffers).toBe(1);
  clearPropBatches(state);
  expect(registry.stats().buffers).toBe(0);
  registry.release();
});

test("card records can install, replace and remove a layer without changing identity or casters", async () => {
  const { setPropCardLayer } = await import("@packages/battle-renderer/src/models/propBatches");
  const uploads = new Map<GPUBuffer, number[]>();
  const device = {
    createBuffer: (d: GPUBufferDescriptor) => ({ size: d.size, destroy() {} }),
    createTexture: () => {
      throw new Error("not needed");
    },
    queue: {
      writeBuffer: (b: GPUBuffer, _offset: number, data: Float32Array) => uploads.set(b, [...data]),
    },
  } as unknown as GPUDevice;
  const registry = new GpuRegistry(device);
  const state = createPropBatches();
  const world = [{ name: "test_prop", state: "default", lies: NONE, placed: true, draws: 7 }];
  const memory = new ArrayBuffer(1024);
  new Uint32Array(memory, 0, 28).set([128, 2, 256, 2], 4);
  const frame = frameViews(memory, 0);
  const original = [10, 20, 3, 0.7, 0, 0, 0, 0, 0.2, 0.4, 0.6, -1, 2, 3, 0.5, 0];
  for (const [i, pass, tier] of [
    [0, VIEW_PASS, 4],
    [1, CAST_PASS, 3],
  ]) {
    const o = i * BATCH_WORDS;
    frame.batches.u32[o + Batch.key] = i;
    frame.batches.u32[o + Batch.tier] = tier;
    frame.batches.u32[o + Batch.pass] = pass;
    frame.batches.u32[o + Batch.first] = i;
    frame.batches.u32[o + Batch.count] = 1;
    frame.instances.f32.set(original, i * INSTANCE_WORDS);
  }
  setPropBatches(state, frame, world, registry, device);
  const [card, caster] = state.listed;
  for (const layer of [3, 7, -1]) {
    setPropCardLayer(card, layer, device);
    const expected = Array.from(new Float32Array(original));
    expected[11] = layer;
    expect(uploads.get(card.buffer!)).toEqual(expected);
    expect(drawnProps(state, world)).toEqual([
      { appearance: "test_prop", levels: layer < 0 ? [0, 0, 0, 1] : [0, 0, 0, 0, 1] },
    ]);
    expect(uploads.get(caster.buffer!)).toEqual(Array.from(new Float32Array(original)));
  }
  clearPropBatches(state);
  expect(registry.stats().buffers).toBe(0);
});
