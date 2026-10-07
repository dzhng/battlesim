// Slice 10: supported AT guidance, release and escape, through real orders.
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, until, snapshot } from "./_lab.mjs";

/** Switch variant (a fresh battle), paused at its start. */
async function begin(page, variant) {
  await lab(page, (v) => window.__lab.route.variant(v), variant);
  await page.waitForFunction(() => window.__lab.route?.reset);
  await lab(page, () => window.__lab.route.reset());
  await page.waitForFunction(
    () => window.__lab.route?.tick() > 3 && window.__lab.route.tick() < 60,
    undefined,
    {
      timeout: 20000,
    },
  );
  await lab(page, () => window.__lab.route.pause());
}

/** Red's first tank's health, read through the lab's diagnostic red view. */
async function redTankHp(page) {
  await lab(page, () => window.__lab.route.observeAs("red"));
  const o = await until(page, (f) => f.own.some((u) => u.kind === "test_tank"), 30, 1);
  const hp = o?.own.find((u) => u.id === 0)?.hp ?? 0;
  await lab(page, () => window.__lab.route.observeAs("blue"));
  await advance(page, 1);
  return hp;
}

/** A close look at the cover and the tank, then back to the lab's framing. */
async function closeUp(ctx, page, name) {
  const saved = await lab(page, () => window.__lab.camera());
  await lab(
    page,
    (c) => window.__lab.setCamera({ ...c, target: [585, 275, 0], distance: 130, yaw: -1.9 }),
    saved,
  );
  await snapshot(ctx, page, `closeup-${name}-1280x800.png`);
  await lab(page, (c) => window.__lab.setCamera(c), saved);
}

async function frame(ctx, page, name, focus) {
  const shot = await snapshot(ctx, page, `frame-${name}-1280x800.png`);
  if (focus) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), focus.at);
    await writeCrop(
      decode(shot),
      ctx.evidencePath(`crop-${name}-2x.png`),
      at[0],
      at[1],
      focus.half[0],
      focus.half[1],
      2,
    );
  }
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });

  // Prompt escape: launch on own sight, release when sight is lost.
  await begin(page, "prompt");
  let o = await until(page, (f) => f.guided.length > 0, 300, 3);
  const launcher = o?.own.find((u) => u.id === 2);
  ctx.check(
    "the AT team launches on its own sight and guides the missile",
    !!o && o.guided[0].supported && launcher.mounts.some((m) => m.guiding),
    JSON.stringify(o?.guided),
  );
  await frame(ctx, page, "guided", { at: [330, 290], half: [300, 90] });
  o = await until(page, (f) => f.guided.length > 0 && !f.guided[0].supported, 120, 1);
  const released = o?.guided[0];
  ctx.check(
    "losing sight releases the missile to a fixed point on the ground",
    !!released && Math.abs(released.point[2]) < 0.01,
    JSON.stringify(released),
  );
  await frame(ctx, page, "released");
  await closeUp(ctx, page, "released");
  let fixed = true;
  while ((o = await obs(page)).guided.length > 0) {
    fixed &&= o.guided[0].point.join() === released.point.join() && !o.guided[0].supported;
    await advance(page, 1);
  }
  ctx.check("the released point never moves and support never returns", fixed);
  await frame(ctx, page, "prompt-outcome");
  await closeUp(ctx, page, "prompt-outcome");
  const prompt = await redTankHp(page);
  ctx.check("a prompt escape out of sight is not hit", prompt === 100, `hp ${prompt}`);
  o = await until(
    page,
    (f) => f.own.find((u) => u.id === 2)?.mounts[1].reason === "no_own_sight",
    300,
    3,
  );
  ctx.check(
    "a target only the scout sees cannot be engaged",
    !!o && o.identified.length > 0,
    JSON.stringify(o?.own.find((u) => u.id === 2)?.mounts[1]),
  );
  await frame(ctx, page, "scout-only");

  // Late escape: hit.
  await begin(page, "late");
  await until(page, (f) => f.guided.length > 0, 300, 3);
  await until(page, (f) => f.guided.length === 0, 300, 3);
  await frame(ctx, page, "late-outcome");
  await closeUp(ctx, page, "late-outcome");
  const late = await redTankHp(page);
  ctx.check("a late escape is hit", late < 100, `hp ${late}`);

  // Moving the launcher releases at once and frees the crew.
  await begin(page, "late");
  await until(page, (f) => f.guided.length > 0, 300, 3);
  await lab(page, () => window.__lab.route.moveLauncher());
  o = await until(page, (f) => f.guided.length > 0 && !f.guided[0].supported, 10, 1);
  const before = o?.own.find((u) => u.id === 2)?.position;
  await advance(page, 15);
  const after = (await obs(page)).own.find((u) => u.id === 2)?.position;
  await frame(ctx, page, "launcher-moved", { at: [60, 345], half: [90, 60] });
  ctx.check(
    "moving releases support at once and the crew moves on",
    !!o && before && after && before.join() !== after.join(),
    JSON.stringify({ before, after }),
  );

  // Prepared crossfire: the second team still sees the tank behind cover.
  await begin(page, "crossfire");
  o = await until(page, (f) => f.guided.length === 2, 300, 3);
  ctx.check("both teams launch", !!o, JSON.stringify(o?.guided));
  await advance(page, 30);
  await frame(ctx, page, "crossfire-in-flight");
  await until(page, (f) => f.guided.length === 0, 300, 3);
  const crossfire = await redTankHp(page);
  ctx.check("a prompt escape does not beat a crossfire", crossfire < 100, `hp ${crossfire}`);
  await frame(ctx, page, "crossfire-outcome");
}
