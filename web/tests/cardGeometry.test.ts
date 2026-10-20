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
  expect(live).toBe(4);
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

test("baked surface depth restores ground clearance after nonuniform scale and heading", async () => {
  const { cardSurface } = await import("@packages/battle-renderer/src/models/cardGeometry");
  const toward = d.vec3f(Math.cos(0.85), 0, Math.sin(0.85));
  const scale = d.vec3f(2, 0.6, 1.3);
  const yaw = 0.7;
  const placement = d.vec3f(10, 20, 0);
  // Visible nose above the ground, whose projection onto the centre plane is below it.
  const surface = d.vec3f(2, 0, 0.2);
  const centre = d.vec3f(0, 0, 0.7);
  const offset = (surface.x - centre.x) * toward.x + (surface.z - centre.z) * toward.z;
  const plane = d.vec3f(surface.x - offset * toward.x, 0, surface.z - offset * toward.z);
  expect(plane.z).toBeLessThan(0);
  const radius = 2.5;
  const depth = 0.5 + offset / (2 * radius);
  for (const coverage of [1, 0.6]) {
    const bits = Math.round(depth * coverage * 65535);
    const packed = d.vec2f((bits & 255) / 255, (bits >>> 8) / 255);
    const result = cardSurface(
      cardPoint(plane, scale, yaw, placement),
      cardPoint(toward, scale, yaw, d.vec3f(0)),
      packed,
      coverage,
      radius,
    );
    const expected = cardPoint(surface, scale, yaw, placement);
    expect(result.x).toBeCloseTo(expected.x, 3);
    expect(result.y).toBeCloseTo(expected.y, 3);
    expect(result.z).toBeCloseTo(expected.z, 3);
  }
});

test("depth filtering keeps a covered surface beside empty atlas texels", async () => {
  const { filterCardDepth } = await import("@packages/battle-renderer/src/models/cardDepth");
  const albedo = new Uint8Array(16);
  albedo[3] = 255;
  albedo[7] = 255;
  const packed = filterCardDepth(new Float32Array([0.8, 0.8, 0, 0]), albedo, 2, 2, 2);
  const depth = (packed[0] + 256 * packed[1]) / 65535 / (128 / 255);
  expect(depth).toBeCloseTo(0.8, 4);
  expect(filterCardDepth(new Float32Array(4), new Uint8Array(16), 2, 2, 2)).toEqual(
    new Uint8Array(2),
  );
});
