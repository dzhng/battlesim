import { installWorkerStartFence } from "./_workerStart.mjs";
// One squad holds an objective against visible enemies. The AI ally has a
// separate main force and sends only a detachment to help. Fixed test forces
// isolate that decision from generated-map travel and roster tuning.
import { advance, aim, lab, obs, presented, snapshot, until } from "./_lab.mjs";

const YOU = 0;
const ALLY = 1;

const hypot = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const xy = (u) => [u.position[0], u.position[1]];

/** Ally units heading for `objective`: their goal lies on it. */
const heading = (o, objective) =>
  o.own.filter(
    (u) => u.owner === ALLY && u.goal && hypot(u.goal, objective.center) <= objective.radiusM * 1.5,
  );

export async function run(ctx) {
  const page = await ctx.newPage();
  await page.addInitScript(installWorkerStartFence);
  await page.addInitScript(() => window.__pauseNextWorkerStart(1));
  await ctx.openLab(page, ctx.url, 120000);
  await page.addStyleTag({ content: ".frame-rate { display: none !important; }" });
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(
    () => window.__lab.route.status().status === "paused" && window.__lab.route.tick() >= 1,
  );
  ctx.check(
    "the opening is held at tick one",
    (await lab(page, () => window.__lab.route.tick())) === 1,
  );
  await presented(page);
  await ctx.writeEvidence(
    "opening.json",
    await lab(page, () => ({
      tick: window.__lab.route.tick(),
      digest: window.__lab.route.digest(),
    })),
  );

  const east = (await obs(page)).skirmish.objectives.find((o) => o.id === "east");
  if (!east) throw Error("the help fixture has no east objective");
  await lab(page, (order) => window.__lab.route.command(order), {
    kind: "confirm_purchase",
    variant: "test_rifle",
    destination: east.center,
  });
  await advance(page, 1);
  await lab(page, (order) => window.__lab.route.command(order), { kind: "ready" });
  const initial = await until(page, (o) => o.own.some((u) => u.owner === YOU), 150 * 30, 30);
  ctx.check("your squad is on the field", !!initial);
  if (!initial) return;
  const squad = initial.own.find((u) => u.owner === YOU);
  const objective = initial.skirmish.objectives.find(
    (o) => squad && hypot(xy(squad), o.center) <= o.radiusM,
  );
  ctx.check(
    "your squad holds an objective",
    !!objective,
    JSON.stringify({ squad: squad?.id, at: squad && xy(squad), objective }),
  );
  if (!objective) return;
  ctx.check(
    "the held objective has visible enemy pressure",
    initial.identified.some((u) => hypot(xy(u), objective.center) <= objective.radiusM * 3),
  );

  // Under pressure your ally answers: some of its units head for your
  // objective while others keep their own goals.
  const support = [];
  let o = await until(
    page,
    (o) => heading(o, objective).length > 0,
    4 * 60 * 30,
    300,
    (o) =>
      support.push({
        tick: o.tick,
        squad: o.own.find((u) => u.id === squad.id)?.position ?? null,
        enemies: o.identified.map((u) => [u.id, xy(u)]),
        allies: o.own.filter((u) => u.owner === ALLY).map((u) => [u.id, u.goal]),
      }),
  );
  await ctx.writeEvidence("support-trace.json", support);
  const latest = o ?? (await obs(page));
  const helpers = heading(latest, objective);
  const allyUnits = latest.own.filter((u) => u.owner === ALLY);
  const holding = latest.own.find((u) => u.id === squad.id);
  ctx.check(
    "your squad still holds the objective when help is sent",
    !!holding && hypot(xy(holding), objective.center) <= objective.radiusM,
  );
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

  // Follow each dispatched unit from its first help order. It closes on the
  // objective or holds to fight an enemy along its attack move.
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
