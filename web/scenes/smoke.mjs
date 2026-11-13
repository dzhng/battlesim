// Smoke screens on the smoke lab, through the player's own input: blue selects
// its road tank from the army deck and presses H. The tank's panel counts its
// charge down and shows HIDDEN once the cloud is up. Red, watching from across
// open ground, has its fog
// close over the ground behind the cloud once it blooms and open again once it
// has faded, while the ground on its near side stays lit; red keeps a
// last-contact marker where the tank vanished, and sees the cloud itself.
// Blue's own cloud is always its own to see, and each side's cloud draws over
// its fog veil. Then the drawn cloud, from a
// fresh battle: blue pops all three carriers' smoke, and the look tour shoots
// the clouds blooming, full, overlapping, over road, grass and wood, beside
// the burning wreck, at far zoom and in their fade. Every number is read from
// the encounter, the map and the published cloud. SMOKE_COST=1 also measures fog's GPU cost at a hundred eyes, with and
// without twelve clouds (run it alone, under the GPU lock).
import { readFile } from "node:fs/promises";
import { advance, aim, lab, obs, openBattle, snapshot } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";

const fixture = async (path) =>
  JSON.parse(await readFile(new URL(`../../fixtures/${path}`, import.meta.url), "utf8"));

/** When the scene pops the road tank's smoke: after blue's grass tank has
 *  shelled the jeep and held its fire again. */
const POP_TICK = 120;

/** The pop: its tick, and its carrier, the smoke tank on the map's road. */
async function roadPop() {
  const encounter = await fixture("maps/smoke/encounters/smoke.json");
  const map = await fixture("maps/smoke/map.json");
  const road = map.surfaces[0].shape.points[0][1];
  const carrier = encounter.units.findIndex(
    (u) => u.side === "blue" && u.kind === "test_smoke_tank" && u.position[1] === road,
  );
  return { tick: POP_TICK, carrier };
}

/** The floating panel's state rows of own unit `id`: each row's state and words. */
const panelRows = (page, id) =>
  lab(
    page,
    (id) =>
      [...document.querySelectorAll(`.ro-unit[data-unit="${id}"] .ro-state`)].map((r) => ({
        state: r.dataset.state,
        word: r.textContent.trim(),
      })),
    id,
  );

/** A shot of `centre` framed by `view` (close up by default), then the
 *  lab's own framing again. */
async function closeShot(ctx, page, centre, file, view = { distance: 90, pitch: 0.85 }) {
  const opening = await lab(page, () => window.__lab.camera());
  await aim(page, centre, view);
  const shot = await snapshot(ctx, page, file);
  await lab(page, (c) => window.__lab.setCamera(c), opening);
  return shot;
}

/** The fog mask over cloud `cloud`, framed as `closeShot` frames it: each
 *  point of a ring half way out and half way up, and the middle, by whether
 *  the mask reads it unseen (black in the mask view). */
async function veiledOver(ctx, page, cloud, file) {
  const opening = await lab(page, () => window.__lab.camera());
  await aim(page, cloud.centre, { distance: 90, pitch: 0.85 });
  const points = await lab(
    page,
    async ({ c, r, h }) => {
      await window.__lab.frame();
      const ring = Array.from({ length: 12 }, (_, k) => {
        const a = (k / 12) * 2 * Math.PI;
        return [c[0] + Math.cos(a) * r, c[1] + Math.sin(a) * r];
      });
      return [[c[0], c[1]], ...ring].map(([x, y]) => window.__lab.projectToCss(x, y, c[2] + h));
    },
    { c: cloud.centre, r: cloud.spec.radiusM * 0.5, h: cloud.spec.heightM * 0.5 },
  );
  await lab(page, () => window.__lab.setFrameView("fog-mask"));
  const mask = decode(await snapshot(ctx, page, file));
  await lab(page, () => window.__lab.setFrameView("final"));
  await lab(page, (c) => window.__lab.setCamera(c), opening);
  return points.map((p) => luma(pixel(mask, ...p)) < 8);
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
  const pop = await roadPop();
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

  // Blue pops it by hand: the tank's card, then H.
  await setSide(page, "blue");
  await advanceTo(page, pop.tick - 1);
  await page.locator(`.hud-army-card[data-unit="${pop.carrier}"]`).click();
  const charges = (await obs(page)).own.find((u) => u.id === pop.carrier).smoke;
  const rowsBefore = await panelRows(page, pop.carrier);
  await snapshot(ctx, page, "blue-before-pop-panel.png");
  await page.keyboard.press("KeyH");
  await page.waitForFunction(() =>
    window.__lab.route.acks().some((a) => a.order.kind === "pop_smoke"),
  );
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
  await page.evaluate(() => window.__lab.frame());
  const rowsAfter = await panelRows(page, pop.carrier);
  const word = (rows, state) => rows.find((r) => r.state === state)?.word ?? null;
  ctx.check(
    "H spends one charge, and the panel's SMOKE row counts it down",
    blue.own.find((u) => u.id === pop.carrier).smoke === charges - 1 &&
      word(rowsBefore, "smoke") !== word(rowsAfter, "smoke") &&
      word(rowsAfter, "smoke")?.includes(String(charges - 1)),
    JSON.stringify({ charges, rowsBefore, rowsAfter }),
  );
  await snapshot(ctx, page, "blue-pop-panel.png");
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
  await setSide(page, "blue");
  await page.locator(`.hud-army-card[data-unit="${pop.carrier}"]`).click();
  await page.evaluate(() => window.__lab.frame());
  const hidden = await panelRows(page, pop.carrier);
  ctx.check(
    "in its cloud, the carrier's panel shows HIDDEN",
    hidden.some((r) => r.state === "hidden"),
    JSON.stringify(hidden),
  );
  await snapshot(ctx, page, "blue-hidden-panel.png");
  await setSide(page, "red");
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
  // A side sees a cloud it sees whole, though not into it (D12): the cloud
  // draws over the veil, from the enemy's side and from its own.
  const redVeiled = await veiledOver(ctx, page, cloud, "red-smoked-mask.png");
  await setSide(page, "blue");
  await snapshot(ctx, page, "blue-smoked.png");
  const blueVeiled = await veiledOver(ctx, page, cloud, "blue-smoked-mask.png");
  ctx.check(
    "a cloud a side sees draws over its fog veil, from either side",
    !redVeiled.some(Boolean) && !blueVeiled.some(Boolean),
    JSON.stringify({ redVeiled, blueVeiled }),
  );
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
  await lookTour(ctx, pop);
  if (process.env.SMOKE_COST === "1") await measureCost(ctx);
}

/** Rec. 709 luminance and the spread of a pixel's channels (its colour). */
const luma = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
const spread = (p) => Math.max(...p.slice(0, 3)) - Math.min(...p.slice(0, 3));

/** The drawn cloud's look, shot by shot: blooming, full, overlapping, over
 *  road, grass and wood, beside the wreck, far off and fading. */
async function lookTour(ctx, pop) {
  const encounter = await fixture("maps/smoke/encounters/smoke.json");
  const blue = encounter.units.map((u, id) => ({ ...u, id })).filter((u) => u.side === "blue");
  const grass = blue.find((u) => u.kind === "test_smoke_tank" && u.id !== pop.carrier);
  const squad = blue.find((u) => u.kind === "test_smoke_squad");
  const road = blue.find((u) => u.id === pop.carrier);
  const wreck = encounter.units.find((u) => u.kind === "test_jeep").position;
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 } });
  const popAt = (unit) =>
    lab(page, (id) => window.__lab.route.command({ kind: "pop_smoke", units: [id] }), unit.id);
  const first = pop.tick - 60;
  await advanceTo(page, first - 2);
  const before = decode(await closeShot(ctx, page, grass.position, "look-grass-before.png"));
  await popAt(grass);
  await advanceTo(page, first + 2);
  await closeShot(ctx, page, grass.position, "look-grass-bloom.png");
  const cloud = (await obs(page)).smokeClouds.find(
    (c) => Math.hypot(c.centre[0] - grass.position[0], c.centre[1] - grass.position[1]) < 1,
  );
  const hz = (await fixture("rules.json")).tick_hz;
  await advanceTo(page, cloud.popped + Math.ceil(cloud.spec.bloomS * hz) + 30);
  const full = decode(await closeShot(ctx, page, grass.position, "look-grass-full.png"));
  // Round the cloud's middle, half its height up, as both close shots frame
  // it: brighter and greyer than the grass was there before the pop.
  const opening = await lab(page, () => window.__lab.camera());
  await aim(page, grass.position, { distance: 90, pitch: 0.85 });
  const ring = await lab(
    page,
    async ({ c, r, h }) => {
      await window.__lab.frame();
      return Array.from({ length: 12 }, (_, k) => {
        const a = (k / 12) * 2 * Math.PI;
        const [x, y] = [c[0] + Math.cos(a) * r, c[1] + Math.sin(a) * r];
        return window.__lab.projectToCss(x, y, c[2] + h);
      });
    },
    { c: cloud.centre, r: cloud.spec.radiusM * 0.5, h: cloud.spec.heightM * 0.5 },
  );
  await lab(page, (c) => window.__lab.setCamera(c), opening);
  const mean = (img, f) => ring.reduce((s, p) => s + f(pixel(img, ...p)), 0) / ring.length;
  const look = {
    luma: [mean(before, luma), mean(full, luma)],
    spread: [mean(before, spread), mean(full, spread)],
  };
  ctx.check(
    "a full cloud draws brighter and greyer than the grass it stands on",
    look.luma[1] > look.luma[0] && look.spread[1] < look.spread[0],
    JSON.stringify(look),
  );
  await snapshot(ctx, page, "look-default.png");
  await closeShot(ctx, page, grass.position, "look-far.png", { distance: 600, pitch: 0.95 });
  await advanceTo(page, pop.tick);
  await popAt(road);
  await advanceTo(page, pop.tick + 5);
  await closeShot(ctx, page, road.position, "look-road-bloom.png");
  await popAt(squad);
  await advanceTo(page, pop.tick + 50);
  const between = [
    (road.position[0] + grass.position[0]) / 2,
    (road.position[1] + grass.position[1]) / 2,
  ];
  await closeShot(ctx, page, between, "look-overlap.png", { distance: 110, pitch: 0.85 });
  await closeShot(ctx, page, road.position, "look-bubble.png", { distance: 40, pitch: 0.55 });
  await closeShot(ctx, page, squad.position, "look-forest.png");
  const nearWreck = [(grass.position[0] + wreck[0]) / 2, (grass.position[1] + wreck[1]) / 2];
  await closeShot(ctx, page, nearWreck, "look-wreck.png", { distance: 140, pitch: 0.8 });
  await setSide(page, "red");
  await closeShot(ctx, page, between, "look-red.png", { distance: 110, pitch: 0.85 });
  await setSide(page, "blue");
  // Half way through the grass cloud's fade.
  const fade = cloud.popped + Math.round((cloud.spec.lifeS - cloud.spec.fadeS / 2) * hz);
  await advanceTo(page, fade);
  await closeShot(ctx, page, grass.position, "look-grass-fade.png");
  await page.close();
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
