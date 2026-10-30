// Opt-in look sheet: the battle's resolved weapon appearance keys, with
// launch, flight, contact and blast frames. ROUNDS_FRAMES keeps an earlier
// set's cameras; ROUNDS_KINDS narrows iteration without changing the default.
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { MECHANICS_API } from "../../apps/mechanics-editor/src/protocol.ts";
import { lab, advance, openBattle, aim, hideHud } from "./_lab.mjs";
import { PNG } from "pngjs";
import { capture, decode } from "./_png.mjs";
import { rules, weapons, curvePitch, visuals } from "./_units.mjs";

const CAMERA = visuals.camera;
const LOOKS = visuals.weapon_looks;
/** One row for every authored look, optionally narrowed while iterating. */
const KINDS = process.env.ROUNDS_KINDS?.split(",") ?? Object.keys(LOOKS);
const weapon = (id) => weapons[id];
const lookOf = (id) => weapons[id].appearance;
const TICK_HZ = rules.tick_hz;
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
const SCAN = 1;
/** How close the close view stands, metres, and its pitch: steep enough to
 *  look over a house beside the round's line. */
const CLOSE_M = 30;
const CLOSE_PITCH = 0.55;
/** The busy moment: this many round looks in flight at once. */
const BUSY_KINDS = 3;
/** Whether a look's rounds leave a smoke trail (its tracer row's `smoke`). */
const TRAILS = (kind) => !!LOOKS[kind]?.tracer.smoke;
/** A trail is framed this long after its round strikes, seconds. */
const AFTER_S = 4;
/** Each shot's row: a kind, and a trail's launch and aftermath around it. */
const ROWS = KINDS.flatMap((k) => [
  `${k}-launch`,
  k,
  `${k}-impact`,
  `${k}-blast`,
  ...(TRAILS(k) ? [`${k}-after`] : []),
]);

/** Each round in flight this tick: its look (`kind`) and weapon row, head,
 *  the stretch's start, hit, own. */
const rounds = async (page) =>
  (
    await lab(page, () =>
      window.__lab.route.observation().projectiles.map((p) => ({
        weapon: p.kind,
        head: p.path.at(-1),
        from: p.path[0],
        hit: p.hit,
        own: p.own,
      })),
    )
  )
    .filter((r) => !page.reviewLook || r.weapon === "tank_he")
    .map((r) => ({ ...r, kind: page.reviewLook ?? lookOf(r.weapon) }));
const tick = (page) => lab(page, () => window.__lab.route.tick());

async function shoot(page, name, marks) {
  await page.evaluate(() => window.__lab.frame());
  await writeFile(new URL(name, OUT), await page.screenshot());
  const onPage = [];
  for (const m of marks) onPage.push(await lab(page, (p) => window.__lab.projectToCss(...p), m));
  return onPage;
}

/** Whether the round's effects show along its latest stretch: presentation
 *  can draw the head between the publication's endpoints. */
async function shows(page, head, from) {
  const [at, tail] = await lab(
    page,
    (points) => points.map((p) => window.__lab.projectToCss(...p)),
    [head, from],
  );
  if (!at || !tail || at[0] < 0 || at[1] < 0 || at[0] >= VIEWPORT.width || at[1] >= VIEWPORT.height)
    return false;
  await page.evaluate(() => window.__lab.frame());
  const on = decode(await page.screenshot());
  await lab(page, () => window.__lab.suppressEffects(true));
  const off = decode(await page.screenshot());
  await lab(page, () => window.__lab.suppressEffects(false));
  let most = 0;
  const steps = Math.max(1, Math.ceil(Math.hypot(tail[0] - at[0], tail[1] - at[1]) / 8));
  for (let step = 0; step <= steps; step++)
    for (let dy = -4; dy <= 4; dy++)
      for (let dx = -4; dx <= 4; dx++) {
        const x = Math.round(at[0] + ((tail[0] - at[0]) * step) / steps) + dx;
        const y = Math.round(at[1] + ((tail[1] - at[1]) * step) / steps) + dy;
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
    (q) => q.weapon === r.weapon && Math.hypot(...q.from.map((v, i) => v - r.head[i])) < 1e-3,
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
  const d = r.direction ?? dirOf(r);
  const launch = key.endsWith("-launch");
  const aftermath = key.endsWith("-after");
  const contact = key.endsWith("-impact") || key.endsWith("-blast");
  const close = aftermath
    ? far
    : contact
      ? 60
      : launch
        ? Math.max(10, LOOKS[r.kind].flash.fireball_m * 8)
        : CLOSE_M;
  const pitch = aftermath ? 1.1 : CLOSE_PITCH;
  const center =
    launch || contact || aftermath ? [r.head[0], r.head[1], r.head[2] + (contact ? 4 : 1)] : null;
  const back = (m) => [r.head[0] - d[0] * m, r.head[1] - d[1] * m, r.head[2] - d[2] * m];
  const marks = [r.head, back(10), back(40)];
  const out = { kind: r.kind, tick: await tick(page), head: r.head, dir: d, own: r.own };
  // The player's view: the default camera over the round.
  await aim(
    page,
    center ?? r.head,
    { distance: far, pitch: curvePitch(far), yaw: CAMERA.default.yaw },
    { onGround: false },
  );
  out.default = { distance: far, page: await shoot(page, `${key}-default.png`, marks) };
  // Close beside its line of flight, looking across it: from the side
  // where nothing stands between the camera and the round (its effects
  // show at the head), else the first.
  // ROUNDS_FRAMES=<label> takes that set's side, so a look change is judged
  // from the same framing.
  const course = Math.atan2(d[1], d[0]);
  let yaw = FRAMES?.shots[key]?.close.yaw ?? course - Math.PI / 2;
  let visible = false;
  for (const side of FRAMES ? [] : [course - Math.PI / 2, course + Math.PI / 2]) {
    await aim(page, center ?? back(4), { distance: close, pitch, yaw: side }, { onGround: false });
    if (await shows(page, r.head, r.from)) {
      yaw = side;
      visible = true;
      break;
    }
  }
  await aim(page, center ?? back(4), { distance: close, pitch, yaw }, { onGround: false });
  if (FRAMES) visible = await shows(page, r.head, r.from);
  out.visible = visible;
  out.close = { distance: close, yaw, page: await shoot(page, `${key}-close.png`, marks) };
  if (launch) {
    out.lights = await lab(page, () => window.__lab.route.castLights());
    if (
      page.reviewLook &&
      !out.lights.some(
        (light) =>
          light.cause === `flash:${r.kind}` &&
          Math.hypot(...light.at.map((v, i) => v - r.head[i])) < 3,
      )
    )
      throw new Error(
        `review look ${r.kind} did not reach the rendered flash: ${JSON.stringify(out.lights)}`,
      );
  }
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
  // A hull publishes its shot counter a tick before its first flight segment.
  // Capture that moment: a short rifle-style flash can be gone next tick.
  const lane = page.reviewLook
    ? await lab(page, () => window.__lab.route.lanes().find((l) => l.weapon === "tank_he"))
    : null;
  let fired = 0;
  const done = () => wanted.every((k) => shots[k]) && (!wantBusy || busy);
  while ((await tick(page)) < last && !done()) {
    await advance(page, SCAN);
    if (lane && !shots[`${page.reviewLook}-launch`]) {
      const shooter = await lab(
        page,
        (from) =>
          window.__lab.route
            .observation()
            .own.find((u) => Math.hypot(u.position[0] - from[0], u.position[1] - from[1]) < 2),
        lane.from,
      );
      const pose = shooter?.weaponPoses[0];
      if (pose && pose.shots > fired) {
        const head = await lab(page, (id) => window.__lab.route.drawnMuzzle(id, 0), shooter.id);
        if (!head) throw new Error("review lane has no drawn muzzle");
        const kind = page.reviewLook;
        shots[`${kind}-launch`] ??= await frameRound(page, `${kind}-launch`, {
          kind,
          weapon: "tank_he",
          head,
          from: head,
          own: true,
          direction: [Math.cos(pose.bearing), Math.sin(pose.bearing), 0],
        });
      }
      fired = pose?.shots ?? fired;
    }
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
      const r = now.find(
        (q) =>
          q.kind === kind &&
          q.hit === "none" &&
          (!lane || Math.hypot(q.from[0] - lane.from[0], q.from[1] - lane.from[1]) < 10),
      );
      if (!r) continue;
      const w = weapon(r.weapon);
      // Far enough into its flight for its trail to show; ATGMs and shells longest.
      const lead = Math.round(Math.min(1, (0.3 * w.range_m) / w.speed_mps) * TICK_HZ);
      const trails = TRAILS(kind);
      if (!shots[`${kind}-launch`]) throw new Error(`review look ${kind} missed its launch`);
      const flown = await follow(page, r, lead);
      if (flown) {
        const flight = await frameRound(page, kind, flown);
        // Tracer chance belongs to the look. Wait for an actual visible sample.
        if (!flight.visible) continue;
        shots[kind] = { ...flight, blast: w.blast_radius_m > 0 };
      }
      let landed;
      if (flown) {
        landed = await flyOut(page, flown);
        const end = landed;
        if (end.hit !== "none") {
          shots[`${kind}-impact`] = await frameRound(page, `${kind}-impact`, end);
          if (w.blast_radius_m > 0)
            shots[`${kind}-blast`] = await frameRound(page, `${kind}-blast`, end);
        }
      }
      if (flown && trails) {
        // Wide over the whole flight, from launch to the strike, its trail
        // still hanging a few seconds on.
        const end = landed.head;
        await advance(page, AFTER_S * TICK_HZ);
        const mid = r.from.map((v, i) => (v + end[i]) / 2);
        const span = Math.hypot(end[0] - r.from[0], end[1] - r.from[1]);
        const wide = Math.max(CAMERA.default.distance, span * 1.2);
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
  const busy = await scan(page, [], LAST, shots, seen, !process.env.ROUNDS_KINDS);
  await page.close();
  // Every look uses the same review lane so changing street visibility or
  // a missing weapon cannot leave a stage hidden in fog or foliage.
  // Only appearance changes: every look gets the same controlled flash,
  // flight, contact and blast causes, through the real battle/publication path.
  const sources = {};
  for (const kind of KINDS) {
    const review = {
      ...ctx,
      newPage: async (options) => {
        const p = await ctx.newPage(options);
        p.reviewLook = kind;
        await p.route(`**${MECHANICS_API}`, async (route) => {
          const response = await route.fetch();
          const snapshot = await response.json();
          const rules = snapshot.documents.find((d) => d.path === "fixtures/rules.json").value;
          // Development startup installs the authored rules document.
          for (const id of ["tank_ap", "tank_he"]) rules.weapons[id].appearance = kind;
          await route.fulfill({ response, json: snapshot });
        });
        return p;
      },
    };
    const alt = await openPaused(review, new URL("/labs/projectiles", ctx.url).href);
    alt.reviewLook = kind;
    await advance(alt, START - (await tick(alt)));
    await lab(alt, () => window.__lab.setFrameView("world"));
    await lab(alt, () => window.__lab.suppressPaint(true));
    await scan(alt, [kind], 60 * TICK_HZ, shots, seen, false);
    await alt.close();
    if (shots[kind]) sources[kind] = "/labs/projectiles (controlled HE lane)";
  }
  const missing = KINDS.filter((k) => !shots[k]);
  ctx.check(
    "every round look is framed in flight, and a busy moment",
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
  const crop = (src, marks, zoom, band = false) => {
    const height = band ? 50 : H;
    const [h, t] = marks;
    const c = h && t ? [(h[0] + t[0]) / 2, (h[1] + t[1]) / 2] : [960, 540];
    const x = Math.min(Math.max(c[0] * zoom - W / 2, 0), 1920 * zoom - W);
    const y = Math.min(Math.max(c[1] * zoom - height / 2, 0), 1080 * zoom - height);
    const style = `background-image:url(${src});background-size:${1920 * zoom}px ${1080 * zoom}px;background-position:${-x}px ${-y}px`;
    return band
      ? `<div class="crop"><div class="trail" style="height:${height}px;margin-top:${(H - height) / 2}px;${style}"></div></div>`
      : `<div class="crop" style="${style}"></div>`;
  };
  const last = sets.at(-1).record;
  let rows = "";
  for (const key of last.rows) {
    const shot = last.shots[key];
    const from = shot && last.sources[shot.kind];
    const far = shot && shot.default.distance !== CAMERA.default.distance;
    const note = [from, far && `${Math.round(shot.default.distance)} m`].filter(Boolean);
    const label = shot && key === shot.kind ? `${key} — flight` : key;
    let cells = `<div class="kind">${label}${note.length ? `<small>(${note.join(", ")})</small>` : ""}</div>`;
    for (const s of sets) {
      const shot = s.record.shots[key];
      if (!shot) {
        const absent =
          key.endsWith("-blast") && s.record.shots[key.slice(0, -6)]?.blast === false
            ? "no blast in this weapon"
            : "not shot";
        cells += `<div class="none">${absent}</div><div class="none">${absent}</div>`;
        continue;
      }
      // A wide shot whole; the player's view at 1:1 round the round.
      const zoom =
        shot.default.distance === CAMERA.default.distance && !/-(impact|blast)$/.test(key)
          ? 1
          : W / 1920;
      cells += crop(png(s.dir, `${key}-default.png`), shot.default.page, zoom);
      const whole = /-(launch|impact|blast|after)$/.test(key);
      const aftermath = key.endsWith("-after");
      cells += crop(
        png(s.dir, `${key}-close.png`),
        shot.close.page,
        aftermath ? W / 1600 : whole ? W / 1920 : 1,
        aftermath,
      );
    }
    rows += `<div class="row">${cells}</div>`;
  }
  const heads = sets
    .map(
      (s) =>
        `<div class="head">${s.label}: default camera (65 m; launch/flight crop, contacts whole)</div><div class="head">${s.label}: detail (flight crop; full contact; aftermath trail band)</div>`,
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
    p { width: 1320px; font-size: 16px; line-height: 1.5; }
    .row, .heads { display: grid; grid-template-columns: 240px repeat(${cols}, ${W}px); gap: 6px; margin-bottom: 6px; }
    .kind { display: flex; flex-direction: column; justify-content: center; font-size: 16px; overflow-wrap: anywhere; } .kind small { font-size: 12px; color: #999; }
    .head { font-size: 15px; color: #bbb; }
    .trail { background-repeat: no-repeat; }
    .crop, .none { width: ${W}px; height: ${H}px; background-repeat: no-repeat; outline: 1px solid #333; }
    .none { display: flex; align-items: center; justify-content: center; color: #777; }
    figure { margin: 12px 0 0; } figure img { width: ${240 + cols * (W + 6)}px; display: block; }
  </style><p>Each name identifies a presentation look. The tank carries that look on a controlled HE round: these are style samples, not the named weapon’s trajectory or blast radius. Aftermath rows cover looks with authored tracer smoke.</p><div class="heads"><div></div>${heads}</div>${rows}${busy}</body></html>`,
  );
  await page.goto(html.href);
  // Very tall Chromium captures repeat composited sections. Scroll a bounded
  // viewport and stitch its actual pixels instead of resizing the GPU surface.
  const { width, height } = await page.evaluate(() => ({
    width: innerWidth,
    height: document.documentElement.scrollHeight,
  }));
  const stitched = new PNG({ width, height });
  const viewport = page.viewportSize().height;
  for (let y = 0; y < height; y += viewport) {
    const top = await page.evaluate(async (y) => {
      scrollTo(0, y);
      await new Promise(requestAnimationFrame);
      return scrollY;
    }, y);
    const png = decode(await capture(page));
    PNG.bitblt(png, stitched, 0, y - top, width, Math.min(viewport, height - y), 0, y);
  }
  await writeFile(file, PNG.sync.write(stitched));
}
