// One network-held growth battle, driven through the player's production controls.
import { readFile } from "node:fs/promises";
import { advance, lab, obs, presented } from "./_lab.mjs";
import { approachContact, followEnemyObjective } from "./_contact-battle.mjs";

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

    const scoutId = await approachContact(page);
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
      await followEnemyObjective(page, heldObservation, scoutId);
    }
    if (!vehicle || (subject === "both" && !squad)) {
      await ctx.writeEvidence("meeting-failure.json", heldObservation);
      throw new Error(
        `the scout did not meet the required held bodies: ${JSON.stringify({
          tick: heldObservation.tick,
          own: heldObservation.own,
          identified: heldObservation.identified,
          contacts: heldObservation.contacts,
          heldResponses: held.length,
        })}`,
      );
    }
    await presented(page);
    return { page, held, timeline, heldObservation, vehicle, squad, release, close };
  } catch (error) {
    await close();
    throw error;
  }
}
