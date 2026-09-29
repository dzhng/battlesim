// Opt-in (`WATCH_TOURS=cover`): how cover shows with Space held, in the
// watched village battle. The battle runs on from BATTLE_TICK; each own squad
// settled in one tier of cover (most of its soldiers' "cover now" in it), one
// in the open, and one moving with "cover there" at its destination spots are
// framed as they first appear: the default camera and a close-up, captioned.
// Writes `cover-<case>-<frame>.png`.
import { lab, obs, advance, snapshot, openBattle, aim, groundCss } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { village } from "./_units.mjs";

const CAMERA = village.presentation.camera;
const COVER = village.presentation.overlay.orders.cover;
const START = Number(process.env.BATTLE_TICK ?? 8100);
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

/** The tier most of a squad's soldiers hold now, "open" when most hold none. */
function settledTier(u) {
  const count = { light: 0, medium: 0, heavy: 0, open: 0 };
  for (const m of u.memberOrders) count[m.coverNow ?? "open"]++;
  const [tier, n] = Object.entries(count).sort((a, b) => b[1] - a[1])[0];
  return n >= Math.max(2, Math.ceil(u.members.length * 0.6)) ? tier : null;
}

export async function coverTour(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, tick: START });
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders(), undefined, { timeout: 5000 });
  // `COVER_CASES=light,medium` looks for only those.
  const wanted = new Set(
    process.env.COVER_CASES?.split(",") ?? ["light", "medium", "heavy", "open", "moving", "posts"],
  );
  const found = {};
  // A holding squad's soldiers walking to their posts: no line from a
  // soldier (the order is the unit's). `COVER_CASES` may leave it out.
  let posts = null;
  const centring = [];
  for (let step = 0; step < 80 && wanted.size; step++) {
    const o = await obs(page);
    const squads = o.own.filter((u) => u.members.length > 0 && !u.garrison);
    if (wanted.has("posts")) {
      const walking = squads.find(
        (u) =>
          !u.goal &&
          u.memberOrders.filter((m, k) => u.members[k] && legOff(u.members[k], m.spot)).length >= 2,
      );
      if (walking) {
        wanted.delete("posts");
        posts = await legsFromSoldiers(ctx, page, walking);
      }
    }
    for (const u of squads) {
      let key = null;
      if (
        u.goal &&
        u.memberOrders.some((m) => m.coverThere) &&
        u.memberOrders.some((m) => m.coverNow)
      )
        key = "moving";
      else if (!u.goal) key = settledTier(u);
      // Light and medium cover are rarer than a whole squad's: two soldiers
      // in it will do, framed on them.
      let focus = null;
      if (!u.goal && (!key || !wanted.has(key)))
        for (const tier of ["light", "medium"]) {
          const holding = u.memberOrders
            .map((m, k) => (m.coverNow === tier ? u.members[k] : null))
            .filter(Boolean);
          if (wanted.has(tier) && holding.length >= 2) {
            key = tier;
            focus = [
              holding.reduce((a, p) => a + p[0], 0) / holding.length,
              holding.reduce((a, p) => a + p[1], 0) / holding.length,
            ];
            break;
          }
        }
      if (!key || !wanted.has(key)) continue;
      wanted.delete(key);
      found[key] = { tick: o.tick, unit: u.id, at: u.position };
      const words =
        key === "moving"
          ? "moving: cover now at the soldiers, cover there at the spots"
          : key === "open"
            ? "in the open: no cover, no pip"
            : `${key} cover: ${COLOUR_NAME[key]} pip [${COVER[key].slice(0, 3).join(", ")}]`;
      const at = focus
        ? focus
        : key === "moving" && u.goal
          ? [(u.position[0] + u.goal[0]) / 2, (u.position[1] + u.goal[1]) / 2]
          : u.position;
      // The moving squad's close-up fits both its soldiers (cover now) and
      // its destination spots (cover there).
      const span =
        key === "moving" && u.goal
          ? Math.hypot(u.goal[0] - u.position[0], u.goal[1] - u.position[1])
          : 0;
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
    if (wanted.size) await advance(page, 90);
  }
  await page.keyboard.up("Space");
  await ctx.writeEvidence("cover-sheet.json", { found, missing: [...wanted] });
  ctx.check(
    "the cover sheet found its cases",
    true,
    JSON.stringify({ found, missing: [...wanted] }),
  );
  if (!process.env.COVER_CASES || process.env.COVER_CASES.includes("posts"))
    ctx.check(
      "with Space, no line runs from a holding squad's soldier to his post (sampled)",
      !!posts &&
        posts.checked > 0 &&
        posts.posts === posts.checked &&
        posts.inked <= Math.floor(posts.checked / 10),
      JSON.stringify(posts),
    );
  if (!process.env.COVER_CASES || /medium|heavy/.test(process.env.COVER_CASES))
    ctx.check(
      "close up, a soldier's cover pip sits at his marker's centre, within a pixel",
      centring.length > 0 && centring.every((c) => c.dpx <= 1),
      JSON.stringify(centring),
    );
  await page.close();
}

/** Close up, over black: each whole pip of `tier` round a soldier of `u`
 *  holding it, its centroid's distance in pixels from the soldier's own
 *  point, where his marker circle is drawn. */
async function pipsCentred(ctx, page, u, tier) {
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "hidden";
    document.getElementById("cover-tag")?.remove();
  });
  // The soldiers held off, so no body stands over the pip it is drawn under.
  await lab(page, () => window.__lab.suppressModels(true));
  await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
  const png = decode(await snapshot(ctx, page, `cover-${tier}-close-overlay.png`));
  await lab(page, () => window.__lab.setFrameView("final"));
  await lab(page, () => window.__lab.suppressModels(false));
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "";
  });
  // The pip's green (its tier is known here): well over red and blue.
  const green = ([r, g, b]) => g > r + 40 && g > b + 30;
  const out = [];
  for (const [k, m] of u.memberOrders.entries()) {
    if (m.coverNow !== tier || !u.members[k]) continue;
    const q = u.members[k];
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
        if (!green([png.data[i], png.data[i + 1], png.data[i + 2]])) continue;
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

/** Framed on a holding squad whose soldiers walk to their posts, over black:
 *  half way from each soldier to his post (3 m or more off), how many of the
 *  samples hold the orders' yellow (a line from the soldier would). */
async function legsFromSoldiers(ctx, page, u) {
  // Framed on the soldiers and their posts together, and drawn before any
  // projection (the camera moves on the next frame).
  const pts = u.memberOrders.flatMap((m, k) =>
    u.members[k] && legOff(u.members[k], m.spot) ? [m.spot, u.members[k]] : [],
  );
  const at = [0, 1].map((i) => pts.reduce((a, p) => a + p[i], 0) / pts.length);
  await aim(page, at, { distance: 45, pitch: 0.85, yaw: CAMERA.default.yaw });
  await lab(page, () => window.__lab.frame());
  await lab(page, () => window.__lab.frame());
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "hidden";
    document.getElementById("cover-tag")?.remove();
  });
  await lab(page, () => window.__lab.suppressModels(true));
  await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
  const png = decode(await snapshot(ctx, page, "cover-posts-overlay.png"));
  await lab(page, () => window.__lab.setFrameView("final"));
  await lab(page, () => window.__lab.suppressModels(false));
  await page.evaluate(() => {
    document.querySelector("[data-testid=readouts]").style.visibility = "";
  });
  const yellow = ([r, g, b]) => r > 60 && g > 0.7 * r && b < 0.6 * r && r > g * 0.9;
  const inFrame = (p) => p && p[0] >= 16 && p[1] >= 48 && p[0] <= 1904 && p[1] <= 900;
  const inkAt = ([x, y]) => {
    const i = (Math.round(y) * png.width + Math.round(x)) * 4;
    return yellow([png.data[i], png.data[i + 1], png.data[i + 2]]);
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
