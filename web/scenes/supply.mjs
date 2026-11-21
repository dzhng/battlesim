// Slice 13: service eligibility, finite stock and replacements in the browser.
import { decode, writeCrop, mostChanged } from "./_png.mjs";
import { lab, obs, advance, snapshot, openBattle, presented, overlaysOnly } from "./_lab.mjs";

import { unitType } from "./_units.mjs";

const unit = (o, id) => o.own.find((u) => u.id === id);
const seen = (o, id) => o.identified.find((u) => u.id === id);

/** The supply rows a recipient's panel can carry. */
const SUPPLY_ROWS = ["resupplying"];

/** Each own panel's unit, its supply row's word (null without one) and every
 *  state row's word, as drawn. */
const supplyRows = (page) =>
  page.evaluate(
    (kinds) =>
      [...document.querySelectorAll("[data-testid=readouts] .ro-unit[data-owner=own]")].map((n) => {
        const states = [...n.querySelectorAll(".ro-state")];
        const supply = states.find((e) => kinds.includes(e.dataset.state));
        return {
          unit: Number(n.dataset.unit),
          row: supply?.querySelector(".ro-state-word").textContent ?? null,
          words: states.map((e) => e.querySelector(".ro-state-word").textContent),
        };
      }),
    SUPPLY_ROWS,
  );
const rowOf = (rows, id) => rows.find((r) => r.unit === id);

async function select(page, ids) {
  await lab(page, (x) => window.__lab.route.select(x), ids);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, ids.length);
  await page.evaluate(() => window.__lab.frame());
}

async function frame(ctx, page, name, crop) {
  const shot = await snapshot(ctx, page, `frame-${name}-1280x800.png`);
  if (crop) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2] ?? 0), crop);
    await writeCrop(
      decode(shot),
      ctx.evidencePath(`crop-${name}-2x.png`),
      at[0],
      at[1],
      170,
      130,
      2,
    );
  }
}

export async function run(ctx) {
  const page = await openBattle(ctx);
  let o = await obs(page);
  const start = {
    stock: unit(o, 0).stock,
    fallen: JSON.stringify(o.corpses.filter((c) => c.own)),
    at: unit(o, 2).mounts[1].ammo[0],
  };
  ctx.check(
    "nothing is served before the truck is set up",
    unit(o, 3).service === "source_not_deployed",
    JSON.stringify(unit(o, 3)?.service),
  );
  await page.evaluate(() => window.__lab.frame());
  let rows = await supplyRows(page);
  ctx.check(
    "before the truck is set up, nobody shows a supply row and the truck shows DEPLOYING",
    rows.every((r) => r.row === null) && rowOf(rows, 0)?.words.includes("DEPLOYING"),
    JSON.stringify(rows),
  );
  await frame(ctx, page, "setting-up", [205, 210]);

  // Set up, then catch the service underway: what recipients and trucks
  // show is judged while the truck serves. Supply serves every need at
  // once, so by the time the squad is whole nobody is served any more.
  await advance(page, 90);
  o = await obs(page);
  for (let more = 0; more < 60 * 30 && !o.own.some((u) => u.service === "serving"); more += 15) {
    await advance(page, 15);
    o = await obs(page);
  }
  // Recipients show active service only; trucks show stock and supplying.
  await page.evaluate(() => window.__lab.frame());
  rows = await supplyRows(page);
  const WORD = {
    serving: "RESUPPLYING",
  };
  const serving = o.own.filter((u) => u.service === "serving").map((u) => u.id);
  ctx.check(
    "only actively resupplying recipients show a supply row",
    serving.length > 0 &&
      o.own.every((u) => (rowOf(rows, u.id)?.row ?? null) === (WORD[u.service] ?? null)),
    JSON.stringify({ services: o.own.map((u) => [u.id, u.service]), rows }),
  );
  ctx.check(
    "the scouts beside the empty truck have no redundant supply row",
    rowOf(rows, 5)?.row === null,
    JSON.stringify(rowOf(rows, 5)),
  );
  ctx.check(
    "each truck's panel shows its stock, and the serving one SUPPLYING",
    rowOf(rows, 0)?.words.includes(`SUPPLY ${unit(o, 0).stock}`) &&
      rowOf(rows, 0)?.words.includes("SUPPLYING") &&
      rowOf(rows, 4)?.words.includes("SUPPLY 0") &&
      !rowOf(rows, 4)?.words.includes("SUPPLYING"),
    JSON.stringify([rowOf(rows, 0), rowOf(rows, 4)]),
  );
  await frame(ctx, page, "empty-truck", [345, 325]);
  // The truck's reach shows only while it is selected.
  await frame(ctx, page, "serving-unselected", [205, 210]);
  await select(page, [0]);
  await frame(ctx, page, "serving", [205, 210]);
  const truckCamera = await lab(page, () => window.__lab.camera());
  await lab(page, (camera) => window.__lab.setCamera(camera), {
    ...truckCamera,
    target: [190, 210, 0],
    distance: 150,
    pitch: 0.85,
  });
  await frame(ctx, page, "marks-truck", [205, 210]);
  await lab(page, (camera) => window.__lab.setCamera(camera), truckCamera);

  // Serve until the squad is whole: replacement takes the rules' time per
  // soldier, so the wait follows the rules, within a minute.
  for (let more = 0; more < 60 * 30 && unit(o, 3).members.length < 8; more += 30) {
    await advance(page, 30);
    o = await obs(page);
  }
  const truck = unit(o, 0);
  const readyText = await page.getByTestId("service-ready").first().textContent();
  ctx.check(
    "the service-ready text says the deployed truck is ready",
    /service: ready \(fully deployed\)/.test(readyText ?? ""),
    readyText ?? "missing service-ready text",
  );
  ctx.check(
    "the set-up truck pays from its stock for what it restores",
    truck.stock < start.stock && unit(o, 2).mounts[1].ammo[0] > start.at,
    JSON.stringify({ stock: truck.stock, atgm: unit(o, 2).mounts[1].ammo }),
  );
  ctx.check(
    "casualties are replaced and the fallen stay",
    unit(o, 3).members.length === 8 &&
      JSON.stringify(o.corpses.filter((c) => c.own)).startsWith(start.fallen.slice(0, -1)),
    JSON.stringify({ members: unit(o, 3).members.length, corpses: o.corpses.length }),
  );
  ctx.check(
    "the scouts beside the empty truck wait for stock",
    unit(o, 4).stock === 0 && unit(o, 5).service === "no_stock",
    JSON.stringify({ stock: unit(o, 4).stock, scouts: unit(o, 5).service }),
  );

  // A unit that moves off waits; the stock never regrows.
  await lab(page, () => window.__lab.route.demo("Tank: move off"));
  // A vehicle turns on the spot first (still served), then drives.
  await advance(page, 75);
  await select(page, [0, 1]);
  await frame(ctx, page, "waiting", [230, 170]);
  o = await obs(page);
  rows = await supplyRows(page);
  ctx.check(
    "a moving recipient has no redundant supply row",
    unit(o, 1).service === "moving"
      ? rowOf(rows, 1)?.row === null
      : unit(o, 1).service === "out_of_range" && rowOf(rows, 1)?.row === null,
    JSON.stringify({ service: unit(o, 1).service, rows }),
  );
  const before = unit(o, 0).stock;
  await lab(page, () => window.__lab.route.demo("Relocate the truck"));
  await advance(page, 120);
  o = await obs(page);
  ctx.check(
    "a packing, moving truck serves nobody and its stock stays put",
    unit(o, 0).stock === before && o.own.every((u) => u.stock !== null || u.service !== "serving"),
    JSON.stringify({ stock: unit(o, 0).stock, before, services: o.own.map((u) => u.service) }),
  );
  await frame(ctx, page, "relocating", [250, 230]);

  await runHeli(ctx);
  await runHeliFire(ctx);
  await runHeliGround(ctx);
  await runHeliShift(ctx);
  await runHeliCrowded(ctx);
}

async function runHeliShift(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "heli-shift");
  const page = await openBattle(ctx, { url: url.href });
  let o = await obs(page);
  for (let waited = 0; waited < 1200; waited += 15) {
    await advance(page, 15);
    o = await obs(page);
    if (unit(o, 0)?.deployment?.progress === 1) break;
  }
  const heli = unit(o, 0);
  const prop = o.knownProps.find((p) => p.kind === "crate");
  const d = Math.hypot(heli.position[0] - 200, heli.position[1] - 200);
  ctx.check(
    "heli-shift: the landed spot is clear and within rotor reach of the order",
    heli?.deployment?.progress === 1 && d <= 40 && d > 8,
    JSON.stringify({ heli, prop, distance: d }),
  );
  await frame(ctx, page, "heli-shift", [heli.position[0], heli.position[1]]);
  await page.close();
}

async function runHeliCrowded(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "heli-crowded");
  const page = await openBattle(ctx, { url: url.href });
  await advance(page, 300);
  let o = await obs(page);
  ctx.check(
    "heli-crowded: no landing zone is shown and the heli never lands",
    unit(o, 0)?.state === "no_landing_spot" && unit(o, 0)?.deployment?.progress === 0,
    JSON.stringify(unit(o, 0)),
  );
  await lab(page, () => window.__lab.route.select?.([0]));
  await frame(ctx, page, "heli-crowded-hover", [200, 200]);
  await lab(page, () => window.__lab.route.demo("Heli: fly on"));
  for (let waited = 0; waited < 1500; waited += 15) {
    await advance(page, 15);
    o = await obs(page);
    if (unit(o, 0)?.deployment?.progress === 1) break;
  }
  ctx.check(
    "heli-crowded: an order out of the ring lets it land",
    unit(o, 0)?.deployment?.progress === 1,
    JSON.stringify(unit(o, 0)),
  );
  await frame(ctx, page, "heli-crowded", [unit(o, 0).position[0], unit(o, 0).position[1]]);
  await page.close();
}

/** The `?variant=heli` supply route: the same recipients with a landed,
 * packing aircraft source. This keeps the first helicopter checkpoint beside
 * the truck's existing browser contract. */
async function runHeli(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "heli");
  const page = await openBattle(ctx, { url: url.href });
  let o = await obs(page);
  ctx.check(
    "the heli variant fields a supply helicopter",
    unit(o, 0)?.kind === "test_supply_heli",
    JSON.stringify(unit(o, 0)),
  );
  // This isolated encounter has one aircraft and no combat: the only
  // geometry standing in the overlay pass is its dotted drop line.
  const dropLines = async () => {
    const png = await overlaysOnly(page);
    let lit = 0;
    for (let i = 0; i < png.data.length; i += 4)
      if (png.data[i] + png.data[i + 1] + png.data[i + 2] > 120) lit++;
    return { lines: lit > 0 ? 1 : 0, lit };
  };
  let airMarks = null;
  // It flies in, brakes, then drops; sample until it is set up.
  let sawDrop = false;
  let sawTouchdown = false;
  let sawHalf = false;
  const deploy = unitType("test_supply_heli").capabilities.deploy;
  const touchdownProgress = deploy.drop_seconds / deploy.seconds;
  const halfSetup = (1 + touchdownProgress) / 2;
  for (let waited = 0; waited < 900; waited += 15) {
    await advance(page, 15);
    o = await obs(page);
    const heli = unit(o, 0);
    if (!sawDrop && heli.deployment.progress > 0 && heli.deployment.progress < touchdownProgress) {
      sawDrop = true;
      await frame(ctx, page, "heli-drop", heli.position);
      airMarks = await dropLines();
      await frame(ctx, page, "marks-air", heli.position);
      ctx.check(
        "before touchdown the heli has a drop line",
        airMarks.lines === 1,
        JSON.stringify(airMarks),
      );
      const first = decode(await page.screenshot());
      const mast = await lab(page, (p) => window.__lab.projectToCss(...p), [
        heli.position[0],
        heli.position[1],
        heli.position[2] + 3,
      ]);
      await advance(page, 1);
      await presented(page);
      const moving = mostChanged(first, decode(await page.screenshot()), mast, 30);
      ctx.check(
        "the flying heli changes pixels round its mast between ticks",
        moving > 0,
        JSON.stringify({ moving }),
      );
    }
    if (!sawTouchdown && heli.grounded) {
      sawTouchdown = true;
      await frame(ctx, page, "heli-touchdown", heli.position);
      const landedMarks = await dropLines();
      ctx.check(
        "at touchdown the drop-line count falls from one to zero",
        airMarks?.lines === 1 && landedMarks.lines === 0,
        JSON.stringify({ before: airMarks, after: landedMarks }),
      );
      await select(page, [heli.id]);
      await frame(ctx, page, "marks-landed", heli.position);
      await select(page, []);
    }
    if (
      !sawHalf &&
      heli.grounded &&
      heli.deployment.progress >= halfSetup &&
      heli.deployment.progress < 1
    ) {
      sawHalf = true;
      await frame(ctx, page, "heli-half-set-up", heli.position);
    }
    if (heli.deployment.progress === 1 && unit(o, 1).service === "serving") break;
  }
  ctx.check("the heli capture visits touchdown and half set-up", sawTouchdown && sawHalf);
  await frame(ctx, page, "heli-set-up", unit(o, 0).position);
  const stopped = decode(await page.screenshot());
  const heliMast = await lab(page, (p) => window.__lab.projectToCss(...p), [
    unit(o, 0).position[0],
    unit(o, 0).position[1],
    unit(o, 0).position[2] + 3,
  ]);
  await advance(page, 1);
  await presented(page);
  const still = mostChanged(stopped, decode(await page.screenshot()), heliMast, 30);
  ctx.check(
    "the set-up heli holds the pixels round its mast between ticks",
    still === 0,
    JSON.stringify({ still }),
  );
  await frame(ctx, page, "heli-serving", [unit(o, 0).position[0], unit(o, 0).position[1]]);
  ctx.check(
    "the supply helicopter lands and serves",
    unit(o, 0)?.deployment?.progress === 1 && unit(o, 1)?.service === "serving",
    JSON.stringify({ heli: unit(o, 0), recipient: unit(o, 1) }),
  );
  await lab(page, () => window.__lab.route.demo("Heli: lift off"));
  await advance(page, 30);
  o = await obs(page);
  ctx.check(
    "lift off starts packing on the ground",
    unit(o, 0)?.deployment?.target === "packed",
    JSON.stringify(unit(o, 0)?.deployment),
  );
  const before = unit(o, 0).position;
  await lab(page, () => window.__lab.route.demo("Heli: fly on"));
  await advance(page, 120);
  o = await obs(page);
  const after = unit(o, 0).position;
  ctx.check(
    "the packed heli climbs and flies on",
    unit(o, 0)?.deployment?.progress === 0 && after[2] > before[2] && after[0] > before[0],
    JSON.stringify({ before, after, deployment: unit(o, 0)?.deployment }),
  );
  await page.close();
}

/** The `?variant=heli-fire` route: the tank's ground gun opens only after
 * touchdown, then the AA team takes over when the helicopter lifts. */
async function runHeliFire(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "heli-fire");
  const page = await openBattle(ctx, { url: url.href });
  await advance(page, 180);
  let o = await obs(page);
  const tank = seen(o, 1);
  const aa = seen(o, 2);
  ctx.check(
    "heli-fire: the tank fires at the landed heli while AA holds",
    (tank?.weaponPoses ?? []).some((p) => p.shots > 0) &&
      !(aa?.weaponPoses ?? []).some((p) => p.shots > 0),
    JSON.stringify({ tank, aa }),
  );
  await frame(ctx, page, "heli-fire-landed", [180, 200]);
  const shotsOf = (u) => (u?.weaponPoses ?? []).reduce((n, p) => n + p.shots, 0);
  // Catalog mount identity stays stable if its mount order changes.
  const mainGun = unitType("test_tank").mounts.findIndex((m) => m.name === "Main Gun");
  if (mainGun < 0) throw new Error("test tank has no main gun");
  const gunShots = (u) => u?.weaponPoses?.find((p) => p.mount === mainGun)?.shots ?? 0;
  const tankGunBefore = gunShots(seen(o, 1));
  const aaBefore = shotsOf(seen(o, 2));
  await lab(page, () => window.__lab.route.demo("Heli: lift off"));
  // Packing takes its pack time on the ground; then it is an aircraft again.
  await advance(page, 140);
  o = await obs(page);
  ctx.check(
    "heli-fire: once it lifts, the AA team fires and the tank's gun stops",
    unit(o, 0)?.position[2] > 1 &&
      shotsOf(seen(o, 2)) > aaBefore &&
      gunShots(seen(o, 1)) === tankGunBefore,
    JSON.stringify({
      heli: unit(o, 0)?.position,
      tankGunBefore,
      tankGun: gunShots(seen(o, 1)),
      aaBefore,
      aa: shotsOf(seen(o, 2)),
    }),
  );
  await frame(ctx, page, "heli-fire-airborne", [180, 200]);
  await page.close();
}

/** The `?variant=heli-ground` route: a jeep detours around the landed
 * helicopter, while the observation records a ground wreck and a crash. */
async function runHeliGround(ctx) {
  const url = new URL(ctx.url);
  url.searchParams.set("variant", "heli-ground");
  const page = await openBattle(ctx, { url: url.href });
  // The jeep drives the lane the heli landed on; it must go round the hull,
  // never through it (the heli lands facing along the lane).
  const half = unitType("test_supply_heli").body.hull.half_extents_m;
  let overlapped = null;
  let passed = false;
  let groundAt = null;
  let wrecked = null;
  let crashed = null;
  for (let t = 0; t < 3600 && (!wrecked || !crashed || !passed); t += 5) {
    await advance(page, 5);
    const o = await obs(page);
    const heli = unit(o, 0);
    const jeep = unit(o, 1);
    if (heli?.grounded) groundAt = heli.position;
    if (heli && jeep && heli.grounded) {
      const x = jeep.position[0] - heli.position[0];
      const y = jeep.position[1] - heli.position[1];
      const dx = Math.abs(x * Math.cos(heli.yaw) + y * Math.sin(heli.yaw));
      const dy = Math.abs(-x * Math.sin(heli.yaw) + y * Math.cos(heli.yaw));
      // The jeep's centre never enters the heli's hull.
      if (dx < half[0] && dy < half[1]) overlapped ??= { heli: heli.position, jeep: jeep.position };
      if (jeep.position[0] > heli.position[0] + 20) passed = true;
    }
    wrecked ||=
      !heli &&
      groundAt &&
      o.knownProps.some(
        (p) =>
          p.wreckOf === "test_supply_heli" &&
          Math.hypot(p.center[0] - groundAt[0], p.center[1] - groundAt[1]) <
            Math.hypot(half[0], half[1]),
      )
        ? o
        : null;
    crashed ||= (o.crashes ?? []).length > 0 ? o : null;
  }
  ctx.check(
    "heli-ground: the jeep drives round the landed heli, never through it",
    passed && !overlapped,
    JSON.stringify({ passed, overlapped }),
  );
  ctx.check(
    "heli-ground: killed on the ground it leaves a wreck, not a crash",
    !!wrecked && !unit(wrecked, 0) && !(wrecked.crashes ?? []).some((c) => c.id === 0),
    JSON.stringify({
      at: groundAt,
      wrecks: wrecked?.knownProps.filter((p) => p.wreckOf),
      crashes: wrecked?.crashes ?? null,
    }),
  );
  ctx.check(
    "heli-ground: killed in the drop it crashes",
    !!crashed && crashed.crashes.some((c) => c.id === 2),
    JSON.stringify({ crashes: crashed?.crashes ?? [] }),
  );
  await frame(ctx, page, "heli-ground-wrecks", [180, 200]);
  await page.close();
}
