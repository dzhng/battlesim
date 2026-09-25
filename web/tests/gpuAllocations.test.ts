// @vitest-environment node
import { expect, test } from "vitest";
import { trackGpuAllocations } from "@packages/renderer-core/src/gpuAllocations.ts";

test("counts live buffers and textures through create and destroy", () => {
  const resource = () => ({ destroy() {} });
  const device = { createBuffer: resource, createTexture: resource } as unknown as GPUDevice;
  const live = trackGpuAllocations(device);
  const a = device.createBuffer({ size: 4, usage: 0 });
  device.createBuffer({ size: 4, usage: 0 });
  const t = device.createTexture({ size: [1, 1], format: "r8unorm", usage: 0 });
  expect(live()).toEqual({ buffers: 2, textures: 1 });
  a.destroy();
  t.destroy();
  expect(live()).toEqual({ buffers: 1, textures: 0 });
});
