// A first-contact approach through the player's controls and the battle's
// reported geography. The first waypoint is its main settlement; subsequent
// objectives follow what the match says red holds, rather than a frozen route.
import { advance, lab, obs } from "./_lab.mjs";

export async function approachContact(page) {
  // A vehicle of blue's own, placed at its entry; the battle readied.
  const report = await lab(page, () => window.__lab.route.prepared());
  await lab(
    page,
    (at) => window.__lab.setCamera({ ...window.__lab.camera(), target: [...at, 0], distance: 120 }),
    report.start.at,
  );
  const view = page.viewportSize();
  const [cx, cy] = [view.width / 2, view.height / 2];
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "REC", exact: true }).click();
  await page.getByRole("button", { name: "M3A3 Bradley", exact: true }).click();
  await page.mouse.move(cx, cy);
  await page.waitForTimeout(100);
  await page.mouse.click(cx, cy);
  await advance(page, 1);
  // Choosing the variant closes its picker; keep the stage clear for body clicks.
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  await advance(page, 60 * 30);

  // It drives for the objective by the main settlement, which red's army
  // contests, until blue meets a kind its battle had not fielded. Both the
  // settlement and objectives come from this admitted map, not frozen points.
  const seen = await obs(page);
  if (!report.town || !seen.skirmish.objectives.length)
    throw new Error("the contact battle has no reported settlement objective");
  const player = seen.skirmish.players.find((player) => player.controller === "human");
  const scout = seen.own.find((unit) => unit.owner === player?.index && !unit.members.length);
  if (!scout)
    throw new Error(`the contact vehicle did not arrive: ${JSON.stringify(seen.skirmish.players)}`);
  const goal = [...seen.skirmish.objectives].sort(
    (a, b) =>
      Math.hypot(a.center[0] - report.town[0], a.center[1] - report.town[1]) -
      Math.hypot(b.center[0] - report.town[0], b.center[1] - report.town[1]),
  )[0].center;
  await lab(
    page,
    ([id, goal]) =>
      window.__lab.route.command({
        kind: "move",
        units: [id],
        gesture: 2801,
        goal,
        route: "fastest",
      }),
    [scout.id, goal],
  );
  return scout.id;
}

/** Once the first waypoint is reached, scout where the match says red is
 *  capturing or holding ground. Its army need not stay at the town junction. */
export async function followEnemyObjective(page, observation, scoutId) {
  const scout = observation.own.find((unit) => unit.id === scoutId);
  if (!scout || scout.goal) return;
  const distance = (objective) =>
    Math.hypot(objective.center[0] - scout.position[0], objective.center[1] - scout.position[1]);
  const target = observation.skirmish.objectives
    .filter((objective) => objective.owner === "red" || objective.capturing === "red")
    .sort((a, b) => distance(a) - distance(b))[0];
  if (!target || distance(target) <= target.radiusM) return;
  await lab(
    page,
    ([id, goal]) =>
      window.__lab.route.command({
        kind: "move",
        units: [id],
        gesture: 2801,
        goal,
        route: "fastest",
      }),
    [scoutId, target.center],
  );
}
