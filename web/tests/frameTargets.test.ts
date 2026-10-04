// @vitest-environment node
// The battle frame's GPU lifetimes: size-dependent targets rebuilt on resize,
// replaceable buffers, and disposal all return the device to its baseline.
import { expect, test } from "vitest";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry.ts";
import { allocateFrameTargets, SizedTargets } from "@packages/battle-renderer/src/frame/targets.ts";
import { trackGpuAllocations } from "@packages/renderer-core/src/gpuAllocations.ts";

import { fakeGpuDevice } from "./support/gpuDevice";

/** Targets plus a stand-in for the post chain's own intermediates, built a
 *  tick later like the real async post pipelines. */
async function build(scope: GpuRegistry, width: number, height: number) {
  await Promise.resolve();
  const targets = allocateFrameTargets(scope, width, height);
  scope.texture({ size: [width >> 1, height >> 1], format: "rgba16float", usage: 0 });
  return targets;
}

test("resize swaps targets whole and returns to the same bytes at the same size", async () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const live = trackGpuAllocations(device);
  const baseline = live();
  const registry = new GpuRegistry(device);
  const targets = new SizedTargets(registry, build);

  await targets.ensure(1920, 1080);
  const full = live();
  const fullResources = new Set(native.resources);
  expect(targets.current?.width).toBe(1920);
  await targets.ensure(1280, 720);
  expect(live().textureBytes).toBeLessThan(full.textureBytes);
  for (const resource of fullResources) expect(native.resources.has(resource)).toBe(false);
  await targets.ensure(1920, 1080);
  expect(live()).toEqual(full);
  expect(native.resources.size).toBe(full.textures);

  registry.release();
  expect(native.resources.size).toBe(0);
  expect(live()).toEqual(baseline);
});

test("the frame allocates and accounts for all four HDR samples", () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  const targets = allocateFrameTargets(registry, 1920, 1080);
  const allocated = live().textureBytes;
  expect(native.resources.has(targets.hdrMsaa)).toBe(true);
  targets.hdrMsaa.destroy();
  expect(native.resources.has(targets.hdrMsaa)).toBe(false);
  expect(allocated - live().textureBytes).toBe(1920 * 1080 * 8 * 4);
  registry.release();
  expect(native.resources.size).toBe(0);
  expect(live().textureBytes).toBe(0);
});

test("a resize that finishes after a newer one never replaces it", async () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  let finishOlder!: () => void;
  const olderBuild = new Promise<void>((resolve) => {
    finishOlder = resolve;
  });
  const targets = new SizedTargets(registry, async (scope, width, height) => {
    const value = await build(scope, width, height);
    if (width === 800) await olderBuild;
    return value;
  });
  const stale = targets.ensure(800, 600);
  await targets.ensure(1024, 768);
  const latest = targets.current;
  expect(latest).toMatchObject({ width: 1024, height: 768 });
  const overlapping = live();
  const latestResources = Object.values(latest!).filter((value) => typeof value === "object");
  finishOlder();
  await stale;
  expect(targets.current).toBe(latest);
  for (const resource of latestResources) expect(native.resources.has(resource)).toBe(true);
  expect(live().textureBytes).toBeLessThan(overlapping.textureBytes);
  const settled = live();
  expect(native.resources.size).toBe(settled.textures);
  await targets.ensure(1024, 768);
  expect(live()).toEqual(settled);
  registry.release();
  expect(native.resources.size).toBe(0);
  expect(live().textures).toBe(0);
});

test("targets still building when the frame is disposed are freed, not leaked", async () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  let finish!: () => void;
  let began!: () => void;
  const building = new Promise<void>((resolve) => {
    began = resolve;
  });
  const held = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const targets = new SizedTargets(registry, async (scope, width, height) => {
    const value = await build(scope, width, height);
    began();
    await held;
    return value;
  });
  const pending = targets.ensure(640, 480);
  await building;
  expect(native.resources.size).toBeGreaterThan(0);
  registry.release();
  expect(native.resources.size).toBe(0);
  finish();
  await pending;
  expect(native.resources.size).toBe(0);
  expect(live()).toMatchObject({ textures: 0, buffers: 0 });
  expect(targets.current).toBeNull();
});

test("a slot destroys what it held when refilled and when its scope is released", () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  const slot = registry.slot<GPUBuffer>();
  const old = device.createBuffer({ size: 64, usage: 0 });
  const current = device.createBuffer({ size: 128, usage: 0 });
  slot.set(old);
  slot.set(current);
  expect([...native.resources]).toEqual([current]);
  expect(live()).toMatchObject({ buffers: 1, bufferBytes: 128 });
  registry.release();
  expect(native.resources.size).toBe(0);
  expect(live()).toMatchObject({ buffers: 0, bufferBytes: 0 });
});
