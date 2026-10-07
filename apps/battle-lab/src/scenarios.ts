// Lab scenarios compose a resolved map (a saved map of the catalogue, or a
// test's own) with the session catalog's rules (game.json with the
// catalog's documents) and an encounter: units, events and scripted orders.
import type { GameRules } from "@web/battle/catalog/compose";
import type { Engagement, Order, SideName } from "@web/battle/sim/protocol";

/** `rules` with every soldier kind all but unkillable: how a lab keeps a
 *  firefight going so what it shows holds still. */
export function durableSoldiers(rules: GameRules): GameRules {
  const durable = structuredClone(rules);
  for (const doc of durable.catalog as { soldiers?: Record<string, { hp: number }> }[])
    for (const kind of Object.values(doc.soldiers ?? {})) kind.hp = 1.0e6;
  return durable;
}

export interface LabUnit {
  side: SideName;
  /** A unit type's catalog id. */
  kind: string;
  position: [number, number];
  yaw?: number;
  engagement?: Engagement;
  /** Starting condition: vehicle hp, each soldier's hp, fallen soldiers,
   *  rounds spent per weapon row, a squad's hidden suppression level. */
  condition?: {
    hp?: number;
    soldier_hp?: number;
    casualties?: number;
    spent?: Record<string, number>;
    suppression?: number;
  };
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

/** A lab's saved encounter (`fixtures/maps/<id>/encounters/<name>.json`). */
export interface LabEncounter {
  units: LabUnit[];
  events: LabEvent[];
  scripts: LabScript[];
}
