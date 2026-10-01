// Opt-in (`WATCH_TOURS=light`): the light weapons cast, in the watched village
// battle. From LIGHT_TICK (default 30) the battle steps a tick at a time; the
// first time each weapon's shot lights the world (a `flash:<kind>` cast light:
// the tank gun, the HMG, a rifle, a grenade launcher, a missile's launch), the
// first burst, and the first busy moment (LIGHT_BUSY shots' lights at once), it is
// framed at the default battle camera and close beside it, each with the cast
// lights on and off (`suppressCastLights`: the effects still draw), a
// before/after of the light alone at one tick. A missile is then followed in
// flight: three frames from one default-distance camera across its line, its
// motor's light streaking along the ground beneath it. A kind the battle
// never fires is found in the labs that fire it. The HUD hidden, the fixed
// seed and script. LIGHT_SUN=<elevation, radians> sets the sun lower (a dusk
// set, `?sun=`). Writes `throwaway/evidence/light/<label>/`: `<shot>-<view>-
// <on|off>.png`, `light.json` (ticks, cameras, the lights) and `sheet.png`.
// LIGHT_LABEL names the set (default `current`, or `dusk` with LIGHT_SUN).
import { mkdir, writeFile } from "node:fs/promises";
import { lab, advance, aim } from "./_lab.mjs";
import { game, curvePitch } from "./_units.mjs";

const CAMERA = game.presentation.camera;
const TICK_HZ = game.tick_hz;
const START = Number(process.env.LIGHT_TICK ?? 30);
const LAST = Number(process.env.LIGHT_LAST_TICK ?? 300 * TICK_HZ);
const SUN = process.env.LIGHT_SUN ?? null;
const LABEL = process.env.LIGHT_LABEL ?? (SUN ? "dusk" : "current");
const OUT = new URL(`../../throwaway/evidence/light/${LABEL}/`, import.meta.url);
const VIEWPORT = { width: 1920, height: 1080 };
/** Each shot: its name and the cast light that marks it. */
const SHOTS = [
  ["tank", (c) => c === "flash:tank_ap" || c === "flash:tank_he"],
  ["hmg", (c) => c === "flash:hmg"],
  ["rifle", (c) => c === "flash:rifle"],
  ["grenade", (c) => c === "flash:grenade"],
  ["atgm-launch", (c) => c === "flash:atgm"],
  ["burst", (c) => c === "blast"],
];
/** The busy moment: this many shots' lights (`flash:`) at once. */
const BUSY_FLASHES = Number(process.env.LIGHT_BUSY ?? 6);
/** ...within this many metres of one another: one default camera's view. */
const BUSY_REACH_M = 30;
/** Where a kind the battle never fires is found instead. */
const FALLBACK_LABS = [
  ["/lab/ambush", "late"],
  ["/lab/weapons", null],
];
/** The close view: how far, and its pitch. */
const CLOSE_M = 26;
const CLOSE_PITCH = 0.5;
/** A missile's flight frames: how many, the metres it flies between them,
 *  and the ticks its course is read over. */
const FLIGHT_FRAMES = 3;
const FLIGHT_SPAN_M = 18;
const COURSE_TICKS = 3;
/** LIGHT_COST=1 measures the lights' GPU cost at the busy moment and mid-flight. */
const COST = process.env.LIGHT_COST === "1";
const COST_BATCH_MS = 5000;

const lights = (page) => lab(page, () => window.__lab.route.castLights());
const tick = (page) => lab(page, () => window.__lab.route.tick());

/** `name`-on.png and `name`-off.png: the cast lights on, then off. */
async function pair(page, name) {
  await page.evaluate(() => window.__lab.frame());
  await writeFile(new URL(`${name}-on.png`, OUT), await page.screenshot());
  await lab(page, () => window.__lab.suppressCastLights(true));
  await writeFile(new URL(`${name}-off.png`, OUT), await page.screenshot());
  await lab(page, () => window.__lab.suppressCastLights(false));
}

/** LIGHT_COST=1: the cast lights' GPU cost at this paused moment and camera,
 *  paired: lit and unlit batches interleaved, each drawing frames for
 *  COST_BATCH_MS (past the timer's 240-frame rolling mean), the median of
 *  the differences. */
async function lightCost(page) {
  const batch = (off) =>
    lab(
      page,
      async ([off, ms]) => {
        await window.__lab.suppressCastLights(off);
        const until = performance.now() + ms;
        while (performance.now() < until) await window.__lab.frame();
        const s = window.__lab.stats();
        return { ms: s.gpu?.meanMs ?? NaN, lights: s.effects.lights };
      },
      [off, COST_BATCH_MS],
    );
  const on = [];
  const off = [];
  let lights = 0;
  for (let r = 0; r < 5; r++) {
    const b = await batch(false);
    on.push(b.ms);
    lights = b.lights;
    off.push((await batch(true)).ms);
  }
  await lab(page, () => window.__lab.suppressCastLights(false));
  const median = (v) => [...v].sort((a, b) => a - b)[Math.floor(v.length / 2)];
  return { lights, offMs: median(off), lightsMs: median(on.map((v, i) => v - off[i])) };
}

/** Frame the light `l` at the default camera and close, lit and unlit. */
async function frameLight(page, key, l, all) {
  const out = { tick: await tick(page), light: l, lights: all.length };
  await aim(page, l.at, {
    distance: CAMERA.default.distance,
    pitch: curvePitch(CAMERA.default.distance),
    yaw: CAMERA.default.yaw,
  });
  await pair(page, `${key}-default`);
  out.center = await lab(page, (p) => window.__lab.projectToCss(...p), l.at);
  await aim(page, l.at, { distance: CLOSE_M, pitch: CLOSE_PITCH, yaw: CAMERA.default.yaw });
  await pair(page, `${key}-close`);
  out.closeCenter = await lab(page, (p) => window.__lab.projectToCss(...p), l.at);
  return out;
}

/** The missile's motor light now, a frame drawn at the current tick first. */
async function motorNow(page) {
  await page.evaluate(() => window.__lab.frame());
  return (await lights(page)).find((l) => l.cause === "round:atgm") ?? null;
}

/** Follow the missile just launched: two readings of its motor's light give
 *  its course; one default-distance camera looking across the line
 *  ahead then takes FLIGHT_FRAMES frames as it flies FLIGHT_SPAN_M between
 *  each, and it is framed on its own mid-flight. */
async function followMissile(page, shots) {
  let a = null;
  for (let k = 0; k < 20 && !a; k++) {
    await advance(page, 1);
    a = await motorNow(page);
  }
  if (!a) return;
  await advance(page, COURSE_TICKS);
  const b = await motorNow(page);
  if (!b) return;
  const d = [b.at[0] - a.at[0], b.at[1] - a.at[1]];
  const n = Math.hypot(...d) || 1;
  const course = Math.atan2(d[1], d[0]);
  const yaw = course - Math.PI / 2;
  const distance = CAMERA.default.distance;
  // Over the middle of the stretch it will fly, looking across it.
  const center = [b.at[0] + (d[0] / n) * FLIGHT_SPAN_M, b.at[1] + (d[1] / n) * FLIGHT_SPAN_M];
  const frames = [];
  let m = b;
  for (let f = 0; f < FLIGHT_FRAMES; f++) {
    // On a tick at a time until it has flown FLIGHT_SPAN_M on (it speeds up).
    const from = m.at;
    while (m && f > 0 && Math.hypot(m.at[0] - from[0], m.at[1] - from[1]) < FLIGHT_SPAN_M) {
      await advance(page, 1);
      m = await motorNow(page);
    }
    if (!m) break;
    await aim(page, center, { distance, pitch: curvePitch(distance), yaw });
    await pair(page, `atgm-flight-${f + 1}`);
    const at = await lab(page, (p) => window.__lab.projectToCss(...p), m.at);
    frames.push({ tick: await tick(page), at: m.at, center: at });
    if (f === 1 && COST) {
      frames.at(-1).cost = await lightCost(page);
      console.log(`METRIC cast lights, missile in flight: ${JSON.stringify(frames.at(-1).cost)}`);
    }
    if (f === 1) shots["atgm-flight"] = await frameLight(page, "atgm-flight", m, []);
  }
  shots["atgm-sequence"] = { frames, camera: { center, distance, yaw } };
}

async function openPaused(ctx, url) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  const at = new URL(url);
  if (SUN) at.searchParams.set("sun", SUN);
  await ctx.openLab(page, at.href);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
    timeout: 30000,
  });
  await lab(page, () => window.__lab.route.pause());
  await page.addStyleTag({
    content: ".ro-unit, .lab-panel, [data-testid=battle-panel] { display: none !important; }",
  });
  return page;
}

/** Step the paused battle on `page` a tick at a time up to `last`, framing
 *  each wanted shot the first time its light burns, and the busy moment. */
async function scan(page, wanted, last, shots, wantBusy) {
  const done = () => wanted.every((k) => shots[k]) && (!wantBusy || shots.busy);
  while ((await tick(page)) < last && !done()) {
    await advance(page, 1);
    // A frame at the new tick first: the probe reads the last frame drawn.
    await page.evaluate(() => window.__lab.frame());
    const all = await lights(page);
    if (all.length === 0) continue;
    // Shots' lights within the default camera's view of one another.
    const shotLights = all.filter((l) => l.cause.startsWith("flash:"));
    const near = (l) =>
      shotLights.filter((q) => Math.hypot(q.at[0] - l.at[0], q.at[1] - l.at[1]) < BUSY_REACH_M);
    const densest = shotLights.reduce(
      (a, l) => (near(l).length > near(a).length ? l : a),
      shotLights[0],
    );
    const flashes = densest ? near(densest) : [];
    if (wantBusy && !shots.busy && flashes.length >= BUSY_FLASHES) {
      const c = [0, 1].map((i) => flashes.reduce((s, l) => s + l.at[i], 0) / flashes.length);
      const distance = CAMERA.default.distance;
      await aim(page, c, { distance, pitch: curvePitch(distance), yaw: CAMERA.default.yaw });
      await pair(page, "busy-default");
      const center = await lab(page, (p) => window.__lab.projectToCss(...p), [...c, densest.at[2]]);
      shots.busy = { tick: await tick(page), lights: all.length, flashes: flashes.length, center };
      if (COST) {
        shots.busy.cost = await lightCost(page);
        console.log(`METRIC cast lights, busy moment: ${JSON.stringify(shots.busy.cost)}`);
      }
    }
    for (const [key, marks] of SHOTS) {
      if (shots[key] || !wanted.includes(key)) continue;
      const l = all.find((q) => marks(q.cause));
      if (!l) continue;
      shots[key] = await frameLight(page, key, l, all);
      if (key === "atgm-launch") await followMissile(page, shots);
      break; // the battle has moved on: read it again
    }
  }
}

export async function lightTour(ctx) {
  await mkdir(OUT, { recursive: true });
  const keys = SHOTS.map(([k]) => k);
  const shots = {};
  const page = await openPaused(ctx, ctx.url);
  await advance(page, START - (await tick(page)));
  await scan(page, keys, LAST, shots, true);
  await page.close();
  const sources = {};
  for (const key of keys.filter((k) => !shots[k]))
    for (const [route, variant] of FALLBACK_LABS) {
      const alt = await openPaused(ctx, new URL(route, ctx.url).href);
      if (variant) {
        await lab(alt, (v) => window.__lab.route.variant(v), variant);
        await alt.waitForFunction(() => window.__lab.route?.reset);
        await lab(alt, () => window.__lab.route.reset());
        await alt.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, {
          timeout: 20000,
        });
        await lab(alt, () => window.__lab.route.pause());
      }
      await lab(alt, () => window.__lab.setFrameView("world"));
      await lab(alt, () => window.__lab.suppressPaint(true));
      await scan(alt, [key], (await tick(alt)) + 60 * TICK_HZ, shots, false);
      await alt.close();
      if (shots[key]) {
        sources[key] = route;
        break;
      }
    }
  const missing = keys.filter((k) => !shots[k]);
  ctx.check(
    "every weapon's shot is framed lit, with a burst and a busy moment",
    missing.length === 0 && !!shots.busy,
    JSON.stringify({ missing, sources, busy: shots.busy?.tick ?? null }),
  );
  await writeFile(
    new URL("light.json", OUT),
    JSON.stringify({ label: LABEL, sun: SUN, shots, sources }, null, 2),
  );
  const sheet = await ctx.newPage({ viewport: { width: 1600, height: 900 } });
  await contactSheet(sheet, shots);
  await sheet.close();
}

/** One row per shot: its default-camera and close crops, unlit then lit. */
async function contactSheet(page, shots) {
  const W = 480;
  const H = 300;
  const crop = (name, c) => {
    const [x, y] = c ?? [960, 540];
    const left = Math.min(Math.max(x - W / 2, 0), 1920 - W);
    const top = Math.min(Math.max(y - H / 2, 0), 1080 - H);
    return `<div class="crop" style="background-image:url(${new URL(name, OUT).href});background-position:${-left}px ${-top}px"></div>`;
  };
  let rows = "";
  for (const [key, shot] of Object.entries(shots)) {
    if (!shot || key === "atgm-sequence") continue;
    const c = shot.center ?? [960, 540];
    const cells = [
      crop(`${key}-default-off.png`, c),
      crop(`${key}-default-on.png`, c),
      shot.closeCenter ? crop(`${key}-close-off.png`, shot.closeCenter) : "",
      shot.closeCenter ? crop(`${key}-close-on.png`, shot.closeCenter) : "",
    ].join("");
    rows += `<div class="row"><div class="kind">${key}<small>tick ${shot.tick}</small></div>${cells}</div>`;
  }
  const seq = shots["atgm-sequence"];
  if (seq)
    rows += `<div class="row"><div class="kind">atgm flight<small>one camera, ${FLIGHT_SPAN_M} m apart</small></div>${seq.frames
      .map((f, i) => crop(`atgm-flight-${i + 1}-on.png`, f.center))
      .join("")}</div>`;
  const html = new URL("sheet.html", OUT);
  await writeFile(
    html,
    `<html><body><style>
    body { margin: 0; padding: 16px; background: #15171a; color: #eee; font: 600 18px ui-monospace, monospace; width: max-content; }
    .row, .heads { display: grid; grid-template-columns: 140px repeat(4, ${W}px); gap: 6px; margin-bottom: 6px; }
    .kind { display: flex; flex-direction: column; justify-content: center; } .kind small { font-size: 12px; color: #999; }
    .head { font-size: 15px; color: #bbb; }
    .crop { width: ${W}px; height: ${H}px; background-repeat: no-repeat; outline: 1px solid #333; }
  </style><div class="heads"><div>${LABEL}</div><div class="head">default, no cast light</div><div class="head">default, cast light</div><div class="head">close, no cast light</div><div class="head">close, cast light</div></div>${rows}</body></html>`,
  );
  await page.goto(html.href);
  await writeFile(new URL("sheet.png", OUT), await page.screenshot({ fullPage: true }));
}
