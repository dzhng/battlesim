// Slice 10: hold the growth admission's runtime responses at the browser
// network edge, inspect the live stand-ins, then release those same responses.
import { writeFile } from "node:fs/promises";
import { advance, lab, obs, presented } from "./_lab.mjs";

async function clickUnit(page, unit, button = "left") {
  const point = unit.members[0] ?? unit.position;
  const css = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] + 1), point);
  if (!css) throw new Error(`unit ${unit.id} is outside the camera`);
  await page.mouse.click(css[0], css[1], { button });
}

async function frameOn(page, unit) {
  await page.evaluate(
    (at) =>
      window.__lab.setCamera({ ...window.__lab.camera(), target: [at[0], at[1], 0], distance: 35 }),
    unit.position,
  );
  await page.evaluate(() => window.__lab.frame());
}

export async function run(ctx) {
  const page = await ctx.newPage();
  const held = [];
  let hold = false;
  await page.route("**/bundle.bin", async (route) => {
    if (!hold) return route.continue();
    held.push(route);
  });
  const timeline = [];
  page.on("console", (message) => {
    const match = /^\[battle \d+ ms\] (battle staged|art committed)/.exec(message.text());
    if (match) timeline.push(match[1]);
  });
  await ctx.openLab(page, ctx.url, 120000);
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);

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
  await obs(page);
  hold = true;
  const current = await obs(page);
  const goal = current.skirmish.objectives
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
    [current.own[0].id, goal],
  );
  for (let i = 0; i < 10 && !timeline.includes("battle staged"); i++) await advance(page, 60 * 30);
  ctx.check(
    "the growth stages while art responses are held",
    timeline.includes("battle staged"),
    JSON.stringify(timeline),
  );
  const heldObservation = await obs(page);
  const vehicle = heldObservation.identified.find((unit) => unit.members.length === 0);
  const squad = heldObservation.identified.find((unit) => unit.members.length > 0);
  ctx.check(
    "the newly seen vehicle and squad remain observed while art is held",
    !!vehicle && !!squad,
    JSON.stringify(heldObservation.identified),
  );
  if (!vehicle || !squad) return;

  const ownVehicle = current.own.find((unit) => unit.members.length === 0);
  const ownSquad = current.own.find((unit) => unit.members.length > 0);
  ctx.check(
    "the battle supplies vehicle and squad bodies for interaction",
    !!ownVehicle && !!ownSquad,
  );
  if (!ownVehicle || !ownSquad) return;

  await clickUnit(page, ownVehicle);
  await page.waitForFunction((id) => window.__lab.route.selected().includes(id), ownVehicle.id);
  await clickUnit(page, ownSquad);
  await page.waitForFunction((id) => window.__lab.route.selected().includes(id), ownSquad.id);
  ctx.check("withheld vehicle and squad bodies select", true);

  const beforeAcks = await lab(page, () => window.__lab.route.acks().length);
  await frameOn(page, vehicle);
  await clickUnit(page, vehicle, "right");
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, beforeAcks);
  const vehicleAttack = await lab(page, () => window.__lab.route.acks().at(-1)?.order.kind);
  ctx.check(
    "a click on the withheld vehicle is an attack target",
    vehicleAttack === "attack",
    vehicleAttack,
  );

  const squadAcks = await lab(page, () => window.__lab.route.acks().length);
  await frameOn(page, squad);
  await clickUnit(page, squad, "right");
  await page.waitForFunction((n) => window.__lab.route.acks().length > n, squadAcks);
  const squadAttack = await lab(page, () => window.__lab.route.acks().at(-1)?.order.kind);
  ctx.check(
    "a click on the withheld squad is an attack target",
    squadAttack === "attack",
    squadAttack,
  );

  await frameOn(page, vehicle);
  await writeFile(ctx.evidencePath("vehicle-before.png"), await page.screenshot());
  await frameOn(page, squad);
  await writeFile(ctx.evidencePath("squad-before.png"), await page.screenshot());

  for (const route of held.splice(0)) await route.continue();
  hold = false;
  await page.waitForTimeout(1000);
  ctx.check(
    "releasing the responses commits art in the same session",
    timeline.includes("art committed"),
    JSON.stringify(timeline),
  );
  await frameOn(page, vehicle);
  await writeFile(ctx.evidencePath("vehicle-after.png"), await page.screenshot());
  await frameOn(page, squad);
  await writeFile(ctx.evidencePath("squad-after.png"), await page.screenshot());
  await page.unroute("**/bundle.bin");
  await page.close();
}
