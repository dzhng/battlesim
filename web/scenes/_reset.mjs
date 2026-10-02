import { lab, restart, advance, presented, buildingsSettled } from "./_lab.mjs";
import { game } from "./_units.mjs";
const SETTLE_MS = Math.ceil(1000 / game.tick_hz) + 1;

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
  await restart(page);
  await page.waitForFunction((prior) => window.__lab?.route?.tick() < prior, priorTick, {
    timeout: 60000,
  });
  const firstStarted = Date.now();
  // Cleared refs can precede the new client closure. Only a disposed predecessor
  // is retried; the first actual step must acknowledge this battle's tick one.
  await presentedStep(page, async () => {
    for (;;) {
      if (Date.now() - firstStarted >= 60000)
        throw Error("new reset client did not acknowledge its first step");
      const stepped = await lab(page, async () => {
        const route = window.__lab.route;
        if (route.tick() !== 0) throw Error("reset moved before its first controlled step");
        route.pause();
        try {
          const tick = await route.advance(1);
          if (tick !== 1) throw Error(`first reset step acknowledged tick ${tick}`);
          return true;
        } catch (error) {
          if (error.message === "simulation client disposed") return false;
          throw error;
        }
      });
      if (stepped) break;
      await page.waitForTimeout(1);
    }
  });
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
    gpu: window.__lab.allocations(),
  }));
}

// Control only reset presentation. Real-time arms keep native clocks and RAF timestamps.
export function installResetRafFence() {
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
