import { vec3 } from "math";
import { advance, lab, obs, snapshot, openBattle, presented } from "./_lab.mjs";

export async function run(ctx) {
  const page = await openBattle(ctx, { viewport: { width: 1920, height: 1080 }, timeout: 60000 });
  const hiddenPanel = await page.addStyleTag({
    content: "[data-testid=projectiles-panel] { display: none }",
  });
  const lanes = await lab(page, () => window.__lab.route.lanes());
  // Normalize command timing before any lane fires; page startup can deliver
  // its first shared-sighting orders a tick earlier or later under load.
  await advance(page, 20 - (await obs(page)).tick);
  await lab(
    page,
    async (lanes) => {
      const o = window.__lab.route.observation();
      for (const lane of lanes) {
        const own = o.own.find(
          (u) => Math.hypot(u.position[0] - lane.from[0], u.position[1] - lane.from[1]) < 2,
        );
        const target = o.identified.find(
          (u) => Math.hypot(u.position[0] - lane.to[0], u.position[1] - lane.to[1]) < 2,
        );
        await window.__lab.route.command({ kind: "stop", units: [own.id] });
        await window.__lab.route.command({
          kind: "attack",
          units: [own.id],
          target: { kind: "identified", id: target.id },
        });
      }
    },
    lanes,
  );
  const initial = await obs(page);
  const bodies = (o) => o.own.map((u) => [u.id, u.hp, u.memberIds, u.memberHp]);
  const seen = new Set();
  const samples = [];
  const midpoint = [0, 0, 0];
  // GPU positions narrow to f32; one centimetre distinguishes this head from
  // other rifles with the same colour without demanding bitwise equality.
  const tracerFor = async (lane, endpoint) =>
    (await lab(page, () => window.__lab.route.effectInstances())).find(
      (i) =>
        i.shape === 0 &&
        i.rgb.every((c, k) => Math.abs(c - lane.tracerRGB[k]) < 1e-4) &&
        vec3.squaredDistance(i.to, endpoint) < 0.0001,
    );
  // Read every tick so even a rifle crossing its lane between publications
  // contributes evidence. Shots and guidance remain the real authority's.
  for (let t = 0; t < 360; t++) {
    await advance(page, 1);
    const o = await obs(page);
    for (const lane of lanes) {
      if (seen.has(lane.weapon)) continue;
      const unit = o.own.find(
        (u) => Math.hypot(u.position[0] - lane.from[0], u.position[1] - lane.from[1]) < 2,
      );
      // Impact-clipped paths cover only part of a tick, so their length cannot
      // measure launch speed. Use a full, uninterrupted flight publication.
      const round = o.projectiles.find(
        (p) =>
          p.own &&
          p.kind === lane.weapon &&
          p.hit === "none" &&
          p.ricochets.length === 0 &&
          p.path.length > 1 &&
          (p.shooterMember !== null
            ? unit?.memberIds.includes(p.shooterMember)
            : Math.abs(p.path[0][1] - lane.from[1]) < 10 &&
              Math.abs(p.path[0][0] - lane.from[0]) < 50),
      );
      if (!round) continue;
      await presented(page);
      const tracer = await tracerFor(lane, round.path.at(-1));
      if (!tracer) continue;
      seen.add(lane.weapon);
      samples.push({ weapon: lane.weapon, tick: o.tick, round });
      await lab(page, (i) => window.__lab.route.show(i), lanes.indexOf(lane));
      await presented(page);
      await snapshot(ctx, page, `${lane.weapon}-wide.png`);
      let endpoint = round.path.at(-1);
      let currentTracer = tracer;
      for (let frame = 0; frame < 3; frame++) {
        if (currentTracer) vec3.lerp(midpoint, currentTracer.at, currentTracer.to, 0.5);
        await lab(
          page,
          (p) =>
            window.__lab.setCamera({
              ...window.__lab.camera(),
              target: p,
              distance: 80,
              pitch: 0.8,
              yaw: -Math.PI / 2,
            }),
          midpoint,
        );
        await snapshot(ctx, page, `${lane.weapon}-close-${frame}.png`);
        await advance(page, 1);
        await presented(page);
        const next = (await obs(page)).projectiles.find(
          (p) =>
            p.own &&
            p.kind === round.kind &&
            p.shooterMember === round.shooterMember &&
            vec3.squaredDistance(p.path[0], endpoint) < 0.0001,
        );
        if (next) endpoint = next.path.at(-1);
        currentTracer = await tracerFor(lane, endpoint);
      }
      if (lane.top_speed_mps) {
        // Also show established flight, beyond the crowded launch flash.
        await advance(page, 42);
        await presented(page);
        await lab(page, (i) => window.__lab.route.show(i), lanes.indexOf(lane));
        await snapshot(ctx, page, `${lane.weapon}-flight.png`);
      }
    }
    if (seen.size === lanes.length) break;
  }
  ctx.check(
    "every range-limit lane launches its gameplay round",
    seen.size === lanes.length,
    JSON.stringify([...seen]),
  );
  await ctx.writeEvidence("flight-samples.json", samples);
  for (const sample of samples) {
    const lane = lanes.find((l) => l.weapon === sample.weapon);
    const path = sample.round.path;
    const speed =
      path.slice(1).reduce((sum, p, i) => sum + Math.hypot(...p.map((x, k) => x - path[i][k])), 0) *
      lane.tick_hz;
    ctx.check(
      `${sample.weapon} publishes its gameplay flight speed`,
      lane.top_speed_mps
        ? speed >= lane.speed_mps - 1 && speed <= lane.top_speed_mps + 1
        : Math.abs(speed / lane.speed_mps - 1) < 0.02,
      `${speed.toFixed(1)} m/s`,
    );
  }
  await lab(page, () => window.__lab.route.show(-1));
  for (let frame = 0; frame < 3; frame++) {
    await snapshot(ctx, page, `street-${frame}.png`);
    await advance(page, 1);
    await presented(page);
  }
  await hiddenPanel.evaluate((node) => node.remove());
  await snapshot(ctx, page, "street-controls.png");
  await lab(page, () => window.__lab.route.show(4));
  await snapshot(ctx, page, "atgm-controls.png");
  const sustainedFrom = (await obs(page)).tick;
  await lab(page, () => window.__lab.route.resume());
  await page.waitForFunction((tick) => window.__lab.route.tick() >= tick + 600, sustainedFrom, {
    timeout: 120000,
  });
  await lab(page, () => window.__lab.route.pause());
  const after = await obs(page);
  ctx.check(
    "combat leaves every friendly body alive and undamaged",
    JSON.stringify(bodies(after)) === JSON.stringify(bodies(initial)),
  );
  ctx.check(
    "every weapon retains unlimited ammunition",
    after.own.flatMap((u) => u.mounts).every((m) => m.ammo.every((a) => a === null)),
  );
  ctx.check(
    "every firing lane continues shooting",
    lanes.every((l) => {
      const u = after.own.find(
        (u) => Math.hypot(u.position[0] - l.from[0], u.position[1] - l.from[1]) < 2,
      );
      return u?.weaponPoses.some((m) => m.shots >= 2);
    }),
  );
  await page.close();
}
