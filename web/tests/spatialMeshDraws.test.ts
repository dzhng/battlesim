// @vitest-environment node
import { expect, test } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { SpatialMeshDraws } from "@packages/battle-renderer/src/frame/spatialMeshDraws";
import { MeshSlot } from "@packages/battle-renderer/src/frame/geometry";
import { CAMERA_WORDS, CameraAt } from "@packages/battle-renderer/src/records.gen";
import { submittedDraws } from "@packages/battle-renderer/src/frame/submitted";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import { fakeGpuDevice } from "./support/gpuDevice";

await loadPresentation();

test("shared Wasm selection submits camera and independent cascade ranges with exact counters", () => {
  const { device } = fakeGpuDevice();
  const registry = new GpuRegistry(device);
  const uploads: Float32Array[] = [];
  const root = {
    createBuffer() {
      return {
        $usage() {
          return this;
        },
        write(data: ArrayBuffer, options: { endOffset: number }) {
          uploads.push(new Float32Array(data.slice(0, options.endOffset)));
        },
        destroy() {},
      };
    },
  };
  const draws = new SpatialMeshDraws(new MeshSlot(root as never, registry, {} as never), registry);
  // Two separated tiles, distinct attribute IDs, supplied in reverse spatial order.
  const triangle = (x: number, id: number) => [
    x,
    0,
    0.5,
    0,
    0,
    1,
    id,
    0,
    0,
    1,
    x + 0.25,
    0,
    0.5,
    0,
    0,
    1,
    id,
    0,
    0,
    1,
    x,
    0.25,
    0.5,
    0,
    0,
    1,
    id,
    0,
    0,
    1,
  ];
  draws.set(new Float32Array([...triangle(1024, 9), ...triangle(0, 7)]));
  const camera = (x: number) => {
    const words = new Float32Array(CAMERA_WORDS);
    words.set([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -x, 0, 0, 1], CameraAt.view_proj);
    return words;
  };
  const casters = new Float32Array(CAMERA_WORDS * 4);
  [1024, 0, 5000, 1024].forEach((x, i) => casters.set(camera(x), i * CAMERA_WORDS));
  draws.prepare(camera(0), casters, 4);
  const calls: number[][] = [];
  const bound = {
    with() {
      return this;
    },
    draw(...args: number[]) {
      calls.push(args);
    },
  };
  for (const [cascade, id] of [
    [undefined, 7],
    [0, 9],
    [1, 7],
    [2, undefined],
    [3, 9],
  ] as const) {
    calls.length = 0;
    const counter = submittedDraws();
    draws.draw(bound as never, counter, cascade);
    const ids = calls.flatMap(([count, , first]) =>
      Array.from({ length: count / 3 }, (_, i) => uploads[0][(first + i * 3) * 10 + 6]),
    );
    expect(ids).toEqual(id === undefined ? [] : [id]);
    expect(counter).toEqual({
      triangles: ids.length,
      draws: calls.length,
      instances: calls.length,
    });
  }
  expect(uploads).toHaveLength(1);
  expect(draws.vertices).toBe(6);
  expect(draws.visited).toBeGreaterThan(0);
  // Replacement must not leave stale draw ranges; exercise disposal afterward.
  draws.set(new Float32Array());
  draws.prepare(camera(0), casters, 4);
  calls.length = 0;
  draws.draw(bound as never, submittedDraws());
  expect(calls).toEqual([]);
  registry.release();
});
