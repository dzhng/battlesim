// HighAir motion and the vision boundary (slice 02): an F-16 flies across
// the air field over the village, the tower and the wood. Blue's fog never
// clears (HighAir never reveals Ground) and the T-72 in the village is never
// identified, though the jet identifies the enemy Mi-8 (LowAir) and J-10
// (HighAir); it holds the high-air cruise height over the ground, stays on
// the map, and reaches its goal, which it circles rather than stops on.
// The simulation's tests behind these claims:
//   crates/sim/tests/air.rs::high_air_has_no_ground_vision_but_can_identify_low_air
//   crates/sim/tests/fixed_wing_flight.rs::a_jet_holds_cruise_over_rolling_ground_without_steps
//   crates/sim/tests/fixed_wing_flight.rs::a_move_goal_is_circled_at_speed_over_turn_rate
//   crates/sim/tests/fixed_wing_flight.rs::a_circle_by_the_edge_turns_back_inside_the_square
// Browser-only evidence: the same flight on the real field and catalog, and
// a shot of the jet over the wood at its goal.
import { gpuWarnings, aim } from "./_lab.mjs";
import { writeFile } from "node:fs/promises";
import { decode } from "./_png.mjs";
import { unitType } from "./_units.mjs";
import {
  encounter,
  openAir,
  play,
  own,
  onMap,
  surface,
  HIGH_AIR_CRUISE,
  shoot,
  visibleCells,
  field,
} from "./_air.mjs";

const [JET, TANK, HELI, ENEMY_JET] = encounter("high-air", "air-high").units.map((u) => u.kind);
const goal = encounter("high-air", "air-high").scripts[0].order.goal;
const FIELD_SIZE = field("air-high");

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let fog = 0;
  let tankSeen = false;
  const seen = new Set();
  const heights = [];
  let offMap = null;
  let k = 0;
  // A jet never stops: it has reached its goal once it is within one turning
  // radius of it (cruise speed over turn rate), where it starts to circle.
  const flight = unitType(JET).mobility.air;
  const radius = flight.cruise_kmh / 3.6 / ((flight.turn_deg_s * Math.PI) / 180);
  const near = (p) => Math.hypot(p[0] - goal[0], p[1] - goal[1]) <= radius + 10;
  // Until the jet reaches its goal across the field, with a generous cap.
  const last = await play(page, 3600, (o) => {
    fog = Math.max(fog, visibleCells(o));
    for (const u of o.identified) seen.add(u.kind);
    if (o.identified.some((u) => u.kind === TANK)) tankSeen = true;
    const jet = own(o, JET)[0];
    if (jet && !onMap(jet.position, FIELD_SIZE)) offMap = jet.position;
    if (jet && k++ % 30 === 0) heights.push(jet.position);
    return !!jet && near(jet.position);
  });
  ctx.check("blue's fog never clears while only the jet flies", fog === 0, `${fog} cells`);
  ctx.check("the jet never identifies the tank on the ground", !tankSeen);
  ctx.check(
    "the jet identifies the enemy aircraft in both air layers",
    seen.has(HELI) && seen.has(ENEMY_JET),
    JSON.stringify([...seen]),
  );
  ctx.check("the jet stays on the map", offMap === null, JSON.stringify(offMap));
  const lifts = [];
  for (const p of heights) lifts.push(p[2] - (await surface(page, p)));
  ctx.check(
    "it holds the air height over the ground all the way",
    lifts.length > 5 && lifts.every((l) => Math.abs(l - HIGH_AIR_CRUISE) < 3),
    JSON.stringify(lifts.map((l) => l.toFixed(1))),
  );
  const jet = own(last, JET)[0];
  ctx.check(
    "it reaches its goal past the village, within one turning radius",
    !!jet && near(jet.position),
    JSON.stringify(jet?.position),
  );
  // The jet over the wood at its goal, the village and the tower behind it.
  if (jet)
    await aim(
      page,
      jet.position,
      { distance: 120, pitch: 0.3, yaw: -Math.PI / 2 - 0.5 },
      { onGround: false },
    );
  await shoot(ctx, page, "jet-over-the-village");
  const stats = await page.evaluate(() => window.__lab.stats());
  ctx.check(
    "the high-air draw uses the jet's admitted authored geometry",
    stats.models.installed.includes(unitType(JET).appearance) &&
      stats.submitted.highColor.modelsIncludingProps.triangles > 0,
    JSON.stringify({
      appearance: unitType(JET).appearance,
      admitted: stats.models.installed.includes(unitType(JET).appearance),
      highColor: stats.submitted.highColor.modelsIncludingProps,
    }),
  );
  const plane = await page.evaluate(() => {
    const host = document.querySelector("[data-high-plane]");
    const canvas = host?.querySelector("canvas");
    return {
      image: canvas?.toDataURL(),
      betweenCards:
        host?.previousElementSibling?.classList.contains("ro-plane") &&
        host?.nextElementSibling?.hasAttribute("data-high-air"),
    };
  });
  ctx.check("high-air output composes between ground and high-air readouts", plane.betweenCards);
  let covered = 0;
  let transparent = 0;
  if (plane.image) {
    const bytes = Buffer.from(plane.image.split(",")[1], "base64");
    await writeFile(ctx.evidencePath("high-air-plane.png"), bytes);
    const image = decode(bytes);
    for (let i = 3; i < image.data.length; i += 4) {
      if (image.data[i]) covered++;
      else transparent++;
    }
  }
  ctx.check(
    "the separate browser output contains the jet on transparent background",
    covered > 0 && transparent > 0,
    JSON.stringify({ covered, transparent }),
  );
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
