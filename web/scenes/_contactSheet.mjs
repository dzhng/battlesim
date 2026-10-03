// Contact callouts anchor to the reported ground center, independent of uncertainty radius.
import { writeFile } from "node:fs/promises";
import { openBattle, obs, aim, snapshot, groundCss, until, lab } from "./_lab.mjs";
import { decode, writeCrop } from "./_png.mjs";
import { curvePitch, game } from "./_units.mjs";

export async function contactTour(ctx) {
  const page = await openBattle(ctx, {
    url: new URL("/battle/village", ctx.url).href,
    viewport: { width: 1920, height: 1080 },
    tick: 30,
    grass: true,
  });
  // Approach from the open ground west of the town. The town's centre can
  // refuse the whole formation, leaving every observer at its starting point.
  await lab(page, () =>
    window.__lab.route.command({
      kind: "attack_move",
      units: window.__lab.route.observation().own.map((u) => u.id),
      gesture: 1,
      goal: [700, 800],
    }),
  );
  await lab(page, () => window.__lab.route.advance(1));
  await page.waitForFunction(
    () =>
      window.__lab.route
        .acks()
        .some(
          (record) =>
            record.order.kind === "attack_move" &&
            record.order.gesture === 1 &&
            record.order.goal[0] === 700 &&
            record.order.goal[1] === 800,
        ),
    undefined,
    { timeout: 5000 },
  );
  const approach = await lab(
    page,
    () =>
      window.__lab.route
        .acks()
        .find(
          (record) =>
            record.order.kind === "attack_move" &&
            record.order.gesture === 1 &&
            record.order.goal[0] === 700 &&
            record.order.goal[1] === 800,
        )?.ack,
  );
  const admitted =
    approach && !approach.error && approach.placement?.destinations.some((d) => d.placed);
  await ctx.writeEvidence("contact-approach.json", approach);
  ctx.check(
    "the contact approach admits observers before capture",
    !!admitted,
    JSON.stringify(approach),
  );
  if (!admitted) return page.close();
  const refreshed = (o) => o.contacts.some((c) => c.source === "firing" && c.kind === "tank");
  // Follow the one tank's visual memory until fresh hidden firing updates it.
  // No report is a failed fixture, never a passing empty panel comparison.
  let observation = await obs(page);
  if (!refreshed(observation)) observation = await until(page, refreshed, 6000, 30);
  const contact = observation?.contacts.find((c) => c.source === "firing" && c.kind === "tank");
  if (!contact) throw new Error("contact capture needs the hidden tank's refreshed evidence");
  ctx.check(
    "one hidden tank has one updated area rather than old and new patches",
    observation.contacts.length === 1,
    JSON.stringify(observation.contacts),
  );
  const drawnIds = await page
    .locator("[data-contact]")
    .evaluateAll((nodes) => nodes.map((n) => Number(n.dataset.contact)).sort((a, b) => a - b));
  const expectedIds = observation.contacts.map((c) => c.id).sort((a, b) => a - b);
  ctx.check(
    "every contact report has its info panel",
    JSON.stringify(drawnIds) === JSON.stringify(expectedIds),
    JSON.stringify({ drawnIds, expectedIds }),
  );
  const label = process.env.CONTACT_LABEL ?? "center";
  const measurements = [];
  for (const distance of [180, 65]) {
    await aim(page, [...contact.center, 0], {
      distance,
      pitch: curvePitch(distance),
      yaw: game.presentation.camera.default.yaw,
    });
    const file = `contact-${label}-${distance}`;
    const png = await snapshot(ctx, page, `${file}.png`);
    const center = await groundCss(page, contact.center);
    const leader = await page.evaluate((id) => {
      const panel = document.querySelector(`[data-contact="${id}"]`);
      const path = panel?.parentElement.querySelector(".ro-leader")?.getAttribute("d");
      const start = path && [...path.matchAll(/M ([\d.-]+) ([\d.-]+)/g)][1];
      return start ? [Number(start[1]), Number(start[2])] : null;
    }, contact.id);
    measurements.push({ distance, center, leader });
    ctx.check(
      `contact leader begins at its reported center at ${distance}m`,
      leader && Math.hypot(leader[0] - center[0], leader[1] - center[1]) < 0.15,
      JSON.stringify(measurements.at(-1)),
    );
    await writeCrop(
      decode(png),
      ctx.evidencePath(`${file}-crop.png`),
      center[0] + 85,
      center[1] - 45,
      240,
      180,
      2,
    );
  }
  await writeFile(
    ctx.evidencePath(`contact-${label}.json`),
    JSON.stringify({ tick: observation.tick, approach, contact, measurements }, null, 2),
  );
  await page.close();
}
