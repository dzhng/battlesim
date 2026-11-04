// A 2v2 team battle from its link: you and one AI ally against two AI.
// The browser-only evidence for allies in battle (team skirmish):
// - the ally look at default and far zoom, over grass, road, fog and smoke;
// - selecting an ally opens its panel read-only, with its owner line and no
//   order controls, and an order given then carries none of its units;
// - a box drawn over your units and your ally's selects only yours.
// The input rule itself (`controlled`) is tested in web/tests/unitControl.test.tsx.
import { writeFile } from "node:fs/promises";
import { advance, aim, groundCss, lab, obs, presented, snapshot, until } from "./_lab.mjs";

/** The viewing player's index in the link's player list (blue=h,a: you are
 *  blue-1, your ally blue-2). */
const YOU = 0;
const ALLY = 1;
/** What you buy: two tanks, so both armies field vehicles. */
const FAMILY = "abrams";
const BUY = 2;

const dist = (a, b) => Math.hypot(a.position[0] - b.position[0], a.position[1] - b.position[1]);

/** A frame of the battle round `at`, `distance` out, saved as `name`.png. */
async function shoot(ctx, page, name, at, distance) {
  await aim(page, at, { distance });
  await presented(page);
  await snapshot(ctx, page, `${name}.png`);
}

const centroid = (units) =>
  units
    .reduce((s, u) => [s[0] + u.position[0], s[1] + u.position[1]], [0, 0])
    .map((v) => v / Math.max(1, units.length));

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page, ctx.url, 120000);
  await page.addStyleTag({ content: ".frame-rate { display: none !important; }" });
  await lab(page, () => window.__lab.route.pause());
  await presented(page);
  const first = await obs(page);
  const players = first.skirmish?.players ?? [];
  ctx.check(
    "the link fields you and one AI ally on blue",
    players.length === 2 &&
      players[YOU]?.controller === "human" &&
      players[ALLY]?.controller === "active_ai",
    JSON.stringify(players),
  );

  // Buy at your entry through the picker's own keyboard path (a family
  // name, then Enter arms its first variant's ghost), then ready.
  const entry = await lab(page, () => window.__lab.route.prepared().start.at);
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  const search = page.getByRole("textbox", { name: "Search units" });
  await search.fill(FAMILY);
  await search.press("Enter");
  // The ghost stays armed after a placement: each click places another.
  for (let k = 0; k < BUY; k++) {
    const at = await groundCss(page, [entry[0] + 60 * k, entry[1]]);
    await page.mouse.move(at[0], at[1]);
    await page
      .waitForFunction(() => window.__lab.route.purchaseGhost?.()?.valid === true, undefined, {
        timeout: 5000,
      })
      .catch(() => {});
    await page.mouse.click(at[0], at[1]);
    await advance(page, 1);
  }
  // A right-click puts the ghost away.
  await page.mouse.click(40, 300, { button: "right" });
  const bought = [(await obs(page)).skirmish.players[YOU].pending.length];
  ctx.check("you placed reinforcements", bought.at(-1) > 0, JSON.stringify(bought));
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();

  // Both blue players' units on the field, your ally's force growing.
  const owners = (o) => new Set(o.own.map((u) => u.owner));
  let o = await until(
    page,
    (o) =>
      o.skirmish.phase === "active" &&
      o.own.some((u) => u.owner === YOU) &&
      o.own.some((u) => u.owner === ALLY),
    150 * 30,
    150,
  );
  ctx.check(
    "your units and your ally's are on the field",
    !!o && owners(o).has(YOU) && owners(o).has(ALLY),
    JSON.stringify((await obs(page)).own.map((u) => [u.id, u.owner, u.kind])),
  );
  if (!o) return;
  // Your tanks drive over beside your ally's force, so one frame holds both.
  const mineIds = o.own.filter((u) => u.owner === YOU).map((u) => u.id);
  const near = (o) => {
    const allies = o.own.filter((u) => u.owner === ALLY);
    const mine = o.own.filter((u) => u.owner === YOU);
    return mine.length > 0 && mine.every((m) => allies.some((a) => dist(a, m) < 70));
  };
  for (let leg = 0; leg < 6 && !near(o); leg++) {
    const goal = centroid(o.own.filter((u) => u.owner === ALLY));
    await lab(page, (order) => window.__lab.route.command(order), {
      kind: "move",
      units: mineIds,
      gesture: 100 + leg,
      goal: [goal[0] + 25, goal[1] - 25],
      route: "fastest",
    });
    o = (await until(page, near, 20 * 30, 150)) ?? (await obs(page));
  }
  ctx.check("your tanks reach your ally's force", near(o));
  await ctx.writeEvidence(
    "units.json",
    o.own.map((u) => [u.id, u.owner, u.kind, u.position]),
  );
  const mine = o.own.filter((u) => u.owner === YOU);
  const allies = o.own.filter((u) => u.owner === ALLY);

  // The ally look: a frame holding both armies, at default and far zoom.
  const both = centroid([...mine, ...allies.filter((a) => mine.some((m) => dist(a, m) < 90))]);
  await shoot(ctx, page, "ally-default", both, 160);
  await shoot(ctx, page, "ally-far", both, 900);
  // Over each ground the battle takes them across, and near smoke (a wreck
  // blue knows burns), as they come: road, open ground, and fog's edge.
  const shot = new Set();
  for (let step = 0; step < 18 && shot.size < 3; step++) {
    const now = await obs(page);
    const ally = now.own.filter((u) => u.owner === ALLY);
    for (const a of ally) {
      const ground = await lab(page, ([x, y]) => window.__lab.route.surfaceAt(x, y), a.position);
      const name = !ground
        ? null
        : /road|track|paving/.test(ground.kind)
          ? "ally-road"
          : ground.forest
            ? null
            : "ally-grass";
      if (name && !shot.has(name)) {
        shot.add(name);
        await shoot(ctx, page, name, a.position, 140);
      }
    }
    const wreck = now.knownProps.find(
      (p) =>
        p.wreckOf &&
        ally.some(
          (a) => Math.hypot(a.position[0] - p.center[0], a.position[1] - p.center[1]) < 150,
        ),
    );
    if (wreck && !shot.has("ally-smoke")) {
      shot.add("ally-smoke");
      await shoot(ctx, page, "ally-smoke", wreck.center, 160);
    }
    await advance(page, 5 * 30);
  }
  await ctx.writeEvidence("surfaces-shot.json", [...shot]);
  const allyAt = centroid((await obs(page)).own.filter((u) => u.owner === ALLY));
  await shoot(ctx, page, "ally-fog", allyAt, 1400);

  // Selecting an ally: click its card.
  o = await obs(page);
  const ally =
    o.own.find((u) => u.owner === ALLY && u.id === allies[0].id) ??
    o.own.find((u) => u.owner === ALLY);
  await aim(page, ally.position, { distance: 180 });
  await presented(page);
  const card = page.locator(`[data-unit="${ally.id}"]`).first();
  await card.click();
  await presented(page);
  await snapshot(ctx, page, "ally-panel.png");
  const panelText = ((await card.textContent()) ?? "").replace(/\s+/g, " ");
  ctx.check("a selected ally's panel names its owner", /BLUE 2 · AI/.test(panelText), panelText);
  const enabled = await page
    .locator('[role="toolbar"][aria-label="Commands"] button:not([disabled])')
    .count();
  ctx.check(
    "with an ally selected, no order control is offered",
    enabled === 0,
    `${enabled} enabled`,
  );
  // A right-click on the ground then sends nothing that carries the ally.
  const before = (await lab(page, () => window.__lab.route.acks())).length;
  const ground = await lab(
    page,
    ([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)),
    [ally.position[0] + 25, ally.position[1]],
  );
  await page.mouse.click(ground[0], ground[1], { button: "right" });
  await advance(page, 2);
  const acks = await lab(page, () => window.__lab.route.acks());
  ctx.check(
    "no order leaves with an ally's unit",
    acks.length === before &&
      acks.every(({ order }) => !(order.units ?? []).some((id) => allies.some((a) => a.id === id))),
    JSON.stringify(acks.map(({ order }) => order)),
  );

  // A box over both armies selects only yours.
  await lab(page, () => window.__lab.route.select([]));
  o = await obs(page);
  // Your unit and your ally's nearest it, framed together and small enough
  // that the padded box stays clear of the HUD's bars: a drag released on
  // a panel selects nothing.
  const yours = o.own.filter((u) => u.owner === YOU);
  const theirs = o.own.filter((u) => u.owner === ALLY);
  const pair = yours
    .flatMap((m) => theirs.map((a) => [m, a]))
    .sort((p, q) => dist(...p) - dist(...q))[0];
  ctx.check("your units and your ally's are both on the field for the box", !!pair);
  if (!pair) return;
  const together = pair;
  await aim(page, centroid(together), { distance: Math.max(220, 3 * dist(...pair)) });
  await presented(page);
  const box = await lab(
    page,
    (ids) =>
      ids
        .map((id) => window.__lab.route.observation().own.find((u) => u.id === id))
        .filter(Boolean)
        .map((u) => window.__lab.projectToCss(u.position[0], u.position[1], u.position[2]))
        .filter(Boolean),
    together.map((u) => u.id),
  );
  const xs = box.map((p) => p[0]),
    ys = box.map((p) => p[1]);
  const pad = 40;
  await page.mouse.move(Math.min(...xs) - pad, Math.min(...ys) - pad);
  await page.mouse.down();
  await page.mouse.move(Math.max(...xs) + pad, Math.max(...ys) + pad, { steps: 10 });
  await page.mouse.up();
  await presented(page);
  await snapshot(ctx, page, "box-select.png");
  const picked = await lab(page, () => window.__lab.route.selected());
  const ownerOf = new Map(o.own.map((u) => [u.id, u.owner]));
  ctx.check(
    "a box over your units and your ally's selects only yours",
    together.some((u) => u.owner === ALLY) &&
      picked.length > 0 &&
      picked.every((id) => ownerOf.get(id) === YOU),
    JSON.stringify({ picked, owners: picked.map((id) => ownerOf.get(id)) }),
  );
  // The order the box's selection then takes carries only your units.
  const last = await lab(page, () => Math.max(0, ...window.__lab.route.acks().map((a) => a.seq)));
  const goal = await groundCss(page, [pair[0].position[0] + 20, pair[0].position[1] - 20]);
  await page.mouse.click(goal[0], goal[1], { button: "right" });
  await page
    .waitForFunction((n) => window.__lab.route.acks().some((a) => a.seq > n), last, {
      timeout: 5000,
    })
    .catch(() => {});
  const orders = (await lab(page, () => window.__lab.route.acks())).filter((a) => a.seq > last);
  ctx.check(
    "the boxed selection's order carries only your units",
    orders.length > 0 &&
      orders.every(({ order }) => (order.units ?? []).every((id) => ownerOf.get(id) === YOU)),
    JSON.stringify(orders.map(({ order }) => order)),
  );
  await writeFile(ctx.evidencePath("selection.json"), JSON.stringify({ picked, mine, allies }));

  // Smoke: once blue knows a burning wreck near your ally's force.
  const smoking = (o) =>
    o.knownProps.find(
      (p) =>
        p.wreckOf &&
        o.own.some(
          (u) =>
            u.owner === ALLY &&
            Math.hypot(u.position[0] - p.center[0], u.position[1] - p.center[1]) < 300,
        ),
    );
  if (!shot.has("ally-smoke")) {
    // Optional, so its search is bounded: a minute of battle, not three.
    const burning = await until(page, smoking, 60 * 30, 300);
    if (burning) {
      // The wreck and your ally's unit nearest it, in one frame.
      const wreck = smoking(burning).center;
      const near = (u) => Math.hypot(u.position[0] - wreck[0], u.position[1] - wreck[1]);
      const ally = burning.own.filter((u) => u.owner === ALLY).sort((a, b) => near(a) - near(b))[0];
      const mid = [(wreck[0] + ally.position[0]) / 2, (wreck[1] + ally.position[1]) / 2];
      await shoot(ctx, page, "ally-smoke", mid, Math.max(180, 1.6 * near(ally)));
    }
    await ctx.writeEvidence("smoke.json", { found: !!burning, tick: burning?.tick ?? null });
  }
  await page.close();
}
