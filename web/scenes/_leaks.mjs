// Live-resource accounting for a page, installed before any of its scripts
// run: GPU devices, workers, audio contexts and window/document listeners,
// each counted up on creation and down on its release. A scene reads the
// counts to prove repeated reset, side switches and remounts leave nothing
// behind. (GPU buffers and textures are the lab's own `allocations()`.)

/** Wrap the page's constructors and registration calls. Call before `goto`. */
export async function trackPageResources(page) {
  await page.addInitScript(() => {
    const live = { devices: 0, workers: 0, audioContexts: 0, listeners: 0 };
    window.__resources = () => ({ ...live });

    if (window.GPUAdapter) {
      const requestDevice = GPUAdapter.prototype.requestDevice;
      GPUAdapter.prototype.requestDevice = async function (...args) {
        const device = await requestDevice.apply(this, args);
        live.devices++;
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
