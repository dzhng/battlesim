// @vitest-environment node
import { expect, test } from "vitest";
import {
  textureByteSize,
  trackGpuAllocations,
} from "@packages/renderer-core/src/gpuAllocations.ts";

import { fakeGpuDevice } from "./support/gpuDevice";

test("counts live buffers, their bytes and textures through create and destroy", () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const live = trackGpuAllocations(device);
  const a = device.createBuffer({ size: 4, usage: 0 });
  const b = device.createBuffer({ size: 16, usage: 0 });
  const t = device.createTexture({ size: [2, 2], format: "r8unorm", usage: 0 });
  expect(live()).toMatchObject({ buffers: 2, textures: 1, bufferBytes: 20, textureBytes: 4 });
  a.destroy();
  t.destroy();
  expect(live()).toEqual({ buffers: 1, textures: 0, bufferBytes: 16, textureBytes: 0 });
  expect([...native.resources]).toEqual([b]);
  expect(native.destroyCalls).toBe(2);
});

test("allocations made before a mark are counted apart from those made after it", () => {
  const { device } = fakeGpuDevice();
  const live = trackGpuAllocations(device);
  const old = device.createBuffer({ size: 4, usage: 0 });
  const mark = live.created();
  // What arrives after the mark (the next owner's) never counts against it.
  device.createBuffer({ size: 8, usage: 0 });
  device.createTexture({ size: [1, 1], format: "r8unorm", usage: 0 });
  expect(live(mark)).toEqual({ buffers: 1, textures: 0, bufferBytes: 4, textureBytes: 0 });
  old.destroy();
  expect(live(mark)).toEqual({ buffers: 0, textures: 0, bufferBytes: 0, textureBytes: 0 });
  expect(live()).toMatchObject({ buffers: 1, textures: 1 });
});

test("texture bytes count every sample, layer and mip level", () => {
  // The frame's 1080p MSAA HDR colour: 8 bytes a texel, four samples.
  expect(
    textureByteSize({
      size: [1920, 1080],
      format: "rgba16float",
      sampleCount: 4,
      usage: 0,
    }),
  ).toBe(1920 * 1080 * 8 * 4);
  // Four 2048² depth32float cascade layers.
  expect(textureByteSize({ size: [2048, 2048, 4], format: "depth32float", usage: 0 })).toBe(
    2048 * 2048 * 4 * 4,
  );
  // A full mip chain of 8×4 rgba8: 8×4 + 4×2 + 2×1 + 1×1 texels.
  expect(
    textureByteSize({
      size: { width: 8, height: 4 },
      format: "rgba8unorm",
      mipLevelCount: 4,
      usage: 0,
    }),
  ).toBe((32 + 8 + 2 + 1) * 4);
  // A 3D texture halves its depth with each level too.
  expect(
    textureByteSize({
      size: [4, 4, 4],
      format: "rg16float",
      dimension: "3d",
      mipLevelCount: 2,
      usage: 0,
    }),
  ).toBe((64 + 8) * 4);
});

test("a format the table does not size is an error, not a silent zero", () => {
  expect(() => textureByteSize({ size: [4, 4], format: "bc1-rgba-unorm", usage: 0 })).toThrow(
    /bc1-rgba-unorm/,
  );
});

test("tracking a device twice shares one tracker, so resources are not double-wrapped", () => {
  const native = fakeGpuDevice();
  const { device } = native;
  const first = trackGpuAllocations(device);
  const second = trackGpuAllocations(device);
  const texture = device.createTexture({ size: [1, 1], format: "rgba8unorm", usage: 0 });
  expect(first()).toEqual(second());
  expect(first().textures).toBe(1);
  texture.destroy();
  expect(native.resources.size).toBe(0);
  expect(native.destroyCalls).toBe(1);
  expect(first().textures).toBe(0);
});
