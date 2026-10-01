import { GAME_RULES, type LabScript } from "./scenarios";
import { buildStreetScenario } from "./streetScenario";
import type { LabUnit } from "./scenarios";
import type { Wasm } from "@web/battle/sim/module";
import { WEAPONS } from "@packages/scene-assets/src/shippedUnits";

export const REVIEW_LANES = [
  { name: "Rifle", weapon: "rifle", shooter: "rifle", target: "rifle" },
  { name: "HMG", weapon: "hmg", shooter: "jeep", target: "rifle" },
  { name: "Tank AP", weapon: "tank_ap", shooter: "tank", target: "tank" },
  { name: "Tank HE", weapon: "tank_he", shooter: "tank", target: "rifle" },
  { name: "ATGM", weapon: "atgm", shooter: "at", target: "tank" },
] as const;

export function reviewLanePositions(index: number): {
  from: [number, number];
  to: [number, number];
} {
  const lane = REVIEW_LANES[index];
  const y = 80 + index * 75;
  const range = WEAPONS[lane.weapon].range_m;
  return { from: [100, y], to: [100 + range - 10, y] };
}

/** Private experiment controls; gameplay keeps the authored rules. */
export function projectileReviewRules() {
  const rules = structuredClone(GAME_RULES);
  for (const weapon of Object.values(rules.weapons)) {
    Object.assign(weapon, { ammo: "unlimited", damage: 0, structural_damage: 0 });
  }
  return rules;
}

export function buildProjectileReview(wasm: Pick<Wasm, "village_scenario">) {
  const s = JSON.parse(buildStreetScenario(wasm, projectileReviewRules())) as {
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
    const { from, to } = reviewLanePositions(i);
    s.units.push(
      { side: "blue", kind: lane.shooter, position: from, engagement: "return_fire_only" },
      {
        side: "red",
        kind: lane.target,
        position: to,
        yaw: Math.PI,
        engagement: "return_fire_only",
      },
      { side: "blue", kind: "recon", position: [(from[0] + to[0]) / 2, from[1] + 20] },
    );
  }
  return JSON.stringify(s);
}
