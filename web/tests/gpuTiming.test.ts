// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { createFrameTimer } from "@packages/battle-renderer/src/frame/gpuTiming";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import { fakeGpuDevice } from "./support/gpuDevice";

afterEach(() => vi.unstubAllGlobals());

function timing() {
  vi.stubGlobal("GPUBufferUsage", { QUERY_RESOLVE: 512, COPY_SRC: 4, MAP_READ: 1, COPY_DST: 8 });
  vi.stubGlobal("GPUTextureUsage", { RENDER_ATTACHMENT: 16 });
  vi.stubGlobal("GPUMapMode", { READ: 1 });
  const { device } = fakeGpuDevice();
  const maps: { finish(ms: number): void; reject(): void }[] = [];
  Object.assign(device, {
    features: new Set(["timestamp-query"]),
    createQuerySet: () => ({ destroy() {} }),
    createBuffer: ({ size }: GPUBufferDescriptor) => {
      let mapped = false;
      let data = new BigUint64Array(2);
      return {
        size,
        get mapState() {
          return mapped ? "mapped" : "unmapped";
        },
        destroy() {},
        mapAsync() {
          if (mapped) throw new Error("Buffer is already mapping or mapped");
          mapped = true;
          return new Promise<void>((resolve, reject) => {
            maps.push({
              finish(ms) {
                data = new BigUint64Array([1n, 1n + BigInt(ms * 1e6)]);
                resolve();
              },
              reject() {
                mapped = false;
                reject(new Error("Map failed"));
              },
            });
          });
        },
        getMappedRange: () => data.buffer,
        unmap() {
          mapped = false;
        },
      };
    },
  });
  const encoder = {
    beginRenderPass: () => ({ end() {} }),
    resolveQuerySet() {},
    copyBufferToBuffer(_source: GPUBuffer, _offset: number, target: GPUBuffer) {
      // A mapped buffer cannot be a copy destination in WebGPU.
      expect(target.mapState).toBe("unmapped");
    },
  } as unknown as GPUCommandEncoder;
  const timer = createFrameTimer(device, new GpuRegistry(device))!;
  const frame = () => {
    timer.begin(encoder);
    timer.end(encoder);
    timer.collect();
  };
  // Drain the map's then/catch/finally chain without waiting on wall time.
  const settle = () => new Promise<void>((resolve) => setImmediate(resolve));
  return { timer, encoder, maps, frame, settle };
}

test("reset excludes delayed old readbacks while retaining new frames", async () => {
  const { timer, maps, frame, settle } = timing();
  frame();
  maps[0].finish(2);
  await settle();
  expect(timer.stats()).toEqual({ frames: 1, meanMs: 2, p95Ms: 2 });
  frame();
  timer.reset();
  expect(timer.stats()).toEqual({ frames: 0, meanMs: 0, p95Ms: 0 });
  frame();
  maps[2].finish(5);
  await settle();
  maps[1].finish(90);
  await settle();
  expect(timer.stats()).toEqual({ frames: 1, meanMs: 5, p95Ms: 5 });
});

test.each(["begin", "end"] as const)(
  "reset after %s excludes the partially queued frame",
  async (phase) => {
    const { timer, encoder, maps, frame, settle } = timing();
    timer.begin(encoder);
    if (phase === "begin") timer.reset();
    timer.end(encoder);
    if (phase === "end") timer.reset();
    timer.collect();
    maps[0].finish(90);
    await settle();
    expect(timer.stats()).toEqual({ frames: 0, meanMs: 0, p95Ms: 0 });
    frame();
    maps[1].finish(7);
    await settle();
    expect(timer.stats()).toEqual({ frames: 1, meanMs: 7, p95Ms: 7 });
  },
);

test("reset keeps busy slots reserved, and rejected or stale maps release them for reuse", async () => {
  const { timer, maps, frame, settle } = timing();
  frame();
  frame();
  frame();
  timer.reset();
  timer.reset();
  frame();
  expect(maps).toHaveLength(3);
  maps[0].reject();
  maps[1].finish(80);
  maps[2].finish(90);
  await settle();
  expect(timer.stats()).toEqual({ frames: 0, meanMs: 0, p95Ms: 0 });
  frame();
  frame();
  frame();
  expect(maps).toHaveLength(6);
  maps[3].finish(4);
  maps[4].finish(6);
  maps[5].finish(8);
  await settle();
  expect(timer.stats()).toEqual({ frames: 3, meanMs: 6, p95Ms: 6 });
});
