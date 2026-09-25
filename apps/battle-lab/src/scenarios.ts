// Lab scenarios compose a map fixture with the one rules owner (village.json).
import village from "@fixtures/village.json";
import type { SideName } from "@web/battle/sim/protocol";

export interface LabUnit {
  side: SideName;
  kind: "rifle" | "recon" | "at" | "tank" | "supply";
  position: [number, number];
  yaw?: number;
}

export function labScenario(map: unknown, units: LabUnit[]): string {
  return JSON.stringify({
    map,
    rules: { tick_hz: village.tick_hz, movement: village.movement },
    units,
  });
}
