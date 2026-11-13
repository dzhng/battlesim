// Smoke screens on the smoke lab: the encounter's scripted pop raises a cloud
// round blue's road tank. Red, watching from across open ground, has its fog
// close over the ground behind the cloud once it blooms and open again once it
// has faded, while the ground on its near side stays lit; red keeps a
// last-contact marker where the tank vanished, and sees the cloud itself.
// Blue's own cloud is always its own to see. Every number is read from the
// encounter and the published cloud. SMOKE_COST=1 also measures fog's GPU
// cost at a hundred eyes, with and without twelve clouds (run it alone, under
// the GPU lock).
import { readFile } from "node:fs/promises";
import { advance, aim, lab, obs, openBattle, snapshot } from "./_lab.mjs";

const fixture = async (path) =>
  JSON.parse(await readFile(new URL(`../../fixtures/${path}`, import.meta.url), "utf8"));

/** The encounter's scripted pop: its tick and its carrier. */
async function scriptedPop() {
  const encounter = await fixture("maps/smoke/encounters/smoke.json");
  const pop = encounter.scripts.find((s) => s.order.kind === "pop_smoke");
  return { tick: pop.tick, carrier: pop.order.units[0] };
}

/** The cloud close up, from red's side of it, then the lab's own framing. */
async function closeShot(ctx, page, centre, file) {
  const opening = await lab(page, () => window.__lab.camera());
  await aim(page, centre, { distance: 90, pitch: 0.85 });
  await snapshot(ctx, page, file);
  await lab(page, (c) => window.__lab.setCamera(c), opening);
}

/** Whether the GPU fog sees each ground point, for the side now viewed. */
const seen = (page, points) =>
  lab(
    page,
    async (points) => {
      const z = (p) => window.__lab.route.surfaceZ(p[0], p[1]);
      await window.__lab.frame();
      const flags = await window.__lab
        .fog()
        .probe(points.map((p) => ({ position: [p[0], p[1], z(p)] })));
      return Array.from(flags, (f) => f !== 0);
    },
    points,
  );

const tickNow = (page) => lab(page, () => window.__lab.route.tick());
const advanceTo = async (page, tick) => advance(page, tick - (await tickNow(page)));
const setSide = async (page, side) => {
  await lab(page, (s) => window.__lab.route.setSide(s), side);
  // The switch loads the side's presentation again; one tick publishes it.
  await advance(page, 1);
  await page.waitForFunction(
    (s) => window.__lab.route.side() === s && window.__lab.route.observation()?.own.length > 0,
    side,
  );
};

export async function run(ctx) {
  const pop = await scriptedPop();
  const hz = (await fixture("rules.json")).tick_hz;
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });

  // Before the pop, red sees the carrier and the ground round it.
  await setSide(page, "red");
  await advanceTo(page, pop.tick - 5);
  const redBefore = await obs(page);
  const watcher = redBefore.own.find((u) => u.kind === "test_tank");
  ctx.check(
    "red sees a smoke tank before the pop",
    !!watcher && redBefore.identified.some((e) => e.kind === "test_smoke_tank"),
    JSON.stringify({ watcher: watcher?.position, identified: redBefore.identified.length }),
  );
  await snapshot(ctx, page, "red-before-pop.png");

  // Blue's view at the pop: the cloud, its centre and its law.
  await setSide(page, "blue");
  await advanceTo(page, pop.tick + 1);
  const blue = await obs(page);
  const cloud = blue.smokeClouds[0];
  ctx.check(
    "the carrier's cloud stands where it popped, in its own side's view",
    blue.smokeClouds.length === 1 &&
      Math.hypot(
        cloud.centre[0] - blue.own.find((u) => u.id === pop.carrier).position[0],
        cloud.centre[1] - blue.own.find((u) => u.id === pop.carrier).position[1],
      ) < 1,
    JSON.stringify(blue.smokeClouds),
  );
  const bloomed = cloud.popped + Math.ceil(cloud.spec.bloomS * hz) + 2;
  const gone = cloud.popped + Math.ceil(cloud.spec.lifeS * hz) + 2;
  // The ground behind the cloud along red's line, and in front of it.
  const r = cloud.spec.radiusM;
  const [cx, cy] = cloud.centre;
  const [wx, wy] = watcher.position;
  const d = Math.hypot(cx - wx, cy - wy);
  const [ux, uy] = [(cx - wx) / d, (cy - wy) / d];
  const behind = [cx + ux * (r + 10), cy + uy * (r + 10)];
  const before = [cx - ux * (r + 10), cy - uy * (r + 10)];
  await snapshot(ctx, page, "blue-popped.png");
  await setSide(page, "red");
  await closeShot(ctx, page, cloud.centre, "red-blooming-close.png");
  const [behindBefore, nearBefore] = await seen(page, [behind, before]);
  ctx.check(
    "before it blooms, red sees the ground on both sides of the cloud",
    behindBefore && nearBefore,
    JSON.stringify({ behind, before, behindBefore, nearBefore }),
  );

  // Bloomed: red's fog closes behind it, not before it.
  await advanceTo(page, bloomed);
  const [behindSmoked, nearSmoked] = await seen(page, [behind, before]);
  const red = await obs(page);
  ctx.check(
    "a bloomed cloud closes red's fog behind it and leaves the near side seen",
    !behindSmoked && nearSmoked,
    JSON.stringify({ behindSmoked, nearSmoked }),
  );
  ctx.check(
    "red sees the cloud, no longer the carrier, and keeps a last-contact marker where it vanished",
    red.smokeClouds.some((c) => c.id === cloud.id) &&
      !red.identified.some(
        (e) =>
          e.kind === "test_smoke_tank" && Math.hypot(e.position[0] - cx, e.position[1] - cy) < r,
      ) &&
      red.contacts.some((c) => Math.hypot(c.center[0] - cx, c.center[1] - cy) <= c.radius + 1),
    JSON.stringify({ clouds: red.smokeClouds.length, contacts: red.contacts }),
  );
  await snapshot(ctx, page, "red-smoked.png");
  await closeShot(ctx, page, cloud.centre, "red-smoked-close.png");
  await setSide(page, "blue");
  await snapshot(ctx, page, "blue-smoked.png");
  // Blue's other units see up to the cloud's rim on their side of it.
  const watchers = (await obs(page)).own.filter((u) => u.id !== pop.carrier);
  const rims = watchers.map((u) => {
    const k = Math.hypot(u.position[0] - cx, u.position[1] - cy);
    return [cx + ((u.position[0] - cx) / k) * (r + 3), cy + ((u.position[1] - cy) / k) * (r + 3)];
  });
  const rimSeen = await seen(page, rims);
  ctx.check(
    "blue sees the ground just outside its cloud on each of its other units' side",
    rimSeen.every(Boolean),
    JSON.stringify({ rims, rimSeen }),
  );
  await closeShot(ctx, page, cloud.centre, "blue-smoked-close.png");
  // A side's contacts are its own memory: switched to blue, red's marker
  // over blue's tank is gone with red's view.
  const panels = await lab(page, () => ({
    drawn: document.querySelectorAll("[data-contact]").length,
    published: window.__lab.route.observation().contacts.length,
  }));
  ctx.check(
    "blue's view draws only blue's contacts, none of red's",
    panels.drawn === panels.published,
    JSON.stringify(panels),
  );

  // Faded and gone: red's fog opens behind it again.
  await setSide(page, "red");
  await advanceTo(page, gone);
  const [behindGone] = await seen(page, [behind]);
  const after = await obs(page);
  ctx.check(
    "once the cloud is gone, red's fog opens behind it again",
    behindGone && !after.smokeClouds.some((c) => c.id === cloud.id),
    JSON.stringify({ behindGone, clouds: after.smokeClouds.length }),
  );
  await snapshot(ctx, page, "red-gone.png");
  await page.close();
  if (process.env.SMOKE_COST === "1") await measureCost(ctx);
}

/** Fog's GPU time at a hundred eyes (`?cost`): frames drawn with fog, without
 *  it and with every eye's map rebuilt each frame, in interleaved batches,
 *  before the pop and with twelve clouds standing; and the most maps any one
 *  frame rebuilt while the clouds bloomed, against the moved eyes' budget. */
async function measureCost(ctx) {
  const settings = await fixture("presentation.json");
  const budget = settings.fog_geometry.rebuild_eyes_per_frame;
  const encounter = await fixture("maps/smoke/encounters/cost.json");
  const pop = encounter.scripts.find((s) => s.order.kind === "pop_smoke").tick;
  const page = await openBattle(ctx, {
    viewport: { width: 1920, height: 1080 },
    url: new URL("/labs/smoke?cost", ctx.url).href,
  });
  await page.evaluate(() => setInterval(() => performance.clearMeasures(), 500));
  const batch = (mode) =>
    lab(
      page,
      async (mode) => {
        await window.__lab.suppressFog(mode === "off");
        let loop = 0;
        const rebuild = () => {
          window.__lab.fog().rebuildAll();
          loop = requestAnimationFrame(rebuild);
        };
        if (mode === "rebuild") rebuild();
        await new Promise((r) => setTimeout(r, 1500));
        cancelAnimationFrame(loop);
        return window.__lab.stats().gpu.meanMs;
      },
      mode,
    );
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  const measure = async () => {
    const rows = { on: [], off: [], rebuild: [] };
    for (let r = 0; r < 6; r++)
      for (const mode of ["on", "off", "rebuild"]) rows[mode].push(await batch(mode));
    await lab(page, () => window.__lab.suppressFog(false));
    return {
      offMs: median(rows.off),
      fogMs: median(rows.on.map((v, i) => v - rows.off[i])),
      rebuildAllMs: median(rows.rebuild.map((v, i) => v - rows.off[i])),
      samples: rows,
    };
  };
  await advanceTo(page, pop - 10);
  const result = { eyes: (await lab(page, () => window.__lab.stats().fog)).eyes, budget };
  result.clear = await measure();
  // Through the bloom a tick and a frame at a time: what each frame rebuilt.
  let most = 0;
  for (let t = pop - 10; t < pop + 60; t++) {
    await advance(page, 1);
    const fog = await lab(page, async () => {
      await window.__lab.frame();
      return window.__lab.stats().fog;
    });
    most = Math.max(most, fog.rebuilt);
  }
  result.mostRebuiltInAFrame = most;
  result.clouds = (await obs(page)).smokeClouds.length;
  result.smoke = await measure();
  result.adapter = await page.evaluate(() => window.__lab.adapter);
  await ctx.writeEvidence("smoke-cost.json", result);
  ctx.check(
    "twelve clouds bloom without a frame rebuilding past the moved eyes' budget",
    result.eyes >= 100 && result.clouds === 12 && most > 0 && most <= budget,
    JSON.stringify({
      eyes: result.eyes,
      clouds: result.clouds,
      most,
      budget,
      fogMs: [result.clear.fogMs, result.smoke.fogMs],
      rebuildAllMs: [result.clear.rebuildAllMs, result.smoke.rebuildAllMs],
    }),
  );
  await page.close();
}
