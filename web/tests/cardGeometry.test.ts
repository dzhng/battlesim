// @vitest-environment node
import { expect, test } from "vitest";
import { d } from "typegpu";
import {
  cardPoint,
  cardDirection,
  cardNormal,
} from "@packages/battle-renderer/src/models/cardGeometry";

test("card geometry scales in model space before heading and placement", () => {
  const scale = d.vec3f(2, 3, 0.5);
  const yaw = Math.PI / 2;
  const point = cardPoint(d.vec3f(1, 2, 4), scale, yaw, d.vec3f(10, 20, 30));
  expect(point.x).toBeCloseTo(4);
  expect(point.y).toBeCloseTo(22);
  expect(point.z).toBeCloseTo(32);
  const eye = cardDirection(d.vec3f(-6, 2, 2), scale, yaw);
  expect(eye.x).toBeCloseTo(1);
  expect(eye.y).toBeCloseTo(2);
  expect(eye.z).toBeCloseTo(4);
  const normal = cardNormal(d.vec3f(1, 1, 0), scale, yaw);
  expect(normal.x).toBeCloseTo(-2 / Math.sqrt(13));
  expect(normal.y).toBeCloseTo(3 / Math.sqrt(13));
  expect(normal.z).toBe(0);
});

test("atlas upload failure releases partial GPU resources and retry disposes to baseline", async () => {
  const { uploadCardAtlases } = await import("@packages/battle-renderer/src/models/impostorCards");
  const { GpuRegistry } = await import("@packages/battle-renderer/src/frame/registry");
  let live = 0;
  let fail = true;
  const resource = () => {
    live++;
    let destroyed = false;
    return {
      destroy() {
        if (!destroyed) live--;
        destroyed = true;
      },
      $usage() {
        return this;
      },
      createView() {
        return {};
      },
      write() {},
    };
  };
  const device = { createTexture() {}, createBuffer() {}, queue: {} } as unknown as GPUDevice;
  const root = {
    device,
    createTexture() {
      if (fail && live === 1) throw new Error("allocation failed");
      return resource();
    },
    createBuffer: resource,
    createSampler() {},
    createBindGroup() {
      return {};
    },
  } as unknown as Parameters<typeof uploadCardAtlases>[0];
  const registry = new GpuRegistry(device);
  expect(() => uploadCardAtlases(root, registry, [])).toThrow("allocation failed");
  expect(live).toBe(0);
  fail = false;
  uploadCardAtlases(root, registry, []);
  expect(live).toBe(3);
  registry.release();
  expect(live).toBe(0);
  registry.release();
  expect(live).toBe(0);
});

test("scaled card vertex resolves to WGSL without a GPU", async () => {
  const { tgpu } = await import("typegpu");
  const { cardVertex } = await import("@packages/battle-renderer/src/models/impostorCards");
  expect(() => tgpu.resolve([cardVertex])).not.toThrow();
});
