// Lab scenarios compose a map fixture with the one rules owner (village.json)
// and the unit catalog.
import village from "@fixtures/village.json";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { Engagement, Order, SideName } from "@web/battle/sim/protocol";

/** The village fixture with the resolved unit catalog: the rules every
 *  scenario runs on, and what the village and endurance builders read. */
export const VILLAGE_RULES = { ...village, catalog: UNITS.documents };

export interface LabUnit {
  side: SideName;
  /** A unit type's catalog id. */
  kind: string;
  position: [number, number];
  yaw?: number;
  engagement?: Engagement;
  /** Starting wear: vehicle hp, fallen soldiers, rounds spent per weapon row. */
  condition?: { hp?: number; casualties?: number; spent?: Record<string, number> };
  /** A supply vehicle's starting stock. */
  stock?: number;
}

export type LabEvent =
  | {
      tick: number;
      add_prop: {
        kind: string;
        center: [number, number];
        yaw: number;
        half_extents: [number, number, number];
      };
    }
  /** Lab emitter: the unit fires, producing a weapon's firing evidence. */
  | { tick: number; fire: { unit: number } }
  /** Lab emitter: a round of the weapon row bursts on the ground, leaving its
   *  craters and scorch; it flies nothing and hurts nobody. */
  | { tick: number; burst: { point: [number, number]; weapon: string } };

export interface LabScript {
  tick: number;
  side: SideName;
  order: Order;
  queued?: boolean;
}

export function labScenario(
  map: unknown,
  units: LabUnit[],
  events: LabEvent[] = [],
  scripts: LabScript[] = [],
): string {
  return JSON.stringify({
    map,
    // The rules read the sections they own from the one fixture and ignore the rest.
    rules: VILLAGE_RULES,
    units,
    events,
    scripts,
  });
}
