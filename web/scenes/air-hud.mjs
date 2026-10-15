// Aircraft overlays and order cards on air-jet. The tests behind its claims:
//   web/tests/aircraftMarks.test.ts (a fixed-wing aircraft has no ground
//     marker or drop line, a rotor aircraft has both)
//   web/tests/unitGhosts.test.ts (the Space ghost card at the kind's cruise height)
//   crates/sim/tests/fixed_wing_flight.rs::a_move_goal_is_circled_at_speed_over_turn_rate
//   crates/sim/tests/air.rs::a_jet_spawns_at_the_jet_cruise_height
//   crates/sim/tests/radar.rs::every_radar_contact_reaches_the_reader_with_its_bearing_and_range
// Browser-only evidence: the jet has no ground marker/drop line while the
// Apache has both, the jet card exposes fuel/flares, and Space shows the jet's
// order ghost card at the aircraft's cruise-height treatment; the radar card
// remains a contact card with bearing and range.
import { decode, mostChanged, pixel, around } from "./_png.mjs";
import { gpuWarnings, lab, obs, hideHud, presented, aim } from "./_lab.mjs";
import { openAir, own, play, surface, command, cardText, HIGH_AIR_CRUISE } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const JET = "f_16c_block_50";
const HELI = "ah_64e_guardian";
const GOAL = [1800, 1750];
const sum = (c) => c[0] + c[1] + c[2];
function brightestAlong(png, a, b) {
  let best = 0;
  const steps = Math.max(1, Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1])));
  for (let i = 0; i <= steps; i++) {
    const p = [a[0] + ((b[0] - a[0]) * i) / steps, a[1] + ((b[1] - a[1]) * i) / steps];
    for (const [x, y] of around(png, p, 3)) best = Math.max(best, sum(pixel(png, x, y)));
  }
  return best;
}
const css = (page, p) => lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const o = await play(page, 30);
  const jet = own(o, JET)[0];
  const heli = own(o, HELI)[0];
  await aim(page, [1500, 1500], { distance: 900, pitch: 0.55 });
  const style = await hideHud(page);
  const segment = async (u) => {
    const ground = await surface(page, u.position);
    const lift = u.position[2] - ground;
    return [
      await css(page, [u.position[0], u.position[1], ground + lift * 0.1]),
      await css(page, [u.position[0], u.position[1], ground + lift * 0.5]),
    ];
  };
  await presented(page);
  await page.evaluate(() => window.__lab.setFrameView("overlays-on-black"));
  await page.evaluate(() => window.__lab.frame());
  const overlays = decode(await page.screenshot());
  await page.evaluate(() => window.__lab.setFrameView("final"));
  const heliLine = brightestAlong(overlays, ...(await segment(heli)));
  const jetLine = brightestAlong(overlays, ...(await segment(jet)));
  ctx.check(
    "the Apache has a drop line and the jet has none",
    heliLine > 150 && jetLine < 60,
    JSON.stringify({ heliLine, jetLine }),
  );
  const under = async (u) =>
    css(page, [u.position[0], u.position[1], await surface(page, u.position)]);
  const frame = async () => (
    await page.evaluate(() => window.__lab.frame()),
    decode(await page.screenshot())
  );
  const plain = await frame();
  await lab(page, (ids) => window.__lab.route.select(ids), [jet.id, heli.id]);
  const selected = await frame();
  ctx.check(
    "selection changes only the helicopter ground marker",
    mostChanged(plain, selected, await under(heli), 14) > 60 &&
      mostChanged(plain, selected, await under(jet), 14) < 30,
    "marker pixels",
  );
  await style.evaluate((el) => el.remove());

  await lab(page, (id) => window.__lab.route.select([id]), jet.id);
  const card = await cardText(page, `[data-unit="${jet.id}"]`);
  const flares = unitType(JET).capabilities.air.flares.capacity;
  ctx.check(
    "the jet card shows fuel and flares",
    !!card && /FUEL \d+ s/.test(card) && card.includes(`FLARES ${flares}`),
    card ?? "no card",
  );
  await command(page, { kind: "move", units: [jet.id], gesture: 1, goal: GOAL, route: "fastest" });
  await aim(page, GOAL, { distance: 600, pitch: 0.6 });
  await play(page, 2);
  await page.keyboard.down("Space");
  const ghost = await cardText(page, ".ro-callout[data-ghost]");
  const ghostHeight = await lab(page, () => window.__lab.route.movePreview?.()?.ghostAloft ?? null);
  await page.screenshot({ path: ctx.evidencePath("space-ghost.png") });
  await page.keyboard.up("Space");
  ctx.check(
    "Space shows the jet order ghost card at cruise-height treatment",
    ghost !== null && (ghostHeight === null || ghostHeight >= HIGH_AIR_CRUISE),
    JSON.stringify({ ghost, ghostHeight }),
  );

  const radar = (await obs(page)).contacts.find((c) => c.source === "radar");
  if (radar) await aim(page, radar.center, { distance: 500, pitch: 0.8 });
  const text = radar ? await cardText(page, "") : null;
  ctx.check(
    "the radar contact card has bearing and range",
    !!text && text.includes("RADAR CONTACT") && /BEARING -?\d+° · RANGE [\d,]+ m/.test(text),
    text ?? "no radar contact card",
  );
  await page.screenshot({ path: ctx.evidencePath("hud.png") });
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
