// Lab scenarios compose a map fixture with the one rules owner (village.json).
import village from "@fixtures/village.json";
import type { Order, SideName } from "@web/battle/sim/protocol";

export interface LabUnit {
  side: SideName;
  kind: "rifle" | "recon" | "at" | "tank" | "supply";
  position: [number, number];
  yaw?: number;
}

export interface LabEvent {
  tick: number;
  add_prop: {
    kind: string;
    center: [number, number];
    yaw: number;
    half_extents: [number, number, number];
  };
}

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
    rules: {
      tick_hz: village.tick_hz,
      movement: village.movement,
      physics: village.physics,
      health: village.health,
      sensors: village.sensors,
      cost_priority: village.cost_priority,
    },
    units,
    events,
    scripts,
  });
}
