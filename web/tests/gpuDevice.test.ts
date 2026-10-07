// Device admission asks for the adapter's own texture-array layer limit:
// models hold one layer per distinct texture, and WebGPU's default (256) is
// far below what an Apple GPU grants.
import { afterEach, expect, test, vi } from "vitest";
import { requestGpuDevice } from "@packages/renderer-core/src/device";

afterEach(() => vi.unstubAllGlobals());

test("the device is requested with the adapter's maxTextureArrayLayers", async () => {
  const requested: GPUDeviceDescriptor[] = [];
  const device = { features: new Set(), limits: { maxTextureArrayLayers: 2048 } };
  const adapter = {
    features: new Set(),
    limits: { maxTextureArrayLayers: 2048, maxBufferSize: 1 << 30 },
    info: {},
    requestDevice: async (descriptor: GPUDeviceDescriptor) => {
      requested.push(descriptor);
      return device;
    },
  };
  vi.stubGlobal("navigator", {
    gpu: { requestAdapter: async () => adapter, getPreferredCanvasFormat: () => "bgra8unorm" },
  });
  const info = await requestGpuDevice();
  expect(requested[0].requiredLimits?.maxTextureArrayLayers).toBe(2048);
  expect(info.limits.maxTextureArrayLayers).toBe(2048);
});
