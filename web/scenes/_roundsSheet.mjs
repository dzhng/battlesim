// Opt-in (`WATCH_TOURS=rounds`): rounds in flight, in the watched village
// battle. The battle runs from ROUNDS_TICK (default 30); each round kind the
// fixture's weapons fire (rifle, HMG, grenade, tank AP and HE, ATGM) is
// framed the first time one of them has flown on for a short lead: at the
// default battle camera and from close beside its line of flight. The first
// busy moment (three or more kinds in flight at once) is framed too. Daylight
// on grass, the HUD hidden, the fixed seed and script, so the same frames come
// back after a look change. Writes `throwaway/evidence/rounds/<label>/`:
// `<kind>-default.png`, `<kind>-close.png`, `busy.png`, `rounds.json` (ticks,
// cameras and each round's head and tail on the page) and `sheet.png`, one
// labelled row per kind. ROUNDS_LABEL names the set (default `current`).
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { lab, advance } from "./_lab.mjs";
import { decode } from "./_png.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
/** Every round kind, or ROUNDS_KINDS (comma-separated) while iterating. */
const KINDS = process.env.ROUNDS_KINDS?.split(",") ?? Object.keys(village.weapons);
const TICK_HZ = village.tick_hz;
/** The scan starts here, whatever tick the page paused at, so its samples repeat. */
const START = Number(process.env.ROUNDS_TICK ?? 30);
/** The battle's length: a kind never fired by then is reported missing. */
const LAST = Number(process.env.ROUNDS_LAST_TICK ?? 900 * TICK_HZ);
const LABEL = process.env.ROUNDS_LABEL ?? "current";
const OUT = new URL(`../../throwaway/evidence/rounds/${LABEL}/`, import.meta.url);
const VIEWPORT = { width: 1920, height: 1080 };
/** An earlier set whose close-view sides this run keeps (ROUNDS_FRAMES). */
const FRAMES = process.env.ROUNDS_FRAMES
  ? JSON.parse(
      await readFile(
        new URL(
          `../../throwaway/evidence/rounds/${process.env.ROUNDS_FRAMES}/rounds.json`,
          import.meta.url,
        ),
        "utf8",
      ),
    )
  : null;
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

/** The camera curve's pitch at `distance` (the controller's own rule). */
function curvePitch(distance) {
  const curve = CAMERA.pitch_curve;
  if (distance <= curve[0][0]) return curve[0][1];
  for (let k = 1; k < curve.length; k++)
    if (distance <= curve[k][0]) {
      const [[d0, p0], [d1, p1]] = [curve[k - 1], curve[k]];
      return p0 + ((p1 - p0) * (distance - d0)) / (d1 - d0);
    }
  return curve.at(-1)[1];
}

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

/** The camera on `at`: on the ground under it, or at its own height (`at[2]`). */
const pose = (page, at, distance, pitch, yaw, onGround = true) =>
  lab(
    page,
    (c) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        distance: c.distance,
        pitch: c.pitch,
        yaw: c.yaw,
        target: [
          c.at[0],
          c.at[1],
          c.onGround ? window.__lab.route.surfaceZ(c.at[0], c.at[1]) : c.at[2],
        ],
      }),
    { at, distance, pitch, yaw, onGround },
  );

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

/** Follow the round `r` one tick at a time for `ticks`, by its stretch
 *  starting where the last ended; null when it strikes or is lost first. */
async function follow(page, r, ticks) {
  let at = r;
  for (let k = 0; k < ticks; k++) {
    await advance(page, 1);
    const next = (await rounds(page)).find(
      (q) => q.kind === at.kind && Math.hypot(...q.from.map((v, i) => v - at.head[i])) < 1e-3,
    );
    if (process.env.ROUNDS_DEBUG) console.log("follow", k, at.kind, at.head, next ?? null);
    if (!next || next.hit !== "none") return null;
    at = next;
  }
  return at;
}

async function frameRound(page, kind, r) {
  const d = dirOf(r);
  const back = (m) => [r.head[0] - d[0] * m, r.head[1] - d[1] * m, r.head[2] - d[2] * m];
  const marks = [r.head, back(10), back(40)];
  const out = { kind, tick: await tick(page), head: r.head, dir: d, own: r.own };
  // The player's view: the default camera over the round.
  const far = CAMERA.default.distance;
  await pose(page, r.head, far, curvePitch(far), CAMERA.default.yaw);
  out.default = { distance: far, page: await shoot(page, `${kind}-default.png`, marks) };
  // Close beside its line of flight, looking across it: from the side
  // where nothing stands between the camera and the round (its effects
  // show at the head), else the first.
  // ROUNDS_FRAMES=<label> takes that set's side, so a look change is judged
  // from the same framing.
  const course = Math.atan2(d[1], d[0]);
  let yaw = FRAMES?.shots[kind]?.close.yaw ?? course - Math.PI / 2;
  for (const side of FRAMES ? [] : [course - Math.PI / 2, course + Math.PI / 2]) {
    await pose(page, back(4), CLOSE_M, CLOSE_PITCH, side, false);
    if (await shows(page, r.head)) {
      yaw = side;
      break;
    }
  }
  await pose(page, back(4), CLOSE_M, CLOSE_PITCH, yaw, false);
  // The world alone: no overlay (a guided missile's marker) over the round.
  await lab(page, () => window.__lab.setFrameView("world"));
  out.close = { distance: CLOSE_M, yaw, page: await shoot(page, `${kind}-close.png`, marks) };
  if (!page.worldView) await lab(page, () => window.__lab.setFrameView("final"));
  return out;
}

/** Open `url` paused at its start, the HUD hidden. */
async function openPaused(ctx, url) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(page, url);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
    timeout: 30000,
  });
  await lab(page, () => window.__lab.route.pause());
  await page.addStyleTag({
    content: ".ro-unit, [data-testid=battle-panel] { display: none !important; }",
  });
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
      await pose(page, c, distance, curvePitch(distance), CAMERA.default.yaw);
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
      const w = village.weapons[kind];
      // Far enough into its flight for its trail to show; ATGMs and shells longest.
      const lead = Math.round(Math.min(1, (0.3 * w.range_m) / w.speed_mps) * TICK_HZ);
      const flown = await follow(page, r, lead);
      if (flown) shots[kind] = await frameRound(page, kind, flown);
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
  const record = { label: LABEL, kinds: KINDS, shots, sources, seen, busy, viewport: VIEWPORT };
  await writeFile(new URL("rounds.json", OUT), JSON.stringify(record, null, 2));
  const sheet = await ctx.newPage({ viewport: { width: 1600, height: 900 } });
  await contactSheet(sheet, [{ label: LABEL, dir: OUT, record }], new URL("sheet.png", OUT));
  await sheet.close();
}

/** A labelled contact sheet: one row per kind, and for each set (a column
 *  group: before, after) its default-camera and close crops around the round,
 *  then the busy moments. Each set is `{label, dir, record}`. */
export async function contactSheet(page, sets, file) {
  const png = async (dir, name) =>
    `data:image/png;base64,${(await readFile(new URL(name, dir))).toString("base64")}`;
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
  const kinds = sets[0].record.kinds;
  let rows = "";
  for (const kind of kinds) {
    const from = sets[0].record.sources?.[kind];
    let cells = `<div class="kind">${kind}${from ? `<small>(${from})</small>` : ""}</div>`;
    for (const s of sets) {
      const shot = s.record.shots[kind];
      if (!shot) {
        cells += `<div class="none">not fired</div><div class="none">not fired</div>`;
        continue;
      }
      cells += crop(await png(s.dir, `${kind}-default.png`), shot.default.page, 1);
      cells += crop(await png(s.dir, `${kind}-close.png`), shot.close.page, 1);
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
      busy += `<figure><img src="${await png(s.dir, "busy.png")}"><figcaption>${s.label}: busy moment, tick ${s.record.busy.tick}, ${s.record.busy.count} rounds (${s.record.busy.kinds.join(", ")})</figcaption></figure>`;
  const cols = sets.length * 2;
  await page.setContent(`<html><body><style>
    body { margin: 0; padding: 16px; background: #15171a; color: #eee; font: 600 18px ui-monospace, monospace; width: max-content; }
    .row, .heads { display: grid; grid-template-columns: 120px repeat(${cols}, ${W}px); gap: 6px; margin-bottom: 6px; }
    .kind { display: flex; flex-direction: column; justify-content: center; font-size: 22px; } .kind small { font-size: 12px; color: #999; }
    .head { font-size: 15px; color: #bbb; }
    .crop, .none { width: ${W}px; height: ${H}px; background-repeat: no-repeat; outline: 1px solid #333; }
    .none { display: flex; align-items: center; justify-content: center; color: #777; }
    figure { margin: 12px 0 0; } figure img { width: ${120 + cols * (W + 6)}px; display: block; }
  </style><div class="heads"><div></div>${heads}</div>${rows}${busy}</body></html>`);
  await writeFile(file, await page.screenshot({ fullPage: true }));
}
