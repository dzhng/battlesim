// A skirmish fields the player's faction from the start and another
// faction's kind once its side meets it. That battle grows in place: the 3D
// keeps drawing, and the timeline the page logs shows the battle staged and
// its art committed, never a scene rebuild (which draws nothing until done).
// The fixture's link is a fixed small skirmish, so the meeting is the same
// every run.
import { advance, lab, obs, presented } from "./_lab.mjs";

/** Minutes of play allowed for blue to meet red. */
const ROUNDS = 10;

export async function run(ctx) {
  const page = await ctx.newPage();
  const timeline = [];
  page.on("console", (message) => {
    const kind = /^\[battle \d+ ms\] (battle staged|art committed|scene rebuild)\b/.exec(
      message.text(),
    );
    if (kind) timeline.push(kind[1]);
  });
  await ctx.openLab(page, ctx.url, 120000);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab?.route?.status().status === "paused");
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  // The first scene is built: from here on, only growth.
  timeline.length = 0;

  // A tank of blue's own, placed at its entry; the battle readied.
  const entry = await page.evaluate(() => window.__lab.route.prepared().start.at);
  await page.evaluate(
    (at) => window.__lab.setCamera({ ...window.__lab.camera(), target: [...at, 0], distance: 120 }),
    entry,
  );
  const view = page.viewportSize();
  const [cx, cy] = [view.width / 2, view.height / 2];
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "MBT", exact: true }).click();
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ }).click();
  await page.mouse.move(cx, cy);
  await page.waitForTimeout(100);
  await page.mouse.click(cx, cy);
  await advance(page, 1);
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  await advance(page, 60 * 30);

  // It drives for the objective farthest from blue's entry, which red's army
  // contests, until blue meets a kind its battle had not fielded.
  const seen = await obs(page);
  const goal = seen.skirmish.objectives
    .map((objective) => objective.center)
    .sort(
      (a, b) =>
        Math.hypot(b[0] - entry[0], b[1] - entry[1]) - Math.hypot(a[0] - entry[0], a[1] - entry[1]),
    )[0];
  await page.evaluate(
    ([id, at]) =>
      window.__lab.route.command({
        kind: "move",
        units: [id],
        gesture: 2801,
        goal: at,
        route: "fastest",
      }),
    [seen.own[0].id, goal],
  );
  for (let round = 0; round < ROUNDS && !timeline.includes("battle staged"); round++)
    await advance(page, 60 * 30);
  // Its art is admitted while the scene keeps drawing.
  const staged = Date.now();
  const committed = () =>
    timeline.slice(timeline.indexOf("battle staged")).includes("art committed");
  while (timeline.includes("battle staged") && !committed() && Date.now() - staged < 60_000)
    await page.waitForTimeout(250);
  ctx.check(
    "a newly fielded kind joins the battle's scene without rebuilding it",
    timeline.includes("battle staged") && committed() && !timeline.includes("scene rebuild"),
    JSON.stringify(timeline),
  );
  await page.close();
}
