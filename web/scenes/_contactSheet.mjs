// Contact callouts anchor to the reported ground center, independent of uncertainty radius.
import { writeFile } from "node:fs/promises";
import {
  openBattle,
  obs,
  aim,
  snapshot,
  groundCss,
  until,
  lab,
  advance,
  presented,
} from "./_lab.mjs";
import { decode, writeCrop } from "./_png.mjs";
import { curvePitch, game, unitType } from "./_units.mjs";

export async function contactTour(ctx) {
  const page = await openBattle(ctx, {
    url: new URL("/battle/village", ctx.url).href,
    viewport: { width: 1920, height: 1080 },
    tick: 600, // A fixed paused checkpoint keeps asset startup out of the seeded journey.
  });
  await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
    timeout: 30000,
  });
  const { observation, contact, journey } = await stageContact(ctx, page);
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
  const panelName = await page
    .locator(`[data-contact="${contact.id}"] .ro-name-word`)
    .textContent();
  ctx.check(
    "the renewed contact panel names the previously identified tank",
    panelName === unitType(contact.kind).name.toUpperCase(),
    JSON.stringify({ panelName, contact }),
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
    JSON.stringify({ tick: observation.tick, journey, contact, measurements }, null, 2),
  );
  await page.close();
}

// Recon identifies at the open western sight boundary. Withdrawing the one
// observer leaves the tank alive to refresh its own report through firing.
async function stageContact(ctx, page) {
  const start = await obs(page);
  ctx.check(
    "contact journey starts at its seeded checkpoint",
    start.tick === 600,
    JSON.stringify({ tick: start.tick }),
  );
  if (start.tick !== 600) throw new Error("contact journey missed its opening checkpoint");
  const recon = start.own.find((u) => u.kind === "recon");
  if (!recon) throw new Error("contact capture needs a living recon observer");
  const journey = [];
  const stage = async (order, name) => {
    const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
    await lab(page, (o) => window.__lab.route.command(o), order);
    await page.waitForFunction((n) => (window.__lab.route.acks()[0]?.seq ?? 0) > n, before);
    const command = await lab(page, () => window.__lab.route.acks()[0]);
    const admitted =
      command.ack.error === null &&
      (order.kind !== "move" ||
        order.units.every((id) =>
          command.ack.placement?.destinations.some((d) => d.unit === id && d.placed),
        ));
    journey.push({ name, command });
    ctx.check(`${name}: command is admitted`, admitted, JSON.stringify(command));
    if (!admitted) throw new Error(`${name}: refused staging command`);
    // Opening fire policy and movement share the same application tick.
    if (order.kind === "move")
      await advance(page, Math.max(0, command.ack.applied_tick - (await obs(page)).tick));
    return command.ack.placement?.destinations.find((d) => d.unit === recon.id)?.goal;
  };
  const arrived = (o, goal) => {
    const unit = o.own.find((u) => u.id === recon.id);
    return unit && Math.hypot(unit.position[0] - goal[0], unit.position[1] - goal[1]) < 1;
  };
  await stage(
    { kind: "set_engagement", units: [recon.id], policy: "return_fire_only" },
    "contact observer holds fire",
  );
  const approach = await stage(
    { kind: "move", units: [recon.id], gesture: 1, goal: [460, 750], route: "shortest" },
    "contact identification approach",
  );
  const seen = await until(
    page,
    (o) => arrived(o, approach) && o.identified.some((u) => u.kind === "tank"),
    4200,
    15,
  );
  ctx.check(
    "the observer arrives and identifies the tank before withdrawing",
    !!seen,
    JSON.stringify(seen && { tick: seen.tick, own: seen.own, identified: seen.identified }),
  );
  if (!seen) throw new Error("contact capture needs an arrived observer identifying the tank");
  const tank = seen.identified.find((u) => u.kind === "tank");
  journey.push({
    name: "identified",
    tick: seen.tick,
    tank,
    observer: seen.own.find((u) => u.id === recon.id),
  });
  const withdrawal = await stage(
    { kind: "move", units: [recon.id], gesture: 2, goal: [425, 750], route: "shortest" },
    "contact visual-memory withdrawal",
  );
  const hidden = await until(
    page,
    (o) =>
      !o.identified.some((u) => u.id === tank.id) &&
      o.contacts.some((c) => c.kind === "tank" && c.source === "last_seen"),
    1200,
    15,
  );
  ctx.check(
    "withdrawing loses sight of the identified tank",
    !!hidden,
    JSON.stringify(hidden?.contacts),
  );
  if (!hidden) throw new Error("contact capture needs the tank's last sighting");
  const memory = hidden.contacts.find((c) => c.kind === "tank" && c.source === "last_seen");
  journey.push({ name: "last_seen", tick: hidden.tick, contact: memory });
  const observation = await until(
    page,
    (o) =>
      arrived(o, withdrawal) &&
      !o.identified.some((u) => u.id === tank.id) &&
      o.contacts.some(
        (c) =>
          c.id === memory.id &&
          c.kind === "tank" &&
          c.source === "firing" &&
          c.evidenceTick > memory.evidenceTick,
      ),
    2400,
    15,
  );
  ctx.check(
    "the observer arrives withdrawn and fresh hidden firing renews the same tank report",
    !!observation,
    JSON.stringify(
      observation && {
        tick: observation.tick,
        own: observation.own,
        contacts: observation.contacts,
      },
    ),
  );
  if (!observation) throw new Error("contact capture needs the hidden tank's refreshed evidence");
  const contact = observation.contacts.find((c) => c.id === memory.id);
  journey.push({
    name: "firing",
    tick: observation.tick,
    contact,
    observer: observation.own.find((u) => u.id === recon.id),
  });
  await ctx.writeEvidence("contact-approach.json", journey);
  await presented(page);
  return { observation, contact, journey };
}
