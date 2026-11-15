// One network-held growth battle, driven through the player's production controls.
import { readFile } from "node:fs/promises";
import { advance, lab, obs, presented } from "./_lab.mjs";

/** Stop at first vehicle contact, or at the vehicle-and-squad mechanics meeting. */
export async function meetStandIns(ctx, subject = "both") {
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
  const release = async () => {
    timeline.length = 0;
    hold = false;
    for (const route of held.splice(0)) await route.continue();
  };
  const close = async () => {
    hold = false;
    for (const route of held.splice(0)) await route.continue().catch(() => {});
    await page.unroute("**/bundle.bin").catch(() => {});
    await page.close().catch(() => {});
  };
  try {
    await ctx.openLab(page, ctx.url, 120000);
    hold = true;
    const installed = new Set(await lab(page, () => window.__lab.stats().models.installed));
    const catalog = JSON.parse(
      await readFile(new URL("../../fixtures/catalog.json", import.meta.url)),
    );
    const runtime = JSON.parse(
      await readFile(new URL("../../assets/runtime/catalog.json", import.meta.url)),
    );
    const unartedSquads = new Set(
      catalog.units
        .filter((unit) =>
          unit.body.squad?.slots.every((slot) =>
            catalog.soldiers[slot].appearance.every(
              (name) =>
                !installed.has(
                  runtime.appearances[name]?.factions?.[
                    new URL(ctx.url).searchParams.get("red_faction")
                  ] ?? name,
                ),
            ),
          ),
        )
        .map((unit) => unit.id),
    );
    timeline.length = 0;
    await lab(page, () => window.__lab.route.pause());
    await advance(page, 90 - (await obs(page)).tick);
    await presented(page);

    const entry = await page.evaluate(() => window.__lab.route.prepared().start.at);
    await page.evaluate(
      (at) =>
        window.__lab.setCamera({ ...window.__lab.camera(), target: [...at, 0], distance: 120 }),
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
    const current = await obs(page);
    const goal = current.skirmish.objectives
      .map((objective) => objective.center)
      .sort(
        (a, b) =>
          Math.hypot(b[0] - entry[0], b[1] - entry[1]) -
          Math.hypot(a[0] - entry[0], a[1] - entry[1]),
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
    let heldObservation;
    let vehicle;
    let squad;
    for (let tick = 0; tick < 10 * 60 * 30; tick += 30) {
      await advance(page, 30);
      heldObservation = await obs(page);
      vehicle = heldObservation.identified.find((unit) => unit.members.length === 0);
      const spread = (unit) =>
        Math.max(
          ...unit.members.map((at) =>
            Math.hypot(at[0] - unit.position[0], at[1] - unit.position[1]),
          ),
        );
      squad = heldObservation.identified
        .filter((unit) => unit.members.length > 1 && unartedSquads.has(unit.kind))
        .sort((a, b) => spread(a) - spread(b))[0];
      if (vehicle && (subject === "vehicle" || squad)) break;
    }
    if (!vehicle || (subject === "both" && !squad))
      throw new Error("the scout did not meet the required held bodies");
    await presented(page);
    return { page, held, timeline, heldObservation, vehicle, squad, release, close };
  } catch (error) {
    await close();
    throw error;
  }
}
