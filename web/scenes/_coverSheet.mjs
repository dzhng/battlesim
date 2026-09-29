// Opt-in (`WATCH_TOURS=cover`): how cover shows with Space held, in the
// watched village battle. From BATTLE_TICK the battle steps on, half a second
// at a time and within SEARCH_TICKS, until its own state has shown each case:
// an own squad holding with soldiers in a tier of cover, one holding in the
// open, one moving with "cover there" at its destination spots, and one
// holding while its soldiers walk to their posts. Each is framed the tick it
// first appears (the default camera and a close-up, captioned); a case the
// battle never reaches fails the sheet. Order marks and cover pips are
// ground paint, so the checks read the frame's paint view.
// Writes `cover-<case>-<frame>.png` and `cover-sheet.json`.
import {
  lab,
  obs,
  advance,
  snapshot,
  openBattle,
  aim,
  groundCss,
  pointerOffCanvas,
} from "./_lab.mjs";
import { orderPaint, paintHue } from "./_overlays.mjs";
import { village } from "./_units.mjs";

const CAMERA = village.presentation.camera;
const COVER = village.presentation.overlay.orders.cover;
const START = Number(process.env.BATTLE_TICK ?? 8100);
/** How far past START the cases are looked for (five minutes), and the
 *  step: a squad may hold in cover for only a second or two. */
const SEARCH_TICKS = 9000;
const STEP = 15;
const TIERS = ["heavy", "medium", "light"];
/** Without `COVER_CASES`, the cases the sheet needs; besides these, a squad
 *  holding in medium or heavy cover (the tiers whose pips the centring check
 *  can tell from the orders' yellow). Light cover is framed if the battle
 *  shows it on the way. */
const NEEDED = ["open", "moving", "posts"];
const COLOUR_NAME = { light: "yellow (the orders')", medium: "light green", heavy: "strong green" };

const caption = (page, text) =>
  page.evaluate((t) => {
    let tag = document.getElementById("cover-tag");
    if (!tag) {
      tag = document.createElement("div");
      tag.id = "cover-tag";
      tag.style.cssText =
        "position:fixed;z-index:99;left:24px;top:60px;font:700 30px ui-monospace,monospace;" +
        "color:#fff;background:rgb(0 0 0 / 0.65);padding:6px 14px";
      document.body.append(tag);
    }
    tag.textContent = t;
  }, text);

const hideCaption = (page) => page.evaluate(() => document.getElementById("cover-tag")?.remove());

/** A squad holding in the open: most of its soldiers hold no cover. */
function holdsInOpen(u) {
  const open = u.memberOrders.filter((m) => !m.coverNow).length;
  return open >= Math.max(2, Math.ceil(u.members.length * 0.6));
}

/** The soldiers of `u` holding cover of `tier`. */
const inTier = (u, tier) =>
  u.memberOrders.flatMap((m, k) => (m.coverNow === tier && u.members[k] ? [u.members[k]] : []));

const centroid = (pts) => [0, 1].map((i) => pts.reduce((a, p) => a + p[i], 0) / pts.length);

export async function coverTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: START });
  // Off the canvas: with Space held the range ruler (paint too) stays away.
  await pointerOffCanvas(page);
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders(), undefined, { timeout: 5000 });
  // `COVER_CASES=light,medium` looks for only those, and needs each.
  const only = process.env.COVER_CASES?.split(",");
  const wanted = new Set(only ?? [...TIERS, ...NEEDED]);
  const found = {};
  const missing = () =>
    only
      ? only.filter((c) => !found[c])
      : [
          ...NEEDED.filter((c) => !found[c]),
          ...(found.medium || found.heavy ? [] : ["medium or heavy"]),
        ];
  // A holding squad's soldiers walking to their posts: no line from a
  // soldier (the order is the unit's).
  let posts = null;
  const centring = [];
  let o = await obs(page);
  for (;;) {
    const squads = o.own.filter((u) => u.members.length > 0 && !u.garrison);
    if (wanted.has("posts")) {
      const walking = squads.find(
        (u) =>
          !u.goal &&
          u.memberOrders.filter((m, k) => u.members[k] && legOff(u.members[k], m.spot)).length >= 2,
      );
      if (walking) {
        wanted.delete("posts");
        found.posts = { tick: o.tick, unit: walking.id, at: walking.position };
        posts = await legsFromSoldiers(ctx, page, walking);
      }
    }
    for (const u of squads) {
      // The case this squad shows, and where to frame it.
      let key = null;
      let at = u.position;
      if (u.goal) {
        if (
          wanted.has("moving") &&
          u.memberOrders.some((m) => m.coverThere) &&
          u.memberOrders.some((m) => m.coverNow)
        ) {
          key = "moving";
          at = centroid([u.position, u.goal]);
        }
      } else {
        // Holding: two soldiers in one tier will do, framed on them.
        const tier = TIERS.find((t) => wanted.has(t) && inTier(u, t).length >= 2);
        if (tier) {
          key = tier;
          at = centroid(inTier(u, tier));
        } else if (wanted.has("open") && holdsInOpen(u)) key = "open";
      }
      if (!key) continue;
      wanted.delete(key);
      found[key] = { tick: o.tick, unit: u.id, at: u.position };
      const words =
        key === "moving"
          ? "moving: cover now at the soldiers, cover there at the spots"
          : key === "open"
            ? "in the open: no cover, no pip"
            : `${key} cover: ${COLOUR_NAME[key]} pip [${COVER[key].slice(0, 3).join(", ")}]`;
      // The moving squad's close-up fits both its soldiers (cover now) and
      // its destination spots (cover there).
      const span =
        key === "moving" ? Math.hypot(u.goal[0] - u.position[0], u.goal[1] - u.position[1]) : 0;
      for (const [frame, distance, pitch] of [
        ["default", CAMERA.default.distance, 0.85],
        ["close", Math.max(25, Math.min(span * 1.1, 80)), 0.7],
      ]) {
        await aim(page, at, { distance, pitch, yaw: CAMERA.default.yaw });
        await caption(page, `${words} · ${frame}`);
        await lab(page, () => window.__lab.frame());
        await lab(page, () => window.__lab.frame());
        await snapshot(ctx, page, `cover-${key}-${frame}.png`);
        if (frame === "close" && (key === "heavy" || key === "medium"))
          centring.push(...(await pipsCentred(ctx, page, u, key)));
      }
    }
    if (missing().length === 0 || o.tick - START >= SEARCH_TICKS) break;
    await advance(page, STEP);
    o = await obs(page);
  }
  await page.keyboard.up("Space");
  const sheet = { from: START, to: o.tick, found, missing: missing() };
  await ctx.writeEvidence("cover-sheet.json", sheet);
  ctx.check(
    `from tick ${START}, the battle shows each cover case within ${SEARCH_TICKS} ticks`,
    sheet.missing.length === 0,
    JSON.stringify(sheet),
  );
  if (!only || only.includes("posts"))
    ctx.check(
      "with Space, no line runs from a holding squad's soldier to his post (sampled)",
      !!posts &&
        posts.checked > 0 &&
        posts.posts === posts.checked &&
        posts.inked <= Math.floor(posts.checked / 10),
      JSON.stringify(posts),
    );
  if (!only || only.some((c) => c === "medium" || c === "heavy"))
    ctx.check(
      "close up, a soldier's cover pip sits at his marker's centre, within a pixel",
      centring.length > 0 && centring.every((c) => c.dpx <= 1),
      JSON.stringify(centring),
    );
  await page.close();
}

/** Close up, in the paint: each whole pip of `tier` round a soldier of `u`
 *  holding it, its centroid's distance in pixels from the soldier's own
 *  point, where his marker circle is drawn. */
async function pipsCentred(ctx, page, u, tier) {
  await hideCaption(page);
  const png = await orderPaint(ctx, page, `cover-${tier}-close`);
  const out = [];
  for (const q of inTier(u, tier)) {
    const p = await groundCss(page, [q[0], q[1]]);
    const edge = await groundCss(page, [q[0] + 0.45, q[1]]);
    if (!p || !edge) continue;
    const R = Math.hypot(edge[0] - p[0], edge[1] - p[1]);
    const sum = [0, 0, 0];
    const reach = Math.ceil(R);
    for (let dy = -reach; dy <= reach; dy++)
      for (let dx = -reach; dx <= reach; dx++) {
        const [x, y] = [Math.round(p[0]) + dx, Math.round(p[1]) + dy];
        if (x < 0 || y < 0 || x >= png.width || y >= png.height) continue;
        if (Math.hypot(x - p[0], y - p[1]) > R * 0.8) continue;
        const i = (y * png.width + x) * 4;
        if (paintHue([png.data[i], png.data[i + 1], png.data[i + 2]]) !== tier) continue;
        sum[0] += x;
        sum[1] += y;
        sum[2]++;
      }
    // Only a whole pip (not one a route or another mark covers part of).
    if (sum[2] < Math.PI * (R * (0.3 / 0.45)) ** 2 * 0.5) continue;
    out.push({
      dpx: +Math.hypot(sum[0] / sum[2] - p[0], sum[1] / sum[2] - p[1]).toFixed(2),
      R: +R.toFixed(1),
      n: sum[2],
    });
  }
  return out;
}

/** A soldier walking to a post near enough to frame with him: 3 to 25 m. */
const legOff = (q, spot) => {
  const d = Math.hypot(spot[0] - q[0], spot[1] - q[1]);
  return d >= 3 && d <= 25;
};

/** Framed on a holding squad whose soldiers walk to their posts, in the
 *  paint: half way from each soldier to his post (3 m or more off), how many
 *  of the samples hold the orders' yellow (a line from the soldier would). */
async function legsFromSoldiers(ctx, page, u) {
  // Framed on the soldiers and their posts together, and drawn before any
  // projection (the camera moves on the next frame).
  const pts = u.memberOrders.flatMap((m, k) =>
    u.members[k] && legOff(u.members[k], m.spot) ? [m.spot, u.members[k]] : [],
  );
  await aim(page, centroid(pts), { distance: 45, pitch: 0.85, yaw: CAMERA.default.yaw });
  await lab(page, () => window.__lab.frame());
  await lab(page, () => window.__lab.frame());
  await hideCaption(page);
  const png = await orderPaint(ctx, page, "cover-posts");
  const inFrame = (p) => p && p[0] >= 16 && p[1] >= 48 && p[0] <= 1904 && p[1] <= 900;
  const inkAt = ([x, y]) => {
    const i = (Math.round(y) * png.width + Math.round(x)) * 4;
    return paintHue([png.data[i], png.data[i + 1], png.data[i + 2]]) === "yellow";
  };
  // `posts` counts the post markers in frame that show ink within 12 px: a
  // positive control, so an empty frame can't pass.
  const out = { unit: u.id, checked: 0, inked: 0, posts: 0 };
  for (const [k, m] of u.memberOrders.entries()) {
    const q = u.members[k];
    if (!q || !legOff(q, m.spot)) continue;
    const p = await groundCss(page, [(q[0] + m.spot[0]) / 2, (q[1] + m.spot[1]) / 2]);
    const s = await groundCss(page, m.spot);
    if (!inFrame(p) || !inFrame(s)) continue;
    out.checked++;
    if (inkAt(p)) out.inked++;
    let seen = false;
    for (let dy = -12; dy <= 12 && !seen; dy++)
      for (let dx = -12; dx <= 12 && !seen; dx++) seen = inkAt([s[0] + dx, s[1] + dy]);
    if (seen) out.posts++;
  }
  return out;
}
