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
import { curvePitch, game, streetMap, unitType, visuals } from "./_units.mjs";

export async function contactTour(ctx) {
  const page = await openBattle(ctx, {
    url: new URL("/labs/street", ctx.url).href,
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
      yaw: visuals.camera.default.yaw,
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

// The street's houses see the open west out to their sight, and the tank
// shells whatever they see, so the recon scouts from the west wood instead.
// It rounds the wood's south, out of the houses' sight like the woods tour's
// walker, and identifies the tank from the trees at the wood's east edge,
// hidden itself. It slips back into the wood out of sight, and a rifle
// section pushes up the road into the houses' sight: the tank, unseen, opens
// fire on it, and its firing renews the report the recon's sighting left.
async function stageContact(ctx, page) {
  const start = await obs(page);
  ctx.check(
    "contact journey starts at its seeded checkpoint",
    start.tick === 600,
    JSON.stringify({ tick: start.tick }),
  );
  if (start.tick !== 600) throw new Error("contact journey missed its opening checkpoint");
  const recon = start.own.find((u) => u.kind === "test_recon");
  if (!recon) throw new Error("contact capture needs a living recon observer");
  const journey = [];
  const section = start.own.find((u) => u.kind === "test_rifle");
  if (!section) throw new Error("contact capture needs a rifle section to draw the tank's fire");
  const stage = async (order, name, queued = false) => {
    const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
    await lab(page, (c) => window.__lab.route.command(c.order, c.queued), { order, queued });
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
  // Arrived: its moves done, settled in its goal's area. Among the trees the
  // scouts take the cover there, so their centre sits metres off the goal.
  const arrived = (o, goal) => {
    const unit = o.own.find((u) => u.id === recon.id);
    return (
      unit &&
      !unit.goal &&
      unit.queue.length === 0 &&
      Math.hypot(unit.area.anchor[0] - goal[0], unit.area.anchor[1] - goal[1]) < 1 &&
      Math.hypot(unit.position[0] - goal[0], unit.position[1] - goal[1]) < unit.area.radius
    );
  };
  await stage(
    { kind: "set_engagement", units: [recon.id], policy: "return_fire_only" },
    "contact observer holds fire",
  );
  // The section waits on the road just beyond the houses' sight.
  await stage(
    { kind: "move", units: [section.id], gesture: 3, goal: [340, 780], route: "shortest" },
    "contact rifle section waits out of sight",
  );
  // Round the wood's south-west corner, then through the trees to its east
  // edge, 260 m from the tank and 170 m from the nearest house.
  const [[x0], , [x1, y1]] = streetMap.forests[0].shape.ring;
  const legs = [
    [x0 - 300, y1 + 40],
    [x0 - 60, y1 + 40],
    [x1 - 10, y1 - 40],
  ];
  let approach;
  for (const [k, goal] of legs.entries())
    approach = await stage(
      { kind: "move", units: [recon.id], gesture: 1, goal, route: "shortest" },
      `contact identification approach, leg ${k + 1}`,
      k > 0,
    );
  let last = start;
  const seen = await until(
    page,
    (o) => arrived(o, approach) && o.identified.some((u) => u.kind === "test_tank"),
    12000,
    15,
    (o) => (last = o),
  );
  const shown = seen ?? last;
  ctx.check(
    "the observer arrives and identifies the tank before withdrawing",
    !!seen,
    JSON.stringify({
      tick: shown.tick,
      observer: shown.own.find((u) => u.id === recon.id),
      identified: shown.identified.map((u) => [u.kind, u.position]),
    }),
  );
  if (!seen) throw new Error("contact capture needs an arrived observer identifying the tank");
  const tank = seen.identified.find((u) => u.kind === "test_tank");
  journey.push({
    name: "identified",
    tick: seen.tick,
    tank,
    observer: seen.own.find((u) => u.id === recon.id),
  });
  const withdrawal = await stage(
    // Back into the trees: 100 m of wood lies between it and the tank.
    { kind: "move", units: [recon.id], gesture: 2, goal: [x0 + 90, y1 - 20], route: "shortest" },
    "contact visual-memory withdrawal",
  );
  const hidden = await until(
    page,
    (o) =>
      !o.identified.some((u) => u.id === tank.id) &&
      o.contacts.some((c) => c.kind === "test_tank" && c.source === "last_seen"),
    1200,
    15,
  );
  ctx.check(
    "withdrawing loses sight of the identified tank",
    !!hidden,
    JSON.stringify(hidden?.contacts),
  );
  if (!hidden) throw new Error("contact capture needs the tank's last sighting");
  const memory = hidden.contacts.find((c) => c.kind === "test_tank" && c.source === "last_seen");
  journey.push({ name: "last_seen", tick: hidden.tick, contact: memory });
  // Up the road into the houses' sight, still beyond its own sight of the tank.
  await stage(
    { kind: "move", units: [section.id], gesture: 4, goal: [450, 780], route: "shortest" },
    "contact rifle section draws the tank's fire",
  );
  // The scouts also saw the houses and the jeep; those reports expire, and
  // their panels fade out. The capture waits until the last has faded.
  const fade = visuals.contacts.fade_s * game.tick_hz;
  let held = new Set(hidden.contacts.map((c) => c.id));
  let faded = hidden.tick;
  const observation = await until(
    page,
    (o) =>
      o.tick >= faded + fade &&
      arrived(o, withdrawal) &&
      !o.identified.some((u) => u.id === tank.id) &&
      o.contacts.some(
        (c) =>
          c.id === memory.id &&
          c.kind === "test_tank" &&
          c.source === "firing" &&
          c.evidenceTick > memory.evidenceTick,
      ),
    2400,
    15,
    (o) => {
      const now = new Set(o.contacts.map((c) => c.id));
      if ([...held].some((id) => !now.has(id))) faded = o.tick;
      held = now;
    },
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
