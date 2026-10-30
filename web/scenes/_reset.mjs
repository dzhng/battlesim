import { lab, restart, advance, presented, buildingsSettled } from "./_lab.mjs";
import { rules } from "./_units.mjs";
const SETTLE_MS = Math.ceil(1000 / rules.tick_hz) + 1;

async function presentedStep(page, action) {
  await lab(page, () => window.__resetRafFence.hold());
  let timer;
  try {
    await Promise.race([
      action(),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(Error("controlled reset step did not complete")), 60000);
      }),
    ]);
    await page.waitForTimeout(SETTLE_MS);
  } finally {
    clearTimeout(timer);
    await lab(page, () => window.__resetRafFence.release());
  }
  await presented(page);
}

export async function restartPresentedOpening(page, targetTick) {
  const priorTick = await lab(page, () => window.__lab.route.tick());
  // Pause at the real worker boundary before the new authority can tick.
  // Renderer admission can finish after its clock has already started.
  await page.evaluate(() => window.__resetRafFence.pauseNextStart());
  await restart(page);
  await page.waitForFunction((prior) => window.__lab?.route?.tick() < prior, priorTick, {
    timeout: 60000,
  });
  await presentedStep(page, () =>
    lab(page, async () => {
      const route = window.__lab.route;
      if (route.tick() !== 0) throw Error("reset moved before its first controlled step");
      route.pause();
      const tick = await route.advance(1);
      if (tick !== 1) throw Error(`first reset step acknowledged tick ${tick}`);
    }),
  );
  for (let tick = 1; tick <= targetTick; tick++) {
    if (tick > 1) await presentedStep(page, () => advance(page, 1));
    const state = await lab(page, () => ({
      tick: window.__lab.route.tick(),
      status: window.__lab.route.status().status,
    }));
    if (state.tick !== tick || state.status !== "paused")
      throw Error(`reset expected paused tick ${tick}, got ${JSON.stringify(state)}`);
  }
  await buildingsSettled(page);
  return lab(page, () => ({
    presentation: {
      tick: window.__lab.route.tick(),
      presented: window.__lab.route.presented(),
      digest: window.__lab.route.digest(),
      lying: window.__lab.route.lying(),
      camera: window.__lab.camera(),
    },
    // The buildings standing, and the module instances the frame drew of them.
    buildings: {
      standing: window.__lab.stats().buildings.buildings,
      drawn: window.__lab.stats().buildings.tiers.reduce((a, b) => a + b, 0),
    },
    gpu: window.__lab.allocations(),
  }));
}

// Control only reset presentation. Real-time arms keep native clocks and RAF timestamps.
export function installResetRafFence() {
  let pauseNextStart = false;
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...args) {
    if (pauseNextStart && message.type === "start") {
      pauseNextStart = false;
      post.call(this, { type: "pause" });
    }
    return post.call(this, message, ...args);
  };
  const request = window.requestAnimationFrame.bind(window);
  const cancel = window.cancelAnimationFrame.bind(window);
  const callbacks = new Map();
  let next = 0;
  let held = false;
  function schedule(id, entry) {
    entry.native = request((time) => {
      entry.native = null;
      if (held) return;
      callbacks.delete(id);
      entry.callback(entry.controlled ? Math.max(time, performance.now()) : time);
    });
  }
  window.requestAnimationFrame = (callback) => {
    const id = ++next;
    const entry = { callback, native: null, controlled: held };
    callbacks.set(id, entry);
    if (!held) schedule(id, entry);
    return id;
  };
  window.cancelAnimationFrame = (id) => {
    const entry = callbacks.get(id);
    if (entry?.native !== null && entry?.native !== undefined) cancel(entry.native);
    callbacks.delete(id);
  };
  window.__resetRafFence = {
    pauseNextStart() {
      pauseNextStart = true;
    },
    hold() {
      held = true;
      for (const entry of callbacks.values()) {
        if (entry.native !== null) cancel(entry.native);
        entry.native = null;
        entry.controlled = true;
      }
    },
    release() {
      held = false;
      for (const [id, entry] of callbacks) if (entry.native === null) schedule(id, entry);
    },
  };
}
