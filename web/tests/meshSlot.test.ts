// @vitest-environment node
// A replaceable mesh redrawn every frame (the overlays) keeps its GPU buffer
// while it fits, and uploads only the vertices it has.
import { expect, test } from "vitest";
import { MeshSlot } from "@packages/battle-renderer/src/frame/geometry.ts";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry.ts";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";

import { fakeGpuDevice } from "./support/gpuDevice";

function fakeRoot() {
  const made: { destroyed: boolean; writes: { endOffset?: number }[] }[] = [];
  const root = {
    createBuffer() {
      const buffer = {
        destroyed: false,
        writes: [] as { endOffset?: number }[],
        $usage() {
          return this;
        },
        write(_data: ArrayBuffer, options: { endOffset?: number } = {}) {
          buffer.writes.push(options);
        },
        destroy() {
          buffer.destroyed = true;
        },
      };
      made.push(buffer);
      return buffer;
    },
  };
  return { root, made };
}

const mesh = (vertices: number) => new Float32Array(vertices * VERTEX_FLOATS);

test("a mesh slot keeps its buffer while the mesh fits, uploading only the mesh", () => {
  const { root, made } = fakeRoot();
  const registry = new GpuRegistry(fakeGpuDevice().device);
  const slot = new MeshSlot(root as never, registry, {} as never);

  slot.set(mesh(30));
  slot.set(mesh(20));
  expect(made).toHaveLength(1);
  expect(made[0].writes.at(-1)).toEqual({ endOffset: 20 * VERTEX_FLOATS * 4 });
  expect(slot.vertices).toBe(20);

  slot.set(mesh(5000));
  expect(made).toHaveLength(2);
  expect(made[0].destroyed).toBe(true);
  expect(slot.vertices).toBe(5000);

  registry.release();
  expect(made[1].destroyed).toBe(true);
});
