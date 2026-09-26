// SPIKE 01 (throwaway, never merged): the village through the ported foundation.
// SPIKE_PHASE: "shots" (frames + checks), "perf" (frame cost), default both.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
import { lab, obs, advance } from "./_lab.mjs";
import { measure } from "./_frameCost.mjs";

const PHASE = process.env.SPIKE_PHASE ?? "all";
const SECONDS = Number(process.env.FRAME_COST_S ?? 10);
const CAMERAS = {
  strategic: { target: [640, 800, 0], distance: 2200, pitch: 0.7, yaw: -1.57 },
  default: null,
  warno: { target: [560, 820, 0], distance: 420, pitch: 0.85, yaw: -1.3 },
  ground: { target: [990, 790, 0], distance: 60, pitch: 0.25, yaw: -1.2 },
  // main's slice 09 CameraController: default framing and the zoom_min end.
  "main-default": { target: [170, 800, 0], distance: 65, pitch: 0.85, yaw: -1.57 },
  closest: { target: [990, 790, 0], distance: 25, pitch: 0.22, yaw: -1.2 },
  village: { target: [1000, 800, 0], distance: 160, pitch: 0.6, yaw: -2.0 },
  // Broken Arrow ground framing: eye ~5 m over a building street.
  street: { target: [1000, 800, 2], distance: 28, pitch: 0.12, yaw: -2.2 },
};

async function frameOf(ctx, page, file) {
  await lab(page, () => window.__lab.frame());
  await lab(page, () => window.__lab.frame());
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath(file), shot);
  return PNG.sync.read(shot);
}

async function stage(ctx, page, camera) {
  await lab(page, (c) => window.__lab.setCamera(c), camera);
}

/** Pause, give blue's tanks a move order (route-line overlays), advance. */
async function prepare(page) {
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 60000 });
  await lab(page, () => window.__lab.route.pause());
  const o = await obs(page);
  const tanks = o.own.filter((u) => u.kind === "tank").map((u) => u.id);
  await lab(page, (ids) => window.__lab.route.select(ids), tanks);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, tanks.length);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [360, 800, 0], distance: 600 }),
  );
  const spot = await lab(page, () => window.__lab.projectToCss(520, 800, 0));
  await page.mouse.click(spot[0], spot[1], { button: "right" });
  await page.waitForFunction(() => window.__lab.route.acks().length > 0, undefined, {
    timeout: 5000,
  });
  await advance(page, 60);
  return tanks;
}

function pixel(png, x, y) {
  const i = (Math.round(y) * png.width + Math.round(x)) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
}

export async function run(ctx) {
  const base = new URL(ctx.url).origin;
  const size = { width: 1920, height: 1080 };
  const spike = await ctx.newPage({ viewport: size });
  await ctx.openLab(spike);
  await lab(spike, () => window.__spike.ready());
  const opening = await lab(spike, () => window.__lab.camera());

  if (PHASE !== "perf") {
    await prepare(spike);
    const flat = await ctx.newPage({ viewport: size });
    await ctx.openLab(flat, `${base}/battle/village-perf`);
    await prepare(flat);
    const cams = { ...CAMERAS, orders: { target: [440, 800, 0], distance: 380, pitch: 0.9, yaw: -1.57 } };
    const overlayDiffs = [];
    // Pixel checks read the canvas alone: hide DOM panels and readouts.
    const canvasOnly = "* { visibility: hidden !important } canvas { visibility: visible !important }";
    await spike.addStyleTag({ content: canvasOnly });
    await flat.addStyleTag({ content: canvasOnly });
    for (const [name, cam] of Object.entries(cams)) {
      const c = { ...opening, ...(cam ?? {}) };
      await stage(ctx, spike, c);
      await stage(ctx, flat, c);
      const a = await frameOf(ctx, spike, `spike-${name}.png`);
      const b = await frameOf(ctx, flat, `flat-${name}.png`);
      {
        // Overlay coverage from the spike's overlay-only frame, eroded 2 px so
        // AA edges (which blend over a different world) are excluded. There,
        // the new frame must show the old frame's colour exactly.
        await lab(spike, () => (window.__spike.debug.overlayOnly = "black"));
        const m = await frameOf(ctx, spike, `spike-${name}-overlay-black.png`);
        await lab(spike, () => (window.__spike.debug.overlayOnly = "white"));
        const w = await frameOf(ctx, spike, `spike-${name}-overlay-white.png`);
        await lab(spike, () => (window.__spike.debug.overlayOnly = false));
        // Opaque overlay: the same colour over black and over white.
        const opaque = (x, y) => {
          const pm = pixel(m, x, y),
            pw = pixel(w, x, y);
          return pm.every((v, k) => Math.abs(v - pw[k]) <= 1);
        };
        let n = 0,
          same = 0,
          worst = 0;
        const hist = {};
        for (let y = 3; y < b.height - 3; y += 1)
          for (let x = 3; x < b.width - 3; x += 1) {
            let inside = true;
            for (let dy = -1; dy <= 1 && inside; dy++)
              for (let dx = -1; dx <= 1 && inside; dx++) inside = opaque(x + dx, y + dy);
            if (!inside) continue;
            const pa = pixel(a, x, y),
              pb = pixel(b, x, y),
              pm = pixel(m, x, y);
            const d = Math.max(...pa.map((v, k) => Math.abs(v - pm[k])));
            const dFlat = Math.max(...pb.map((v, k) => Math.abs(v - pm[k])));
            n++;
            if (d <= 1) same++;
            worst = Math.max(worst, d);
            const bucket = dFlat <= 1 ? "flatSame" : "flatDiff";
            hist[bucket] = (hist[bucket] ?? 0) + 1;
          }
        overlayDiffs.push({ name, overlayPixels: n, identical: same, worst, vsFlat: hist });
      }
    }
    console.log(`METRIC spike overlay isolation: ${JSON.stringify(overlayDiffs)}`);
    ctx.check(
      "overlay pixels keep their colour after post",
      overlayDiffs
        .filter((o) => o.overlayPixels > 50)
        .every((o) => o.identical / o.overlayPixels > 0.99),
      JSON.stringify(overlayDiffs),
    );
    const cascadesAt = {};
    for (const [name, cam] of Object.entries(CAMERAS)) {
      await stage(ctx, spike, { ...opening, ...(cam ?? {}) });
      await lab(spike, () => window.__lab.frame());
      cascadesAt[name] = await lab(spike, () => {
        const s = window.__spike.spikeStats();
        return { splitNear: s.splitNear, cappedFar: s.cappedFar, cascades: s.cascades };
      });
    }
    console.log(`METRIC spike cascades: ${JSON.stringify(cascadesAt)}`);

    // Shadow swimming: the world position of one building's shadow edge,
    // found along a ground line, while the camera pans, zooms and turns.
    const swim = {};
    {
      const boxes = await lab(spike, () => window.__spike.structureBoxes());
      const sun = await lab(spike, () => window.__spike.sunDirection());
      const box = boxes
        .map((b) => ({ b, d: Math.hypot((b[0] + b[3]) / 2 - 1000, (b[1] + b[4]) / 2 - 800) }))
        .sort((p, q) => p.d - q.d)[0].b;
      const c = [(box[0] + box[3]) / 2, (box[1] + box[4]) / 2];
      const h = box[5] - box[2];
      const len = Math.hypot(sun[0], sun[1]);
      const dir = [-sun[0] / len, -sun[1] / len];
      const reach = (h / Math.tan(Math.asin(sun[2]))) * 1.0;
      const r0 = Math.hypot(box[3] - box[0], box[4] - box[1]) / 2;
      const ts = Array.from({ length: 240 }, (_, i) => r0 * 0.3 + ((r0 + reach * 1.6) * i) / 239);
      const pts = ts.map((t) => [c[0] + dir[0] * t, c[1] + dir[1] * t, box[2] + 0.05]);
      const edge = async (cam) => {
        await stage(ctx, spike, cam);
        const png = await frameOf(ctx, spike, `swim-tmp.png`);
        const px = await lab(spike, (p) => p.map((q) => window.__lab.projectToCss(...q)), pts);
        const lum = px.map((q) => {
          const [r, g, b] = pixel(png, q[0], q[1]);
          return 0.2126 * r + 0.7152 * g + 0.0722 * b;
        });
        const lo = Math.min(...lum),
          hi = Math.max(...lum),
          mid = (lo + hi) / 2;
        for (let i = lum.length - 1; i > 0; i--)
          if (lum[i - 1] < mid && lum[i] >= mid)
            return ts[i - 1] + ((mid - lum[i - 1]) / (lum[i] - lum[i - 1])) * (ts[i] - ts[i - 1]);
        return NaN;
      };
      const baseCam = { ...opening, target: [c[0], c[1], box[2]], distance: 140, pitch: 0.7, yaw: -2.0 };
      const run = async (name, cams) => {
        const e = [];
        for (const cam of cams) e.push(await edge(cam));
        const cascade = await lab(spike, () => window.__spike.spikeStats().cascades[0]);
        swim[name] = {
          edges: e.map((v) => +v.toFixed(3)),
          spreadM: +(Math.max(...e) - Math.min(...e)).toFixed(3),
          cascade0TexelM: +cascade.texel.toFixed(3),
        };
      };
      await run(
        "pan",
        Array.from({ length: 10 }, (_, k) => ({
          ...baseCam,
          target: [c[0] + k * 0.137, c[1] + k * 0.071, box[2]],
        })),
      );
      await run(
        "zoom",
        Array.from({ length: 8 }, (_, k) => ({ ...baseCam, distance: 120 + k * 7 })),
      );
      await run(
        "turn",
        Array.from({ length: 8 }, (_, k) => ({ ...baseCam, yaw: -2.0 + k * 0.03 })),
      );
      // Broken Arrow ground framing: eye ~5 m up, 30 m off the shadow edge,
      // looking across the shadow line.
      const mid = r0 + reach;
      const tgt = [c[0] + dir[0] * mid, c[1] + dir[1] * mid, box[2]];
      const side = Math.atan2(dir[1], dir[0]) + Math.PI / 2;
      const streetCam = { ...opening, target: tgt, distance: 30, pitch: 0.17, yaw: side };
      await run(
        "street-pan",
        Array.from({ length: 8 }, (_, k) => ({
          ...streetCam,
          target: [tgt[0] + k * 0.093, tgt[1] + k * 0.041, tgt[2]],
        })),
      );
      await frameOf(ctx, spike, "swim-street.png");
      console.log(`METRIC spike shadow swim: ${JSON.stringify(swim)}`);
    }

    // Allocation baseline across resize and rebuild.
    await stage(ctx, spike, opening);
    await lab(spike, () => window.__lab.frame());
    const before = await lab(spike, () => window.__lab.allocations());
    await spike.setViewportSize({ width: 1280, height: 720 });
    await lab(spike, () => window.__lab.frame());
    await lab(spike, () => window.__lab.frame());
    const small = await lab(spike, () => window.__lab.allocations());
    await spike.setViewportSize(size);
    await lab(spike, () => window.__lab.frame());
    await lab(spike, () => window.__lab.frame());
    const resized = await lab(spike, () => window.__lab.allocations());
    await lab(spike, () => window.__lab.rebuild());
    await lab(spike, () => window.__spike.ready());
    await lab(spike, () => window.__lab.frame());
    await lab(spike, () => window.__lab.frame());
    const rebuilt = await lab(spike, () => window.__lab.allocations());
    console.log(`METRIC spike allocations: ${JSON.stringify({ before, small, resized, rebuilt })}`);
    const same = (x, y) =>
      x.buffers === y.buffers && x.textures === y.textures && x.bufferBytes === y.bufferBytes;
    ctx.check("allocations return to baseline after resize", same(before, resized));
    ctx.check("allocations return to baseline after rebuild", same(before, rebuilt));
    await ctx.writeEvidence("checks.json", { overlayDiffs, cascadesAt, swim, before, small, resized, rebuilt });
    await flat.close();
  }

  if (PHASE !== "shots") {
    // Frame cost: live battle (unpaused), redraw forced every frame.
    const page = PHASE === "perf" ? spike : await ctx.newPage({ viewport: size });
    if (page !== spike) {
      await ctx.openLab(page);
      await lab(page, () => window.__spike.ready());
    }
    await page.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, { timeout: 60000 });
    await page.waitForTimeout(5000);
    const flat = await ctx.newPage({ viewport: size });
    await ctx.openLab(flat, `${base}/battle/village-perf`);
    await flat.waitForFunction(() => window.__lab.route?.tick() > 30, undefined, { timeout: 60000 });
    const rows = {};
    for (const name of ["strategic", "default", "main-default", "closest"]) {
      for (const [label, p] of [
        ["spike", page],
        ["flat", flat],
      ]) {
        // Only the page being measured runs; the other is paused.
        const other = p === page ? flat : page;
        await lab(other, () => window.__lab.route.pause());
        await lab(p, () => window.__lab.route.resume?.());
        await lab(p, (c) => window.__lab.setCamera(c), { ...opening, ...(CAMERAS[name] ?? {}) });
        await p.waitForTimeout(500);
        if (label === "spike") await lab(p, () => window.__spike.resetGpuTiming());
        await lab(p, () => (window.__renderCpu = { sum: 0, n: 0 }));
        const r = await measure(p, SECONDS);
        r.cpuRenderMs = await lab(p, () => window.__renderCpu.sum / window.__renderCpu.n);
        r.textureBytes = await lab(p, () => window.__textureBytes());
        if (label === "spike") r.gpu = await lab(p, () => window.__spike.spikeStats().gpuMs);
        rows[`${label}-${name}`] = r;
        console.log(
          `METRIC ${label} ${name}: p50 ${r.p50.toFixed(2)} ms, p95 ${r.p95.toFixed(2)}, p99 ${r.p99.toFixed(2)}, max ${r.max.toFixed(1)}, cpu render ${r.cpuRenderMs.toFixed(3)} ms, gpu ${JSON.stringify(r.gpu ?? null, (k, v) => (typeof v === "number" ? +v.toFixed(3) : v))}, buffers ${(r.bufferBytes / 2 ** 20).toFixed(1)} MiB in ${r.buffers}, ${r.textures} textures ${(r.textureBytes / 2 ** 20).toFixed(1)} MiB, sim ${r.simHz.toFixed(1)} Hz`,
        );
      }
    }
    await ctx.writeEvidence("frame-cost.json", { browser: ctx.browser, seconds: SECONDS, rows });
  }
}
