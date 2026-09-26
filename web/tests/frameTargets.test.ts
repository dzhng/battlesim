// @vitest-environment node
// The battle frame's GPU lifetimes: size-dependent targets rebuilt on resize,
// replaceable buffers, and disposal all return the device to its baseline.
import { expect, test } from "vitest";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry.ts";
import { allocateFrameTargets, SizedTargets } from "@packages/battle-renderer/src/frame/targets.ts";
import { trackGpuAllocations } from "@packages/renderer-core/src/gpuAllocations.ts";

/** A device whose factories hand back destroyable stand-ins. */
function fakeDevice() {
  const resource = (d: { size: unknown; format?: string }) => ({
    size: d.size,
    format: d.format,
    destroy() {},
    createView: () => ({}),
  });
  return { createBuffer: resource, createTexture: resource } as unknown as GPUDevice;
}

/** Targets plus a stand-in for the post chain's own intermediates, built a
 *  tick later like the real async post pipelines. */
async function build(scope: GpuRegistry, width: number, height: number) {
  await Promise.resolve();
  const targets = allocateFrameTargets(scope, width, height);
  scope.texture({ size: [width >> 1, height >> 1], format: "rgba16float", usage: 0 });
  return targets;
}

test("resize swaps targets whole and returns to the same bytes at the same size", async () => {
  const device = fakeDevice();
  const live = trackGpuAllocations(device);
  const baseline = live();
  const registry = new GpuRegistry(device);
  const targets = new SizedTargets(registry, build);

  await targets.ensure(1920, 1080);
  const full = live();
  expect(targets.current?.width).toBe(1920);
  await targets.ensure(1280, 720);
  expect(live().textureBytes).toBeLessThan(full.textureBytes);
  await targets.ensure(1920, 1080);
  expect(live()).toEqual(full);

  registry.release();
  expect(live()).toEqual(baseline);
});

test("the 1080p frame's targets are sized in texture bytes", async () => {
  const device = fakeDevice();
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  allocateFrameTargets(registry, 1920, 1080);
  // HDR colour and depth are 4× MSAA; the overlay keeps its own MSAA target
  // so its lines stay out of post. At least the HDR MSAA colour alone:
  expect(live().textureBytes).toBeGreaterThanOrEqual(1920 * 1080 * 8 * 4);
  registry.release();
  expect(live().textureBytes).toBe(0);
});

test("a resize that finishes after a newer one never replaces it", async () => {
  const device = fakeDevice();
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  const targets = new SizedTargets(registry, build);
  const stale = targets.ensure(800, 600);
  const latest = targets.ensure(1024, 768);
  await Promise.all([stale, latest]);
  expect(targets.current).toMatchObject({ width: 1024, height: 768 });
  const settled = live();
  await targets.ensure(1024, 768);
  expect(live()).toEqual(settled);
  registry.release();
  expect(live().textures).toBe(0);
});

test("targets still building when the frame is disposed are freed, not leaked", async () => {
  const device = fakeDevice();
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  const targets = new SizedTargets(registry, build);
  const pending = targets.ensure(640, 480);
  registry.release();
  await pending.catch(() => {});
  expect(live()).toMatchObject({ textures: 0, buffers: 0 });
  expect(targets.current).toBeNull();
});

test("a slot destroys what it held when refilled and when its scope is released", () => {
  const device = fakeDevice();
  const live = trackGpuAllocations(device);
  const registry = new GpuRegistry(device);
  const slot = registry.slot<GPUBuffer>();
  slot.set(device.createBuffer({ size: 64, usage: 0 }));
  slot.set(device.createBuffer({ size: 128, usage: 0 }));
  expect(live()).toMatchObject({ buffers: 1, bufferBytes: 128 });
  registry.release();
  expect(live()).toMatchObject({ buffers: 0, bufferBytes: 0 });
});
