import type { LabScript, LabUnit } from "./scenarios";
import { buildStreetScenario } from "./streetScenario";
import type { Wasm } from "@web/battle/sim/module";
import type { GameRules, SessionCatalog } from "@web/battle/catalog/compose";

export const REVIEW_LANES = [
  { name: "Rifle", weapon: "rifle", shooter: "test_rifle", target: "test_rifle" },
  { name: "HMG", weapon: "hmg", shooter: "test_jeep", target: "test_rifle" },
  { name: "Tank AP", weapon: "tank_ap", shooter: "test_tank", target: "test_tank" },
  { name: "Tank HE", weapon: "tank_he", shooter: "test_tank", target: "test_rifle" },
  { name: "ATGM", weapon: "atgm", shooter: "test_at", target: "test_tank" },
  { name: "Grenade", weapon: "grenade", shooter: "test_rifle", target: "test_rifle" },
] as const;

export function reviewLanePositions(
  weapons: SessionCatalog["weapons"],
  index: number,
): {
  from: [number, number];
  to: [number, number];
} {
  const lane = REVIEW_LANES[index];
  const y = 80 + index * 75;
  const range = weapons[lane.weapon].range_m;
  return { from: [100, y], to: [100 + range - 10, y] };
}

/** Private experiment controls; gameplay keeps the authored rules. */
export function projectileReviewRules(game: GameRules): GameRules {
  const rules = structuredClone(game);
  for (const weapon of Object.values(rules.weapons)) {
    Object.assign(weapon, { ammo: "unlimited", damage: 0, structural_damage: 0 });
  }
  return rules;
}

export async function buildProjectileReview(wasm: Wasm, catalog: SessionCatalog) {
  const rules = projectileReviewRules(catalog.rules);
  const s = JSON.parse(await buildStreetScenario(rules)) as {
    units: LabUnit[];
    scripts: LabScript[];
    opponent?: { garrisons: [number, number][] };
  };
  // Preserve the defender's fighting positions, with no retreating commander.
  for (const [unit, building] of s.opponent?.garrisons ?? [])
    s.scripts.push({ tick: 1, side: "red", order: { kind: "garrison", units: [unit], building } });
  delete s.opponent;
  for (let i = 0; i < REVIEW_LANES.length; i++) {
    const lane = REVIEW_LANES[i];
    const { from, to } = reviewLanePositions(catalog.weapons, i);
    s.units.push(
      { side: "blue", kind: lane.shooter, position: from, engagement: "return_fire_only" },
      {
        side: "red",
        kind: lane.target,
        position: to,
        yaw: Math.PI,
        engagement: "return_fire_only",
      },
      { side: "blue", kind: "test_recon", position: [(from[0] + to[0]) / 2, from[1] + 20] },
    );
  }
  return JSON.stringify(s);
}
