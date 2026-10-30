import { nodeCatalogSet } from "../src/battle/catalog/node.ts";
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
  // Get the foot squad to its hold before the opposing armies close it off.
  // Pick an affordable troop helicopter by capacity, independently of roster IDs.
  const { units: catalog } = await nodeCatalogSet("game");
  const wallet = (await obs(page)).skirmish.players[YOU];
  const squadKind = wallet.pending[0].kind;
  const seats = catalog.slots(squadKind).length;
  const carrierType = catalog.cards
    .filter(
      (card) =>
        !card.disabled_reason && card.roster.factions.includes("us") && card.cost <= wallet.credits,
    )
    .map((card) => catalog.type(card.id))
    .filter((type) => type.mobility.air && type.capabilities.carriage?.seats >= seats)
    .sort((a, b) => a.cost - b.cost || a.id.localeCompare(b.id))[0];
  if (!carrierType) throw Error("no affordable troop helicopter fits the bought squad");
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "confirm_purchase",
    variant: carrierType.id,
    destination: entry.map(
      (v, i) => v + ((objective.center[i] - v) * 30) / hypot(entry, objective.center),
    ),
  });
  await advance(page, 1);
  ctx.check(
    "a carrier is reserved to deliver the squad",
    (await obs(page)).skirmish.players[YOU].pending.some((p) => p.kind === carrierType.id),
  );
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();

  // Clear the road entry with the first squad so its carrier can physically
  // enter behind it, then gather the pair nearby for boarding.
  let o = await until(
    page,
    (o) => o.own.some((u) => u.owner === YOU && u.kind === squadKind),
    150 * 30,
    150,
  );
  ctx.check("your squad is on the field", !!o);
  if (!o) return;
  const rifle = o.own.find((u) => u.owner === YOU && u.kind === squadKind);
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "move",
    units: [rifle.id],
    gesture: 199,
    route: "shortest",
    goal: entry.map(
      (v, i) => v + ((objective.center[i] - v) * 80) / hypot(entry, objective.center),
    ),
  });
  o = await until(
    page,
    (o) => o.own.some((u) => u.owner === YOU && u.kind === carrierType.id),
    150 * 30,
    30,
  );
  ctx.check(
    "your carrier is on the field",
    !!o,
    JSON.stringify((await obs(page)).skirmish.players[YOU]),
  );
  if (!o) return;
  const squad = o.own.find((u) => u.owner === YOU && u.kind === squadKind);
  const carrier = o.own.find((u) => u.owner === YOU && u.kind === carrierType.id);
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "board",
    units: [squad.id],
    carrier: carrier.id,
    gesture: 200,
  });
  const aboard = await until(
    page,
    (o) => o.own.find((u) => u.id === squad.id)?.hold?.phase === "inside",
    150 * 30,
    30,
  );
  ctx.check("the squad boards its carrier", !!aboard);
  if (!aboard) return;
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "unload",
    units: [carrier.id],
    goal: objective.center,
    route: "fastest",
    gesture: 201,
  });
  const approach = [];
  const arrived = await until(
    page,
    (o) => {
      const unit = o.own.find((u) => u.id === squad.id);
      return !!unit && !unit.hold && hypot(xy(unit), objective.center) <= objective.radiusM;
    },
    10 * 60 * 30,
    150,
    (o) =>
      approach.push({
        tick: o.tick,
        units: o.own
          .filter((u) => u.owner === YOU)
          .map((u) => ({ id: u.id, at: xy(u), goal: u.goal, state: u.state, hold: u.hold })),
        phase: o.skirmish.phase,
      }),
  );
  await ctx.writeEvidence("approach.json", approach);
  ctx.check(
    "your squad reaches the objective it will hold",
    !!arrived,
    JSON.stringify(approach.at(-1)),
  );
  if (!arrived) return;
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "move",
    units: [carrier.id],
    goal: entry,
    route: "fastest",
    gesture: 202,
  });

  // Under pressure your ally answers: some of its units head for your
  // objective while others keep their own goals.
  o = await until(page, (o) => heading(o, objective).length > 0, 4 * 60 * 30, 300);
  const latest = o ?? (await obs(page));
  const helpers = heading(latest, objective);
  const allyUnits = latest.own.filter((u) => u.owner === ALLY);
  await ctx.writeEvidence("support-state.json", { objective, observation: latest });
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
    // A helper can halt to aim for less than two seconds between movements.
    15,
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
