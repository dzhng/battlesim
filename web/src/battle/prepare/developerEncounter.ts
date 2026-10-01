/** The developer encounter on a generated map: blue in a column on the road
 *  into the main settlement, red at that settlement's districts. It reads the
 *  plan (the settlement, its districts' anchors, the roads) for where, and
 *  the compiled map for the buildings' entrances and prop ids. It is a fixed,
 *  deterministic stand-in until the recipe planner (C59) exists: nothing here
 *  checks range, cover or fairness. */
import { vec2, type Vec2 } from "math";
import type { EncounterRecipe } from "./protocol";

/** The parts of `mapgen::MapPlan` the encounter reads. */
export interface PlanView {
  size: [number, number];
  surfaces: {
    kind: string;
    shape: { kind: string; points?: [number, number][] };
  }[];
  /** In the map's building order. */
  buildings: { id: string; owner: number; frame: { translation: [number, number, number] } }[];
  /** The main settlement first. */
  settlements: {
    id: string;
    center: [number, number];
    districts: { id: string; anchor: [number, number] }[];
  }[];
}

/** The parts of the compiled `MapDefinition` the encounter reads. */
export interface MapView {
  size: [number, number];
  buildings: {
    geometry: {
      entrances: { position: [number, number, number]; normal: [number, number] }[] | null;
    };
  }[];
}

/** The rule numbers the encounter takes from the battle's rules (the
 *  village's hold and defender policy). */
export interface EncounterRuleView {
  encounter: { hold_s: number; max_assessment_s: number };
  defender_policy: {
    at_attack_range_m: number;
    tank_retreat_hp_fraction: number;
    infantry_retreat_survivor_fraction: number;
  };
}

export interface UnitSetup {
  side: "blue" | "red";
  kind: string;
  position: [number, number];
  yaw: number;
}

/** The scenario's encounter half (`ScenarioDefinition` less map and rules). */
export interface DeveloperEncounter {
  units: UnitSetup[];
  opponent: {
    side: "red";
    /** (unit id, building owner prop id). */
    garrisons: [number, number][];
    at_attack_range_m: number;
    tank_retreat_hp_fraction: number;
    tank_fallback: [number, number];
    infantry_retreat_survivor_fraction: number;
    infantry_fallback: [number, number];
  };
  encounter: {
    attacker: "blue";
    success_zone_center: [number, number];
    success_zone_radius_m: number;
    hold_s: number;
    max_assessment_s: number;
  };
  anchors: { blue: [number, number]; blueYaw: number; town: [number, number] };
}

/** A road counts as entering the map when an end lies this near the edge. */
const EDGE_REACH_M = 1;

const _encounter_delta = vec2.create();

/** The point `along` metres down `points` from its first, and the heading
 *  there; null when the line is shorter. */
function pointAlong(points: readonly Vec2[], along: number): { at: Vec2; yaw: number } | null {
  let left = along;
  for (let i = 0; i + 1 < points.length; i++) {
    const [a, b] = [points[i], points[i + 1]];
    const length = vec2.distance(a, b);
    if (left <= length) {
      vec2.subtract(_encounter_delta, b, a);
      return {
        at: vec2.lerp(vec2.create(), a, b, length > 0 ? left / length : 0),
        yaw: Math.atan2(_encounter_delta[1], _encounter_delta[0]),
      };
    }
    left -= length;
  }
  return null;
}

/** The country road whose end on the map's edge lies nearest `town`, as its
 *  control points from that end inward. */
function roadIn(plan: PlanView, town: Vec2): Vec2[] {
  const [w, h] = plan.size;
  const onEdge = (p: Vec2) =>
    p[0] <= EDGE_REACH_M ||
    p[1] <= EDGE_REACH_M ||
    p[0] >= w - EDGE_REACH_M ||
    p[1] >= h - EDGE_REACH_M;
  let best: Vec2[] | null = null;
  let bestDistance = Infinity;
  for (const surface of plan.surfaces) {
    const points = surface.shape.points;
    if (surface.kind !== "country_road" || surface.shape.kind !== "stroke" || !points) continue;
    for (const inward of [points, [...points].reverse()]) {
      const distance = vec2.squaredDistance(inward[0], town);
      if (onEdge(inward[0]) && distance < bestDistance) {
        best = inward;
        bestDistance = distance;
      }
    }
  }
  if (!best) throw new Error("the plan has no country road that reaches the map's edge");
  return best;
}

export function developerEncounter(
  plan: PlanView,
  map: MapView,
  recipe: EncounterRecipe,
  rules: EncounterRuleView,
): DeveloperEncounter {
  const main = plan.settlements[0];
  if (!main?.districts.length) throw new Error("the plan has no settlement to defend");
  const town: [number, number] = [main.center[0], main.center[1]];

  const road = roadIn(plan, town);
  const { column, edge_inset_m, spacing_m } = recipe.blue;
  const units: UnitSetup[] = column.map((kind, k) => {
    const along = edge_inset_m + (column.length - 1 - k) * spacing_m;
    const placed = pointAlong(road, along);
    if (!placed)
      throw new Error(`the road into the map is shorter than blue's column (${along} m)`);
    return { side: "blue", kind, position: [placed.at[0], placed.at[1]], yaw: placed.yaw };
  });
  const head = units[0];

  // Each district's buildings, nearest its anchor first.
  const districts = main.districts.map((district) =>
    plan.buildings
      .map((building, index) => ({ building, index }))
      .filter(({ building }) => building.id.startsWith(`${district.id}/`))
      .map((b) => ({
        ...b,
        distance: vec2.squaredDistance(
          [b.building.frame.translation[0], b.building.frame.translation[1]],
          district.anchor,
        ),
      }))
      .filter(({ index }) => map.buildings[index].geometry.entrances?.length)
      .sort((a, b) => a.distance - b.distance || a.index - b.index),
  );
  const garrisons: [number, number][] = [];
  recipe.red.forEach((row, i) => {
    // Round the districts in plan order; a district's second row takes its
    // next building out.
    const candidates = districts[i % districts.length];
    const chosen = candidates[Math.floor(i / districts.length)];
    if (!chosen)
      throw new Error(
        `district ${main.districts[i % districts.length].id} has no building with an entrance for red's row ${i}`,
      );
    const door = map.buildings[chosen.index].geometry.entrances![0];
    if (row.garrison) garrisons.push([units.length, chosen.building.owner]);
    units.push({
      side: "red",
      kind: row.kind,
      position: [
        door.position[0] + door.normal[0] * row.standoff_m,
        door.position[1] + door.normal[1] * row.standoff_m,
      ],
      yaw: Math.atan2(door.normal[1], door.normal[0]),
    });
  });

  const policy = rules.defender_policy;
  return {
    units,
    opponent: {
      side: "red",
      garrisons,
      at_attack_range_m: policy.at_attack_range_m,
      tank_retreat_hp_fraction: policy.tank_retreat_hp_fraction,
      tank_fallback: town,
      infantry_retreat_survivor_fraction: policy.infantry_retreat_survivor_fraction,
      infantry_fallback: town,
    },
    encounter: {
      attacker: "blue",
      success_zone_center: town,
      success_zone_radius_m: recipe.zone_radius_m,
      hold_s: rules.encounter.hold_s,
      max_assessment_s: rules.encounter.max_assessment_s,
    },
    anchors: { blue: head.position, blueYaw: head.yaw, town },
  };
}
