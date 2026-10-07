// Sight-light fog over the street test map. The GPU
// lookup against its oracle vectors (the sight shape from Rust, the lookup
// from its CPU mirror), agreement with the simulation's 8 m sweep, a sharp
// sight-shadow edge at ground framing, a turned turret, a garrison's eyes,
// and the frames the visual verdict reads. FOG_COST=1 also measures fog's GPU
// cost at 100 a side (run it alone, under the GPU lock).
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
import { decode } from "./_png.mjs";
import { advance, lab, obs, snapshot, until } from "./_lab.mjs";

/** Spike 02's bar: at most 5% of 8 m cells outside the one-cell band disagree. */
const AGREEMENT_BAR_PERCENT = 5;
/** Spike 02's edge bar at ground framing: no stair-steps past a pixel, and the
 *  edge within two pixels of the line through the eye and the corner. */
const STAIR_BAR_PX = 1;
const POSITION_BAR_PX = 2;
/** Spike 02's measured typical fog cost at 100 a side. */
const COST_BAR_MS = 1.6;

/** Building B's south-east corner: its sight shadow from the street recon. */
const CORNER = [1064, 800];
/** Broken Arrow ground framing (spike 02): about 11 m up, 20° down, 40° lens. */
const GROUND = { target: [1150, 824], distance: 36.06, pitch: 0.3393, yaw: Math.PI / 2 };
const NEAR = { target: [1078, 806], distance: 33.8, pitch: 0.3, yaw: 1.1903 };
const LENS = 0.698;

/** The agreement map, north up: red where the GPU sees and the simulation
 *  does not, blue the reverse, grey where both see. */
async function writeDiffMap(ctx, name, a, observation) {
  const { nx, ny, bits } = observation.fog;
  const png = new PNG({ width: nx, height: ny });
  const wrong = new Set(a.diff);
  for (let j = 0; j < ny; j++) {
    for (let i = 0; i < nx; i++) {
      const k = j * nx + i;
      const seen = (bits[k >> 5] >>> (k & 31)) & 1;
      const c = wrong.has(k)
        ? seen
          ? [60, 90, 255]
          : [255, 60, 60]
        : seen
          ? [150, 150, 150]
          : [30, 30, 30];
      png.data.set([...c, 255], ((ny - 1 - j) * nx + i) * 4);
    }
  }
  await writeFile(ctx.evidencePath(name), PNG.sync.write(png));
}

async function checkAgreement(ctx, page, label, file) {
  const o = await obs(page);
  const a = await lab(page, () => window.__lab.route.agreement());
  await writeDiffMap(ctx, file, a, o);
  const { diff: _diff, ...summary } = a;
  ctx.check(
    `${label}: fog agrees with the simulation's sweep within ${AGREEMENT_BAR_PERCENT}% outside the band`,
    a.outside > 1000 && a.simSeen > 100 && a.percent <= AGREEMENT_BAR_PERCENT,
    JSON.stringify(summary),
  );
  return a;
}

const setCamera = (page, c) =>
  lab(
    page,
    ({ c, lens }) => {
      const z = window.__lab.route.surfaceZ(c.target[0], c.target[1]);
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [c.target[0], c.target[1], z],
        distance: c.distance,
        pitch: c.pitch,
        yaw: c.yaw,
        fovY: lens,
      });
    },
    { c, lens: LENS },
  );

/** Per column, where the mask turns from seen (above) to unseen (below)
 *  inside the band; the largest deviation from a straight chord over a 64 px
 *  window (stair-stepping); and the distance from the analytic edge. */
function edgeMetric(png, band, line) {
  const pts = [];
  for (let x = band.x0; x < band.x1; x++) {
    let prev = null;
    for (let y = band.y0; y < band.y1; y++) {
      const v = png.data[(y * png.width + x) * 4] > 127;
      if (prev === true && !v) {
        pts.push([x, y - 0.5]);
        break;
      }
      prev = v;
    }
  }
  let stair = 0;
  for (let i = 32; i + 32 < pts.length; i++) {
    const [xa, ya] = pts[i - 32];
    const [xb, yb] = pts[i + 32];
    const [x, y] = pts[i];
    stair = Math.max(stair, Math.abs(y - (ya + ((yb - ya) * (x - xa)) / (xb - xa))));
  }
  const yAt = (x) => {
    let k = line.findIndex((p) => p[0] >= x);
    if (k <= 0) return null;
    const [x0, y0] = line[k - 1];
    const [x1, y1] = line[k];
    return y0 + ((y1 - y0) * (x - x0)) / (x1 - x0 || 1);
  };
  const errors = pts.map(([x, y]) => {
    const at = yAt(x);
    return at === null ? null : y - at;
  });
  const known = errors.filter((e) => e !== null).map(Math.abs);
  return {
    points: pts.length,
    stairPx: stair,
    positionMaxPx: known.length ? Math.max(...known) : null,
    positionMeanPx: known.length ? known.reduce((s, e) => s + e, 0) / known.length : null,
  };
}

/** The sight-shadow edge past building B's corner, one eye, one framing. */
async function checkEdge(ctx, page, name, framing, band, recon) {
  await setCamera(page, framing);
  await lab(page, () => window.__lab.route.showMask(true));
  const mask = decode(await snapshot(ctx, page, `edge-${name}-mask-1920x1080.png`));
  await lab(page, () => window.__lab.route.showMask(false));
  await snapshot(ctx, page, `edge-${name}-1920x1080.png`);
  // The analytic edge: the ray from the eye through the corner, on the ground.
  const line = await lab(
    page,
    ({ eye, corner }) => {
      const out = [];
      for (let t = 1; t < 12; t += 0.002) {
        const x = eye[0] + (corner[0] - eye[0]) * t;
        const y = eye[1] + (corner[1] - eye[1]) * t;
        const p = window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y));
        if (p && p[0] >= 0 && p[0] < 1920 && p[1] >= 0 && p[1] < 1080) out.push(p);
      }
      return out.sort((a, b) => a[0] - b[0]);
    },
    { eye: recon.position, corner: CORNER },
  );
  const e = edgeMetric(mask, band, line);
  ctx.check(
    `${name} ground framing: the corner's sight-shadow edge is sharp and in place`,
    e.points > 200 &&
      e.stairPx <= STAIR_BAR_PX &&
      e.positionMaxPx !== null &&
      e.positionMaxPx <= POSITION_BAR_PX,
    JSON.stringify(e),
  );
  return e;
}

/** Fog's GPU time at 100 a side: the endurance battle running, frames drawn
 *  with and without fog in interleaved batches (paired, spike 02's landmine
 *  8), then with every eye's maps rebuilt each frame (the worst case). */
async function measureCost(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page, new URL("/lab/endurance", ctx.url).href);
  // React's development build records a measure per render; a battle running
  // in real time for minutes would exhaust that buffer.
  await page.evaluate(() => setInterval(() => performance.clearMeasures(), 500));
  await page.waitForFunction(() => window.__lab.route?.tick() > 900, undefined, {
    timeout: 300000,
  });
  const framings = {
    ground: { target: [1500, 1000], distance: 25, pitch: 0.22, yaw: -1.57 },
    default: { target: [1500, 1000], distance: 65, pitch: 0.85, yaw: -1.57 },
    strategic: { target: [1500, 1000], distance: 2400, pitch: 0.95, yaw: -1.57 },
  };
  const batch = (mode) =>
    lab(
      page,
      async (mode) => {
        await window.__lab.suppressFog(mode === "off");
        await window.__lab.setFrameView("final");
        let loop = 0;
        const rebuild = () => {
          window.__lab.fog().rebuildAll();
          loop = requestAnimationFrame(rebuild);
        };
        if (mode === "rebuild") rebuild();
        await new Promise((r) => setTimeout(r, 1500));
        cancelAnimationFrame(loop);
        return window.__lab.stats().gpu;
      },
      mode,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const result = { eyes: 0, framings: {} };
  for (const [name, f] of Object.entries(framings)) {
    await lab(
      page,
      (f) => {
        const z = window.__lab.route.surfaceZ?.(f.target[0], f.target[1]) ?? 0;
        window.__lab.setCamera({ ...window.__lab.camera(), ...f, target: [...f.target, z] });
      },
      f,
    );
    const rows = { on: [], off: [], rebuild: [] };
    for (let r = 0; r < 6; r++) {
      for (const mode of ["on", "off", "rebuild"]) rows[mode].push((await batch(mode)).meanMs);
    }
    const off = median(rows.off);
    result.framings[name] = {
      offMs: off,
      fogMs: median(rows.on.map((v, i) => v - rows.off[i])),
      rebuildAllMs: median(rows.rebuild.map((v, i) => v - rows.off[i])),
      samples: rows,
    };
  }
  await lab(page, () => window.__lab.suppressFog(false));
  result.eyes = await lab(page, () => window.__lab.stats().fog.eyes);
  result.adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("fog-cost.json", result);
  const typical = Math.max(...Object.values(result.framings).map((f) => f.fogMs));
  ctx.check(
    `fog costs at most ${COST_BAR_MS} ms a frame at 100 a side (the worst case recorded)`,
    result.eyes > 50 && typical <= COST_BAR_MS,
    JSON.stringify({
      eyes: result.eyes,
      ...Object.fromEntries(
        Object.entries(result.framings).map(([k, v]) => [
          k,
          { fogMs: +v.fogMs.toFixed(3), rebuildAllMs: +v.rebuildAllMs.toFixed(3) },
        ]),
      ),
    }),
  );
  await page.close();
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 60000 });
  // The frames are the verdict's evidence: the panel stays out of them.
  await page.addStyleTag({ content: "[data-testid=fog-panel] { display: none; }" });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 2);
  await page.evaluate(() => window.__lab.frame());

  // The lookup's two oracles: Rust's sight shape and the CPU mirror.
  const shape = await lab(page, () => window.__lab.route.shapeOracle());
  ctx.check(
    "the GPU sight shape matches sim::sight::multiplier's oracle vectors",
    shape.vectors > 100 && shape.maxError < 1e-5,
    JSON.stringify(shape),
  );
  const oracle = await lab(page, () => window.__lab.route.lookupOracle(14));
  ctx.check(
    "the GPU lookup matches its CPU mirror on synthetic maps",
    oracle.compared > 0.9 * oracle.vectors &&
      oracle.mismatches === 0 &&
      oracle.seen > 0.1 * oracle.compared &&
      oracle.seen < 0.9 * oracle.compared,
    JSON.stringify(oracle),
  );

  await checkAgreement(ctx, page, "blue in the street", "agreement-street.png");
  const tick = (await obs(page)).tick;
  await snapshot(ctx, page, "street-oblique-1920x1080.png");
  await lab(page, () => window.__lab.route.showMask(true));
  await snapshot(ctx, page, "street-oblique-mask-1920x1080.png");
  await lab(page, () => window.__lab.route.showMask(false));
  const opening = await lab(page, () => window.__lab.camera());

  // One eye, the street recon, at Broken Arrow ground framing.
  const input = await lab(page, () => window.__lab.route.fogInput());
  const recon = input.sight.eyes.find((e) => e.key === "0:0");
  await lab(page, (e) => window.__lab.route.setEyes(e), [recon]);
  await checkEdge(ctx, page, "far", GROUND, { x0: 0, x1: 1920, y0: 300, y1: 800 }, recon);
  await checkEdge(ctx, page, "near", NEAR, { x0: 300, x1: 1300, y0: 360, y1: 1080 }, recon);
  await lab(page, (c) => window.__lab.setCamera(c), opening);
  // The recon's sight alone over the street: ARMAPHRACT's lit wedge between
  // buildings (the visual variable's reference crop).
  await snapshot(ctx, page, "street-recon-oblique-1920x1080.png");
  await lab(page, () => window.__lab.route.showMask(true));
  await snapshot(ctx, page, "street-recon-oblique-mask-1920x1080.png");
  await lab(page, () => window.__lab.route.showMask(false));

  // A tank's turret turns: its lobe swings round the eye, and no map rebuilds.
  const tank = input.sight.eyes.find((e) => e.shape.rear < e.shape.front);
  const ring = await lab(
    page,
    (tank) =>
      Array.from({ length: 24 }, (_, k) => {
        const a = (k / 24) * 2 * Math.PI;
        const x = tank.position[0] + Math.cos(a) * tank.range * 0.85;
        const y = tank.position[1] + Math.sin(a) * tank.range * 0.85;
        return { position: [x, y, window.__lab.route.surfaceZ(x, y)] };
      }),
    tank,
  );
  const turned = async (forward) => {
    await lab(page, (e) => window.__lab.route.setEyes(e), [{ ...tank, forward }]);
    await page.evaluate(() => window.__lab.frame());
    return [...(await lab(page, (r) => window.__lab.route.probe(r), ring))];
  };
  const builds = () => lab(page, () => window.__lab.stats().fog.rebuiltTotal);
  const before = await turned(tank.forward);
  const rebuiltBefore = await builds();
  const after = await turned(tank.forward + Math.PI);
  const rebuiltAfter = await builds();
  const lost = before.filter((s, k) => s && !after[k]).length;
  const gained = after.filter((s, k) => s && !before[k]).length;
  ctx.check(
    "a tank's turret turns its lobe without rebuilding its map",
    lost > 0 && gained > 0 && rebuiltAfter === rebuiltBefore,
    JSON.stringify({ before: before.join(""), after: after.join(""), rebuiltBefore, rebuiltAfter }),
  );
  await lab(page, () => window.__lab.route.setEyes(null));

  // Red's rifle squads garrison the street's buildings: one eye per facade
  // they hold, each seeing out of its own facade.
  await lab(page, () => window.__lab.route.setSide("red"));
  const garrisoned = await until(
    page,
    (f) => f.own.some((u) => u.garrison?.phase === "inside" && u.sight.eyes.length > 1),
    1800,
    30,
  );
  ctx.check(
    "red's squads garrison with one eye per facade they hold",
    !!garrisoned,
    JSON.stringify(garrisoned?.own.map((u) => [u.kind, u.garrison?.phase, u.sight.eyes.length])),
  );
  if (garrisoned) {
    await page.evaluate(() => window.__lab.frame());
    const squad = garrisoned.own.find((u) => u.garrison?.phase === "inside");
    const eyes = await lab(
      page,
      (id) => window.__lab.route.fogInput().sight.eyes.filter((e) => e.key.startsWith(`${id}:`)),
      squad.id,
    );
    const drawnEyes = await lab(page, () => window.__lab.stats().fog.eyes);
    const published = garrisoned.own.reduce((n, u) => n + u.sight.eyes.length, 0);
    // Ground 25 m out from the building through each facade eye.
    const centre = eyes.reduce(
      (c, e) => [c[0] + e.position[0] / eyes.length, c[1] + e.position[1] / eyes.length],
      [0, 0],
    );
    const outward = await lab(
      page,
      ({ eyes, centre }) =>
        eyes.map((e) => {
          const dx = e.position[0] - centre[0];
          const dy = e.position[1] - centre[1];
          const l = Math.hypot(dx, dy) || 1;
          const x = e.position[0] + (dx / l) * 25;
          const y = e.position[1] + (dy / l) * 25;
          return { position: [x, y, window.__lab.route.surfaceZ(x, y)] };
        }),
      { eyes, centre },
    );
    await lab(page, (e) => window.__lab.route.setEyes(e), eyes);
    await page.evaluate(() => window.__lab.frame());
    const fromSlots = [...(await lab(page, (p) => window.__lab.route.probe(p), outward))];
    await lab(page, () => window.__lab.route.setEyes(null));
    await page.evaluate(() => window.__lab.frame());
    const seenSlots = fromSlots.filter(Boolean).length;
    ctx.check(
      "a garrison's facade eyes are drawn and see out of their own facades",
      eyes.length === squad.sight.eyes.length &&
        eyes.length <= 4 &&
        drawnEyes === published &&
        seenSlots >= 0.8 * eyes.length,
      JSON.stringify({
        facades: eyes.length,
        drawnEyes,
        published,
        fromSlots: fromSlots.join(""),
      }),
    );
    await checkAgreement(ctx, page, "red garrisoned", "agreement-red-garrison.png");
    await snapshot(ctx, page, "red-garrison-1920x1080.png");
    await lab(page, () => window.__lab.route.showMask(true));
    await snapshot(ctx, page, "red-garrison-mask-1920x1080.png");
    await lab(page, () => window.__lab.route.showMask(false));
  }
  await lab(page, () => window.__lab.route.setSide("blue"));

  const stats = await lab(page, () => window.__lab.stats().fog);
  const tiles = await lab(page, async () => {
    const counts = [...(await window.__lab.fog().tileCounts())];
    return { max: Math.max(...counts), nonEmpty: counts.filter((c) => c > 0).length };
  });
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    dpr: 1,
    tick,
    fog: stats,
    tiles,
  });
  await page.close();
  if (process.env.FOG_COST === "1") await measureCost(ctx);
}
