// @vitest-environment node
import { expect, test } from "vitest";
import {
  textureByteSize,
  trackGpuAllocations,
} from "@packages/renderer-core/src/gpuAllocations.ts";

/** A device whose factories hand back destroyable stand-ins. */
function fakeDevice() {
  const resource = (d: { size: unknown }) => ({ size: d.size, destroy() {} });
  return { createBuffer: resource, createTexture: resource } as unknown as GPUDevice;
}

test("counts live buffers, their bytes and textures through create and destroy", () => {
  const device = fakeDevice();
  const live = trackGpuAllocations(device);
  const a = device.createBuffer({ size: 4, usage: 0 });
  device.createBuffer({ size: 16, usage: 0 });
  const t = device.createTexture({ size: [2, 2], format: "r8unorm", usage: 0 });
  expect(live()).toMatchObject({ buffers: 2, textures: 1, bufferBytes: 20, textureBytes: 4 });
  a.destroy();
  t.destroy();
  expect(live()).toEqual({ buffers: 1, textures: 0, bufferBytes: 16, textureBytes: 0 });
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
  const device = fakeDevice();
  const first = trackGpuAllocations(device);
  const second = trackGpuAllocations(device);
  device.createTexture({ size: [1, 1], format: "rgba8unorm", usage: 0 });
  expect(first()).toEqual(second());
  expect(first().textures).toBe(1);
});
