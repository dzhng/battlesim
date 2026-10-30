// @vitest-environment node
import { runInNewContext } from "node:vm";
import { expect, test } from "vitest";
import { trackPageResources, pageResources } from "./_leaks.mjs";

/** Fake only the browser boundary. No real GPU or garbage-collection timing. */
async function pageWithGpu() {
  const window = new EventTarget();
  class GPUAdapter {
    async requestDevice() {
      return Object.assign(new EventTarget(), {
        queue: { onSubmittedWorkDone: async () => {} },
        destroy() {},
        createBuffer: () => ({ destroy() {} }),
        createTexture: () => ({ destroy() {} }),
      });
    }
  }
  window.GPUAdapter = GPUAdapter;
  window.Worker = class {
    terminate() {}
  };
  const scope = { window, document: new EventTarget(), GPUAdapter };
  const page = {
    addInitScript: async (script) => runInNewContext(`(${script})()`, scope),
    evaluate: async (script) => runInNewContext(`(${script})()`, scope),
  };
  await trackPageResources(page);
  return { page, adapter: new GPUAdapter(), gpu: window.__gpuResources };
}

test("one observer counts live GPU allocations across devices and before a departure mark", async () => {
  const { page, adapter, gpu } = await pageWithGpu();
  // Reinstalling observation must not wrap the device or allocation APIs twice.
  await trackPageResources(page);
  const first = await adapter.requestDevice();
  const buffer = first.createBuffer();
  const texture = first.createTexture();
  const mark = gpu.created();
  const second = await adapter.requestDevice();
  const next = second.createTexture();
  expect(gpu.devices).toBe(2);
  expect((await pageResources(page)).devices).toBe(2);
  expect(gpu.counts()).toEqual({ buffers: 1, textures: 2 });
  expect(gpu.counts(mark)).toEqual({ buffers: 1, textures: 1 });
  buffer.destroy();
  buffer.destroy();
  texture.destroy();
  expect(gpu.counts(mark)).toEqual({ buffers: 0, textures: 0 });
  expect(gpu.counts()).toEqual({ buffers: 0, textures: 1 });
  next.destroy();
  first.destroy();
  first.destroy();
  second.destroy();
  expect(gpu.counts()).toEqual({ buffers: 0, textures: 0 });
  expect((await pageResources(page)).devices).toBe(0);
});

test("the observer drains every device queue and retains GPU validation failures", async () => {
  const { adapter, gpu } = await pageWithGpu();
  const first = await adapter.requestDevice(),
    second = await adapter.requestDevice();
  let releaseFirst, releaseSecond;
  const firstDone = new Promise((r) => {
    releaseFirst = r;
  });
  const secondDone = new Promise((r) => {
    releaseSecond = r;
  });
  first.queue.onSubmittedWorkDone = () => firstDone;
  second.queue.onSubmittedWorkDone = () => secondDone;
  let drained = false;
  const done = gpu.drain().then(() => {
    drained = true;
  });
  releaseFirst();
  await new Promise(setImmediate);
  expect(drained).toBe(false);
  releaseSecond();
  await done;
  expect(drained).toBe(true);
  const event = new Event("uncapturederror");
  event.error = { message: "deliberate GPU failure" };
  second.dispatchEvent(event);
  expect(gpu.validation).toEqual(["deliberate GPU failure"]);
});
