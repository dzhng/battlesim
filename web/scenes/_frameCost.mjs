// Slice 00's frame-cost probe, until the benchmark's recorder replaces it
// (battle-look slice 10). Measures real-time frame intervals with a redraw
// forced every frame, so the numbers are render cost, not an idle vsync.
import { lab } from "./_lab.mjs";

/** Measure `seconds` of real-time frames at the current camera. */
export async function measure(page, seconds) {
  const tick0 = await lab(page, () => window.__lab.route.tick());
  await page.evaluate((ms) => {
    window.__frames = [];
    let last = performance.now();
    const end = last + ms;
    const loop = (now) => {
      window.__frames.push(now - last);
      last = now;
      // Re-setting the camera marks the viewport dirty: draw every frame.
      window.__lab.setCamera(window.__lab.camera());
      if (now < end) requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
  }, seconds * 1000);
  await page.waitForTimeout(seconds * 1000 + 200);
  return page.evaluate(
    ({ tick0, seconds }) => {
      const f = window.__frames.slice(1).sort((a, b) => a - b);
      const q = (p) => f[Math.min(f.length - 1, Math.round((f.length - 1) * p))];
      const gpu = window.__lab.allocations();
      // The battle frame's GPU time: timestamp-query over its last frames.
      const frame = window.__lab.stats().gpu;
      return {
        gpuFrameMs: frame?.meanMs ?? null,
        gpuFrameP95Ms: frame?.p95Ms ?? null,
        textureBytes: gpu.textureBytes,
        frames: f.length,
        p50: q(0.5),
        p95: q(0.95),
        p99: q(0.99),
        max: f[f.length - 1],
        simHz: (window.__lab.route.tick() - tick0) / seconds,
        publicationBytes: window.__lab.route.publicationBytes(),
        buffers: gpu.buffers,
        bufferBytes: gpu.bufferBytes,
        textures: gpu.textures,
      };
    },
    { tick0, seconds },
  );
}

/** Whether this browser's adapter offers GPU timestamp queries. */
export const timestampQuery = (page) =>
  page.evaluate(
    async () => (await navigator.gpu.requestAdapter())?.features.has("timestamp-query") ?? false,
  );
