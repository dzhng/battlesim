// Live-resource accounting for a page, installed before any of its scripts
// run: GPU devices, buffers and textures, workers, audio contexts and listeners,
// each counted up on creation and down on its release. A scene reads the
// counts to prove repeated reset, side switches and remounts leave nothing
// behind. GPU allocations have weak identities: wgpu may release a handle
// without calling destroy(), and the observer must not keep it alive.

/** Wrap the page's constructors and registration calls. Call before `goto`. */
export async function trackPageResources(page) {
  await page.addInitScript(() => {
    if (window.__resources) return;
    const live = { devices: 0, workers: 0, audioContexts: 0, listeners: 0 };
    window.__resources = () => ({ ...live });

    const resources = new Map();
    const queues = [];
    let created = 0;
    const gpu = {
      devices: 0,
      validation: [],
      created: () => created,
      counts(before = Infinity) {
        const counts = { buffers: 0, textures: 0 };
        for (const [order, { reference, kind }] of resources) {
          if (!reference.deref()) resources.delete(order);
          else if (order < before) counts[kind]++;
        }
        return counts;
      },
      drain: () => Promise.all(queues.map((queue) => queue.onSubmittedWorkDone())),
    };
    window.__gpuResources = gpu;
    const onDestroy = (order, destroy) =>
      function (...args) {
        resources.delete(order);
        return destroy.apply(this, args);
      };
    const observe = (resource, kind) => {
      const order = created++;
      resources.set(order, { reference: new WeakRef(resource), kind });
      resource.destroy = onDestroy(order, resource.destroy);
      return resource;
    };

    if (window.GPUAdapter) {
      const requestDevice = GPUAdapter.prototype.requestDevice;
      GPUAdapter.prototype.requestDevice = async function (...args) {
        const device = await requestDevice.apply(this, args);
        live.devices++;
        gpu.devices++;
        queues.push(device.queue);
        device.addEventListener("uncapturederror", (event) =>
          gpu.validation.push(event.error.message),
        );
        for (const [method, kind] of [
          ["createBuffer", "buffers"],
          ["createTexture", "textures"],
        ]) {
          const create = device[method];
          device[method] = function (...descriptors) {
            return observe(create.apply(this, descriptors), kind);
          };
        }
        const destroy = device.destroy.bind(device);
        let destroyed = false;
        device.destroy = () => {
          if (!destroyed) live.devices--;
          destroyed = true;
          destroy();
        };
        return device;
      };
    }

    const NativeWorker = window.Worker;
    window.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args);
        live.workers++;
        this.__live = true;
      }
      terminate() {
        if (this.__live) live.workers--;
        this.__live = false;
        super.terminate();
      }
    };

    const NativeAudio = window.AudioContext;
    if (NativeAudio)
      window.AudioContext = class extends NativeAudio {
        constructor(...args) {
          super(...args);
          live.audioContexts++;
          this.__live = true;
        }
        close() {
          if (this.__live) live.audioContexts--;
          this.__live = false;
          return super.close();
        }
      };

    // Listeners on the two long-lived targets: a component that registers
    // one on every mount and forgets it on unmount grows this count.
    for (const target of [window, document]) {
      const registered = new Set();
      const key = (type, fn, options) =>
        `${type}|${typeof options === "boolean" ? options : !!options?.capture}|${fnId(fn)}`;
      const add = target.addEventListener.bind(target);
      const remove = target.removeEventListener.bind(target);
      target.addEventListener = (type, fn, options) => {
        const k = key(type, fn, options);
        if (fn && !registered.has(k)) {
          registered.add(k);
          live.listeners++;
          if (options?.signal)
            options.signal.addEventListener("abort", () => {
              if (registered.delete(k)) live.listeners--;
            });
        }
        return add(type, fn, options);
      };
      target.removeEventListener = (type, fn, options) => {
        if (registered.delete(key(type, fn, options))) live.listeners--;
        return remove(type, fn, options);
      };
    }
    const ids = new WeakMap();
    let next = 0;
    function fnId(fn) {
      if (!fn || (typeof fn !== "function" && typeof fn !== "object")) return "none";
      if (!ids.has(fn)) ids.set(fn, ++next);
      return ids.get(fn);
    }
  });
}

/** The page's live resources now. */
export const pageResources = (page) => page.evaluate(() => window.__resources());
