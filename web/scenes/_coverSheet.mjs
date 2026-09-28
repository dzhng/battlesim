// Opt-in (`WATCH_TOURS=cover`): how cover shows with Space held, in the
// watched village battle. The battle runs on from BATTLE_TICK; each own squad
// settled in one tier of cover (most of its soldiers' "cover now" in it), one
// in the open, and one moving with "cover there" at its destination spots are
// framed as they first appear: the default camera and a close-up, captioned.
// Writes `cover-<case>-<frame>.png`.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, snapshot } from "./_lab.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
const COVER = village.presentation.overlay.orders.cover;
const START = Number(process.env.BATTLE_TICK ?? 8100);
const COLOUR_NAME = { light: "yellow (the orders')", medium: "light green", heavy: "strong green" };

const pose = (page, at, distance, pitch) =>
  lab(
    page,
    (c) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        distance: c.distance,
        pitch: c.pitch,
        yaw: c.yaw,
        target: [c.at[0], c.at[1], window.__lab.route.surfaceZ(c.at[0], c.at[1])],
      }),
    { at, distance, pitch, yaw: CAMERA.default.yaw },
  );

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
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, START - (await lab(page, () => window.__lab.route.tick())));
  await page.keyboard.down("Space");
  await page.waitForFunction(() => window.__lab.route.showOrders(), undefined, { timeout: 5000 });
  // `COVER_CASES=light,medium` looks for only those.
  const wanted = new Set(
    process.env.COVER_CASES?.split(",") ?? ["light", "medium", "heavy", "open", "moving"],
  );
  const found = {};
  for (let step = 0; step < 80 && wanted.size; step++) {
    const o = await obs(page);
    const squads = o.own.filter((u) => u.members.length > 0 && !u.garrison);
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
        await pose(page, at, distance, pitch);
        await caption(page, `${words} · ${frame}`);
        await lab(page, () => window.__lab.frame());
        await lab(page, () => window.__lab.frame());
        await snapshot(ctx, page, `cover-${key}-${frame}.png`);
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
  await page.close();
}
