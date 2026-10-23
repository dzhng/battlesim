// An AI ally answers your lane under pressure (team skirmish).
// You and one AI ally against two AI: you send a lone squad to the
// objective nearest the enemy and hold it. As the enemy closes in, your
// ally sends a detachment, not its army, toward your objective, while the
// rest of its force keeps its own lane. It closes on your objective, or is
// engaged by whatever stands between.
// The planner rules themselves are tested in crates/sim/src/skirmish_ai/team/.
import { advance, aim, groundCss, lab, obs, presented, snapshot, until } from "./_lab.mjs";

const YOU = 0;
const ALLY = 1;
/** What you buy: one squad, too little to hold an objective alone. */
const FAMILY = "rifle";

const hypot = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const xy = (u) => [u.position[0], u.position[1]];

/** Ally units heading for `objective`: their goal lies on it. */
const heading = (o, objective) =>
  o.own.filter(
    (u) => u.owner === ALLY && u.goal && hypot(u.goal, objective.center) <= objective.radiusM * 1.5,
  );

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page, ctx.url, 120000);
  await page.addStyleTag({ content: ".frame-rate { display: none !important; }" });
  await lab(page, () => window.__lab.route.pause());
  await presented(page);

  // Your objective: the one farthest from your entry, where the enemy comes.
  const entry = await lab(page, () => window.__lab.route.prepared().start.at);
  const objectives = (await obs(page)).skirmish.objectives;
  const objective = [...objectives].sort(
    (a, b) => hypot(b.center, entry) - hypot(a.center, entry),
  )[0];
  ctx.check("the map has an objective to hold", !!objective, JSON.stringify(objectives));
  if (!objective) return;

  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  const search = page.getByRole("textbox", { name: "Search units" });
  await search.fill(FAMILY);
  await search.press("Enter");
  const at = await groundCss(page, [entry[0], entry[1]]);
  await page.mouse.move(at[0], at[1]);
  await page
    .waitForFunction(() => window.__lab.route.purchaseGhost?.()?.valid === true, undefined, {
      timeout: 5000,
    })
    .catch(() => {});
  await page.mouse.click(at[0], at[1]);
  await advance(page, 1);
  await page.mouse.click(40, 300, { button: "right" });
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();

  // Your squad on the field, then sent to hold the far objective.
  let o = await until(page, (o) => o.own.some((u) => u.owner === YOU), 150 * 30, 150);
  ctx.check("your squad is on the field", !!o);
  if (!o) return;
  const yours = o.own.filter((u) => u.owner === YOU).map((u) => u.id);
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "attack_move",
    units: yours,
    gesture: 200,
    goal: objective.center,
  });

  // Under pressure your ally answers: some of its units head for your
  // objective while others keep their own goals.
  o = await until(page, (o) => heading(o, objective).length > 0, 4 * 60 * 30, 300);
  const helpers = o ? heading(o, objective) : [];
  const allyUnits = o ? o.own.filter((u) => u.owner === ALLY) : [];
  ctx.check(
    "your ally sends units toward your objective",
    helpers.length > 0,
    JSON.stringify(allyUnits.map((u) => [u.id, u.kind, u.goal])),
  );
  ctx.check(
    "it sends a detachment, not its whole force",
    helpers.length > 0 && helpers.length < allyUnits.length,
    `${helpers.length} of ${allyUnits.length}`,
  );
  if (!o) return;
  await ctx.writeEvidence("help.json", {
    tick: o.tick,
    objective,
    helpers: helpers.map((u) => [u.id, u.kind, xy(u), u.goal]),
  });

  // The detachment on its way: followed by id (a squad that boards its own
  // carrier rides with no goal of its own, at the carrier's position), it
  // closes on your objective or is engaged on the way, an attack-move
  // holding to fight what stands between (enemy forces often do).
  // Every unit the ally sends joins as it is sent, a lost one's
  // replacement too, measured from where it was first seen sent.
  const sent = new Set();
  const start = new Map();
  const join = (o) => {
    for (const u of heading(o, objective)) {
      if (!sent.has(u.id)) {
        sent.add(u.id);
        start.set(u.id, hypot(xy(u), objective.center));
      }
    }
  };
  join(o);
  const answering = (o) =>
    o.own.filter(
      (u) =>
        sent.has(u.id) &&
        (u.state === "halted" || hypot(xy(u), objective.center) < start.get(u.id) - 100),
    );
  const path = [];
  const shown = await until(
    page,
    (o) => {
      join(o);
      path.push([
        o.tick,
        o.own.filter((u) => sent.has(u.id)).map((u) => [u.id, xy(u), u.goal, u.state]),
      ]);
      return answering(o).length > 0;
    },
    150 * 30,
    300,
  );
  await ctx.writeEvidence("path.json", path);
  ctx.check(
    "the detachment closes on your objective or fights its way there",
    !!shown,
    JSON.stringify(path.at(-1)),
  );
  if (!shown) return;
  // Wide enough to hold the detachment and the objective it is sent to.
  const party = answering(shown);
  const centre = party.reduce(
    (s, u) => [s[0] + u.position[0] / party.length, s[1] + u.position[1] / party.length],
    [0, 0],
  );
  // Weighted toward the detachment, which the reinforcement bar would
  // otherwise cover at the foot of the frame.
  const frame = [
    0.6 * centre[0] + 0.4 * objective.center[0],
    0.6 * centre[1] + 0.4 * objective.center[1],
  ];
  await aim(page, frame, { distance: Math.max(400, 1.25 * hypot(centre, objective.center)) });
  await presented(page);
  await snapshot(ctx, page, "ally-help.png");
}
