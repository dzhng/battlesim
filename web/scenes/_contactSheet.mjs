// Contact callouts anchor to the reported ground center, independent of uncertainty radius.
import { writeFile } from "node:fs/promises";
import { openBattle, obs, aim, snapshot, groundCss, until, lab, advance } from "./_lab.mjs";
import { decode, writeCrop } from "./_png.mjs";
import { curvePitch, game } from "./_units.mjs";

export async function contactTour(ctx) {
  const page = await openBattle(ctx, {
    url: new URL("/battle/village", ctx.url).href,
    viewport: { width: 1920, height: 1080 },
    tick: 30,
    grass: true,
  });
  // An admitted approach brings the observers within hearing range.
  const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
  await lab(page, () =>
    window.__lab.route.command({
      kind: "attack_move",
      units: window.__lab.route
        .observation()
        .own.filter((u) => u.kind !== "supply")
        .map((u) => u.id),
      gesture: 1,
      goal: [680, 840],
    }),
  );
  await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
  const command = await lab(page, () => window.__lab.route.acks()[0]);
  const admitted =
    command.ack.error === null &&
    command.order.units.every((id) =>
      command.ack.placement?.destinations.some((d) => d.unit === id && d.placed),
    );
  ctx.check("contact approach: every destination is admitted", admitted, JSON.stringify(command));
  if (!admitted) throw new Error("contact approach was refused");
  await advance(page, Math.max(0, command.ack.applied_tick - (await obs(page)).tick));
  const mixedReports = (o) =>
    o.contacts.some((c) => !c.primaryLabel) && o.contacts.some((c) => c.primaryLabel);
  // Prefer a moment with both kinds of report, so the panel check below has an
  // unlabelled one to leave out; a battle that never mixes them is captured as
  // it stands at the end of the wait.
  let observation = await obs(page);
  if (!mixedReports(observation)) {
    observation = (await until(page, mixedReports, 6000, 30)) ?? (await obs(page));
  }
  const contact =
    observation.contacts.find((c) => c.primaryLabel && c.source === "last_seen") ??
    observation.contacts.find((c) => c.primaryLabel);
  if (!contact) throw new Error("contact capture needs a labelled contact");
  const drawnIds = await page.locator("[data-contact]").evaluateAll((nodes) =>
    nodes
      .filter((n) => !n.closest(".ro-callout")?.dataset.retiring)
      .map((n) => Number(n.dataset.contact))
      .sort((a, b) => a - b),
  );
  const expectedIds = observation.contacts
    .filter((c) => c.primaryLabel)
    .map((c) => c.id)
    .sort((a, b) => a - b);
  ctx.check(
    "only preferred contact reports get panels",
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
    JSON.stringify({ contact, measurements }, null, 2),
  );
  await page.close();
}
