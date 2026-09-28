/** The range ruler (Space held with a selection): a line on the ground from
 *  the selected unit nearest the cursor to the cursor's ground point, its
 *  length, and where each of that unit's weapons stops reaching. Built from
 *  the own unit (its drawn position), the unit catalog and the weapon rows
 *  only.
 *
 *  The distance is measured the way the simulation checks a weapon's range
 *  (`weapons.rs` `engage`: `(target − muzzle).length() > range_m`): in 3D,
 *  from the unit's muzzle to its aim point. Here the muzzle is the unit's
 *  position lifted to its first mount's muzzle height (a hand weapon's is
 *  `infantry_muzzle_m`), and the aim point is the ground under the cursor
 *  lifted to `infantry_aim_m`, where the simulation aims at a soldier or a
 *  contact. A mount's muzzle also sits a few metres out along its bearing (a
 *  tank's gun 6 m); the ruler leaves that out, so it errs short. */
import { vec3 } from "math";
import type { UnitCatalog } from "@packages/scene-assets/src/units";

type Point2 = readonly [number, number];
type Point3 = readonly [number, number, number];

/** What the ruler reads of the scenario's rules: each weapon row's display
 *  name and range, and where a soldier's muzzle and aim point stand. */
export interface RulerRules {
  weapons: Record<string, { name: string; range_m: number }>;
  physics: { infantry_muzzle_m: number; infantry_aim_m: number };
}

/** One reach on the ruler: the weapon rows of the unit that share a range. */
export interface RulerMark {
  /** The weapons' names (the callouts' captions), in mount order. */
  names: string[];
  range_m: number;
  /** The cursor is within this range. */
  inRange: boolean;
  /** Where along the ground line (metres from the unit) the reach ends, or
   *  null when it ends past the cursor (the weapon reaches it). */
  along_m: number | null;
}

export interface RangeRuler {
  unit: number;
  /** The unit's drawn foot and the cursor's ground point: the line. */
  from: Point3;
  to: Point3;
  /** Muzzle to aim point, as the range check measures it. */
  distance_m: number;
  /** Nearest reach first. Empty for an unarmed unit. */
  marks: RulerMark[];
}

/** The unit in `units` whose position is nearest `point` across the ground. */
export function closestUnit<U extends { position: Point3 }>(
  units: readonly U[],
  point: Point2,
): U | null {
  let best: U | null = null,
    bestD2 = Infinity;
  for (const u of units) {
    const dx = u.position[0] - point[0],
      dy = u.position[1] - point[1];
    const d2 = dx * dx + dy * dy;
    if (d2 < bestD2) {
      bestD2 = d2;
      best = u;
    }
  }
  return best;
}

const _ruler_muzzle = vec3.create();
const _ruler_aim = vec3.create();

/** The ruler from `unit` (at its drawn position) to the ground point `cursor`. */
export function rangeRuler(
  unit: { id: number; kind: string; position: Point3 },
  cursor: Point3,
  rules: RulerRules,
  units: UnitCatalog,
): RangeRuler {
  const mounts = units.type(unit.kind).mounts;
  const first = mounts[0];
  const muzzleZ =
    first?.muzzle_m != null
      ? first.pivot_m[2] + first.muzzle_m[2]
      : rules.physics.infantry_muzzle_m;
  const [x, y, z] = unit.position;
  vec3.set(_ruler_muzzle, x, y, z + muzzleZ);
  vec3.set(_ruler_aim, cursor[0], cursor[1], cursor[2] + rules.physics.infantry_aim_m);
  const distance = vec3.distance(_ruler_muzzle, _ruler_aim);
  const ground = Math.hypot(cursor[0] - x, cursor[1] - y);
  // Weapons sharing a range are one reach. A weapon goes by the callouts'
  // caption: its one row's name, or for a mount of several rows (a cannon's
  // AP and HE) the mount's own.
  const byRange = new Map<number, string[]>();
  for (const m of mounts)
    for (const row of m.weapons) {
      const w = rules.weapons[row];
      if (!w) continue;
      const name = m.weapons.length === 1 ? w.name : m.name;
      const names = byRange.get(w.range_m) ?? [];
      if (!names.includes(name)) names.push(name);
      byRange.set(w.range_m, names);
    }
  const marks = [...byRange]
    .sort(([a], [b]) => a - b)
    .map(([range, names]): RulerMark => {
      const inRange = distance <= range;
      // The muzzle-to-aim segment runs straight over the ground line, so the
      // share of it a range covers is the same share of the ground line.
      return {
        names,
        range_m: range,
        inRange,
        along_m: inRange ? null : (ground * range) / distance,
      };
    });
  return {
    unit: unit.id,
    from: unit.position,
    to: cursor,
    distance_m: distance,
    marks,
  };
}
