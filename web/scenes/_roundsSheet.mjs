// Opt-in (`WATCH_TOURS=rounds`): rounds in flight, in the watched street battle
// battle. The battle runs from ROUNDS_TICK (default 30); each round kind the
// fixture's weapons fire (rifle, HMG, grenade, tank AP and HE, ATGM) is
// framed the first time one of them has flown on for a short lead: at the
// default battle camera and from close beside its line of flight. The first
// busy moment (three or more kinds in flight at once) is framed too. A kind
// whose tracer row leaves a smoke trail is also framed at launch and, wide
// over its whole flight, a few seconds after it strikes. Daylight
// on grass, the HUD hidden, the fixed seed and script, so the same frames come
// back after a look change. Writes `throwaway/evidence/rounds/<label>/`:
// `<shot>-default.png`, `<shot>-close.png` (a shot is a kind, or a trail's
// `<kind>-launch` and `<kind>-after`), `busy.png`, `rounds.json` (ticks,
// cameras and each round's head and tail on the page) and `sheet.png`, one
// labelled row per shot. ROUNDS_LABEL names the set (default `current`);
// ROUNDS_FRAMES=<label> reuses that earlier set's close-view sides and sets
// its shots beside this run's on the sheet: a before/after of a look change.
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { lab, advance, openBattle, aim, hideHud } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { game, curvePitch } from "./_units.mjs";

const CAMERA = game.presentation.camera;
/** Every round kind, or ROUNDS_KINDS (comma-separated) while iterating. */
const KINDS = process.env.ROUNDS_KINDS?.split(",") ?? Object.keys(game.weapons);
const TICK_HZ = game.tick_hz;
/** The scan starts here, whatever tick the page paused at, so its samples repeat. */
const START = Number(process.env.ROUNDS_TICK ?? 30);
/** The battle's length: a kind never fired by then is reported missing. */
const LAST = Number(process.env.ROUNDS_LAST_TICK ?? 900 * TICK_HZ);
const LABEL = process.env.ROUNDS_LABEL ?? "current";
const OUT = new URL(`../../throwaway/evidence/rounds/${LABEL}/`, import.meta.url);
const VIEWPORT = { width: 1920, height: 1080 };
/** An earlier set whose close-view sides this run keeps (ROUNDS_FRAMES). */
const BEFORE = process.env.ROUNDS_FRAMES
  ? new URL(`../../throwaway/evidence/rounds/${process.env.ROUNDS_FRAMES}/`, import.meta.url)
  : null;
const FRAMES = BEFORE ? JSON.parse(await readFile(new URL("rounds.json", BEFORE), "utf8")) : null;
const SCAN = 15;
/** Where a kind the battle never fires is found instead: a lab that fires it. */
const FALLBACK_LABS = [
  // The AT lab's late escape: the team guides its missile all the way in.
  ["/lab/ambush", "late"],
  // The weapons lab: a squad lobs grenades over a building.
  ["/lab/weapons", null],
];
/** How close the close view stands, metres, and its pitch: steep enough to
 *  look over a house beside the round's line. */
const CLOSE_M = 30;
const CLOSE_PITCH = 0.55;
/** The busy moment: this many round kinds in flight at once. */
const BUSY_KINDS = 3;
/** Whether a kind's rounds leave a smoke trail (its tracer row's `smoke`). */
const TRAILS = (kind) => !!game.presentation.effects.tracers[kind]?.smoke;
/** A trail is framed this long after its round strikes, seconds. */
const AFTER_S = 4;
/** Each shot's row: a kind, and a trail's launch and aftermath around it. */
const ROWS = KINDS.flatMap((k) => (TRAILS(k) ? [`${k}-launch`, k, `${k}-after`] : [k]));

/** Each round in flight this tick: kind, head, the stretch's start, hit, own. */
const rounds = (page) =>
  lab(page, () =>
    window.__lab.route.observation().projectiles.map((p) => ({
      kind: p.kind,
      head: p.path.at(-1),
      from: p.path[0],
      hit: p.hit,
      own: p.own,
    })),
  );
const tick = (page) => lab(page, () => window.__lab.route.tick());

async function shoot(page, name, marks) {
  await page.evaluate(() => window.__lab.frame());
  await writeFile(new URL(name, OUT), await page.screenshot());
  const onPage = [];
  for (const m of marks) onPage.push(await lab(page, (p) => window.__lab.projectToCss(...p), m));
  return onPage;
}

/** Whether the round's effects show within a few pixels of `head`: the
 *  frame with effects less the frame without. */
async function shows(page, head) {
  const at = await lab(page, (p) => window.__lab.projectToCss(...p), head);
  if (!at || at[0] < 0 || at[1] < 0 || at[0] >= VIEWPORT.width || at[1] >= VIEWPORT.height)
    return false;
  await page.evaluate(() => window.__lab.frame());
  const on = decode(await page.screenshot());
  await lab(page, () => window.__lab.suppressEffects(true));
  const off = decode(await page.screenshot());
  await lab(page, () => window.__lab.suppressEffects(false));
  let most = 0;
  for (let dy = -4; dy <= 4; dy++)
    for (let dx = -4; dx <= 4; dx++) {
      const x = Math.round(at[0]) + dx;
      const y = Math.round(at[1]) + dy;
      if (x < 0 || y < 0 || x >= on.width || y >= on.height) continue;
      const i = (y * on.width + x) * 4;
      const lit = (png) => png.data[i] + png.data[i + 1] + png.data[i + 2];
      most = Math.max(most, lit(on) - lit(off));
    }
  return most > 60;
}

const dirOf = (r) => {
  const d = [r.head[0] - r.from[0], r.head[1] - r.from[1], r.head[2] - r.from[2]];
  const n = Math.hypot(...d) || 1;
  return d.map((v) => v / n);
};

/** The round `r`'s next stretch, one tick on: the one starting where it ended. */
async function next(page, r) {
  await advance(page, 1);
  return (await rounds(page)).find(
    (q) => q.kind === r.kind && Math.hypot(...q.from.map((v, i) => v - r.head[i])) < 1e-3,
  );
}

/** Follow the round `r` one tick at a time for `ticks`; null when it
 *  strikes or is lost first. */
async function follow(page, r, ticks) {
  let at = r;
  for (let k = 0; k < ticks; k++) {
    at = await next(page, at);
    if (!at || at.hit !== "none") return null;
  }
  return at;
}

/** Follow the round `r` until it strikes or is lost: its last stretch. */
async function flyOut(page, r) {
  let at = r;
  for (let k = 0; k < 30 * TICK_HZ; k++) {
    const q = await next(page, at);
    if (!q) return at;
    at = q;
    if (at.hit !== "none") return at;
  }
  return at;
}

/** Frame the round `r` as shot `key`: at the default camera (or `far`
 *  metres) over its head, and close beside its line of flight. */
async function frameRound(page, key, r, far = CAMERA.default.distance) {
  const d = dirOf(r);
  const back = (m) => [r.head[0] - d[0] * m, r.head[1] - d[1] * m, r.head[2] - d[2] * m];
  const marks = [r.head, back(10), back(40)];
  const out = { kind: r.kind, tick: await tick(page), head: r.head, dir: d, own: r.own };
  // The player's view: the default camera over the round.
  await aim(page, r.head, { distance: far, pitch: curvePitch(far), yaw: CAMERA.default.yaw });
  out.default = { distance: far, page: await shoot(page, `${key}-default.png`, marks) };
  // Close beside its line of flight, looking across it: from the side
  // where nothing stands between the camera and the round (its effects
  // show at the head), else the first.
  // ROUNDS_FRAMES=<label> takes that set's side, so a look change is judged
  // from the same framing.
  const course = Math.atan2(d[1], d[0]);
  let yaw = FRAMES?.shots[key]?.close.yaw ?? course - Math.PI / 2;
  for (const side of FRAMES ? [] : [course - Math.PI / 2, course + Math.PI / 2]) {
    await aim(
      page,
      back(4),
      { distance: CLOSE_M, pitch: CLOSE_PITCH, yaw: side },
      { onGround: false },
    );
    if (await shows(page, r.head)) {
      yaw = side;
      break;
    }
  }
  await aim(page, back(4), { distance: CLOSE_M, pitch: CLOSE_PITCH, yaw }, { onGround: false });
  // The world alone: no overlay (a lab's marks) over the round.
  await lab(page, () => window.__lab.setFrameView("world"));
  out.close = { distance: CLOSE_M, yaw, page: await shoot(page, `${key}-close.png`, marks) };
  if (!page.worldView) await lab(page, () => window.__lab.setFrameView("final"));
  return out;
}

/** Open `url` paused at its start, the HUD hidden. */
async function openPaused(ctx, url) {
  const page = await openBattle(ctx, { viewport: VIEWPORT, url, grass: true });
  await hideHud(page);
  return page;
}

/** Scan the paused battle on `page` up to tick `last` for the kinds `wanted`
 *  not yet in `shots`, framing each; and the first busy moment when `busy`
 *  is asked for. `seen` tallies scanned ticks each kind flew in. */
async function scan(page, wanted, last, shots, seen, wantBusy) {
  let busy = null;
  const done = () => wanted.every((k) => shots[k]) && (!wantBusy || busy);
  while ((await tick(page)) < last && !done()) {
    await advance(page, SCAN);
    const now = await rounds(page);
    const kinds = new Set(now.map((r) => r.kind));
    for (const k of kinds) seen[k] = (seen[k] ?? 0) + 1;
    if (wantBusy && !busy && kinds.size >= BUSY_KINDS) {
      const c = [0, 1].map((i) => now.reduce((s, r) => s + r.head[i], 0) / now.length);
      const spread = Math.max(...now.map((r) => Math.hypot(r.head[0] - c[0], r.head[1] - c[1])));
      const distance = Math.min(260, Math.max(CAMERA.default.distance, spread * 1.6));
      await aim(page, c, { distance, pitch: curvePitch(distance), yaw: CAMERA.default.yaw });
      busy = {
        tick: await tick(page),
        kinds: [...kinds],
        count: now.length,
        center: c,
        distance,
        page: await shoot(page, "busy.png", []),
      };
    }
    for (const kind of kinds) {
      if (shots[kind] || !wanted.includes(kind)) continue;
      const r = now.find((q) => q.kind === kind && q.hit === "none");
      if (!r) continue;
      const w = game.weapons[kind];
      // Far enough into its flight for its trail to show; ATGMs and shells longest.
      const lead = Math.round(Math.min(1, (0.3 * w.range_m) / w.speed_mps) * TICK_HZ);
      const trails = TRAILS(kind);
      if (trails) shots[`${kind}-launch`] = await frameRound(page, `${kind}-launch`, r);
      const flown = await follow(page, r, lead);
      if (flown) shots[kind] = await frameRound(page, kind, flown);
      if (flown && trails) {
        // Wide over the whole flight, from launch to the strike, its trail
        // still hanging a few seconds on.
        const end = (await flyOut(page, flown)).head;
        await advance(page, AFTER_S * TICK_HZ);
        const mid = r.from.map((v, i) => (v + end[i]) / 2);
        const span = Math.hypot(end[0] - r.from[0], end[1] - r.from[1]);
        const wide = Math.min(400, Math.max(CAMERA.default.distance, span * 0.8));
        const after = { kind, from: r.from, head: mid, hit: "none", own: r.own };
        shots[`${kind}-after`] = await frameRound(page, `${kind}-after`, after, wide);
      }
      break; // the battle has moved on: read it again
    }
  }
  return busy;
}

export async function roundsTour(ctx) {
  await mkdir(OUT, { recursive: true });
  const page = await openPaused(ctx, ctx.url);
  await advance(page, START - (await tick(page)));
  const shots = {};
  /** Ticks each kind was seen in flight, of those scanned. */
  const seen = Object.fromEntries(KINDS.map((k) => [k, 0]));
  const busy = await scan(page, KINDS, LAST, shots, seen, !process.env.ROUNDS_KINDS);
  await page.close();
  // A kind the watched battle never fires (on this seed and script) from the
  // labs that fire it within a minute, their lab overlays hidden (the world
  // view, every effect and no overlay, and no ground paint).
  const sources = {};
  for (const kind of KINDS.filter((k) => !shots[k]))
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
      await advance(alt, START - (await tick(alt)));
      await lab(alt, () => window.__lab.setFrameView("world"));
      alt.worldView = true;
      await lab(alt, () => window.__lab.suppressPaint(true));
      await scan(alt, [kind], 60 * TICK_HZ, shots, seen, false);
      await alt.close();
      if (shots[kind]) {
        sources[kind] = route;
        break;
      }
    }
  const missing = KINDS.filter((k) => !shots[k]);
  ctx.check(
    "every round kind is framed in flight, and a busy moment",
    missing.length === 0 && (!!busy || !!process.env.ROUNDS_KINDS),
    JSON.stringify({ missing, seen, sources, busy: busy?.tick ?? null }),
  );
  const record = {
    label: LABEL,
    kinds: KINDS,
    rows: ROWS,
    shots,
    sources,
    seen,
    busy,
    viewport: VIEWPORT,
  };
  await writeFile(new URL("rounds.json", OUT), JSON.stringify(record, null, 2));
  const sheet = await ctx.newPage({ viewport: { width: 1600, height: 900 } });
  const sets = [{ label: LABEL, dir: OUT, record }];
  if (FRAMES) sets.unshift({ label: FRAMES.label, dir: BEFORE, record: FRAMES });
  await contactSheet(sheet, sets, new URL("sheet.png", OUT));
  await sheet.close();
}

/** A labelled contact sheet: one row per shot of the last set, and for
 *  each set (a column group: before, after) its default-camera and close
 *  crops around the round, then the busy moments. Each set is `{label, dir,
 *  record}`. */
async function contactSheet(page, sets, file) {
  // The shots by file URL, the sheet written beside them and opened: inlined
  // as data, a before/after sheet's shots are too much for one page.
  const png = (dir, name) => new URL(name, dir).href;
  const W = 560;
  const H = 315;
  /** A crop of `W`×`H` at `zoom` centred between the round's head and 10 m back. */
  const crop = (src, marks, zoom) => {
    const [h, t] = marks;
    const c = h && t ? [(h[0] + t[0]) / 2, (h[1] + t[1]) / 2] : [960, 540];
    const x = Math.min(Math.max(c[0] * zoom - W / 2, 0), 1920 * zoom - W);
    const y = Math.min(Math.max(c[1] * zoom - H / 2, 0), 1080 * zoom - H);
    return `<div class="crop" style="background-image:url(${src});background-size:${1920 * zoom}px ${1080 * zoom}px;background-position:${-x}px ${-y}px"></div>`;
  };
  const last = sets.at(-1).record;
  let rows = "";
  for (const key of last.rows) {
    const shot = last.shots[key];
    const from = shot && last.sources[shot.kind];
    const far = shot && shot.default.distance !== CAMERA.default.distance;
    const note = [from, far && `${Math.round(shot.default.distance)} m`].filter(Boolean);
    let cells = `<div class="kind">${key}${note.length ? `<small>(${note.join(", ")})</small>` : ""}</div>`;
    for (const s of sets) {
      const shot = s.record.shots[key];
      if (!shot) {
        cells += `<div class="none">not shot</div><div class="none">not shot</div>`;
        continue;
      }
      // A wide shot whole; the player's view at 1:1 round the round.
      const zoom = shot.default.distance === CAMERA.default.distance ? 1 : W / 1920;
      cells += crop(png(s.dir, `${key}-default.png`), shot.default.page, zoom);
      cells += crop(png(s.dir, `${key}-close.png`), shot.close.page, 1);
    }
    rows += `<div class="row">${cells}</div>`;
  }
  const heads = sets
    .map(
      (s) =>
        `<div class="head">${s.label}: default camera (65 m, 1:1 crop)</div><div class="head">${s.label}: close (30 m, beside its flight, no overlays)</div>`,
    )
    .join("");
  let busy = "";
  for (const s of sets)
    if (s.record.busy)
      busy += `<figure><img src="${png(s.dir, "busy.png")}"><figcaption>${s.label}: busy moment, tick ${s.record.busy.tick}, ${s.record.busy.count} rounds (${s.record.busy.kinds.join(", ")})</figcaption></figure>`;
  const cols = sets.length * 2;
  const html = new URL(file.href.replace(/\.png$/, ".html"));
  await writeFile(
    html,
    `<html><body><style>
    body { margin: 0; padding: 16px; background: #15171a; color: #eee; font: 600 18px ui-monospace, monospace; width: max-content; }
    .row, .heads { display: grid; grid-template-columns: 120px repeat(${cols}, ${W}px); gap: 6px; margin-bottom: 6px; }
    .kind { display: flex; flex-direction: column; justify-content: center; font-size: 22px; } .kind small { font-size: 12px; color: #999; }
    .head { font-size: 15px; color: #bbb; }
    .crop, .none { width: ${W}px; height: ${H}px; background-repeat: no-repeat; outline: 1px solid #333; }
    .none { display: flex; align-items: center; justify-content: center; color: #777; }
    figure { margin: 12px 0 0; } figure img { width: ${120 + cols * (W + 6)}px; display: block; }
  </style><div class="heads"><div></div>${heads}</div>${rows}${busy}</body></html>`,
  );
  await page.goto(html.href);
  await writeFile(file, await page.screenshot({ fullPage: true }));
}
