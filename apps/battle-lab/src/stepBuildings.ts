// The building each garrison or exit step of an own unit's orders enters or
// leaves, so the order chain can ring it as a held building is ringed: the
// unit's hold for the one it is in, else the map's parts of the garrison
// step's host (the map's buildings are known to both sides).
import type { BuildingIndex } from "@packages/battle-renderer/src/models/buildingReferences";
import type { OrderStep } from "@packages/battle-renderer/src/orderSteps";
import type { OwnUnitView } from "@web/battle/sim/observationView";

type P2 = readonly [number, number];
type Footprint = NonNullable<OrderStep["building"]>;
export type StepBuildingIndex = Pick<BuildingIndex, "partBuilding" | "parts">;

/** The bounds round every corner of the parts of the building prop `id`
 *  belongs to; null when it is no map building. */
function footprint(id: number, buildings: StepBuildingIndex): Footprint | null {
  const index = buildings.partBuilding.get(id);
  if (index === undefined) return null;
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const p of buildings.parts[index]) {
    const [c, s] = [Math.cos(p.yaw), Math.sin(p.yaw)];
    const dx = Math.abs(p.half[0] * c) + Math.abs(p.half[1] * s);
    const dy = Math.abs(p.half[0] * s) + Math.abs(p.half[1] * c);
    [x0, y0] = [Math.min(x0, p.center[0] - dx), Math.min(y0, p.center[1] - dy)];
    [x1, y1] = [Math.max(x1, p.center[0] + dx), Math.max(y1, p.center[1] + dy)];
  }
  if (x0 > x1) return null;
  const center: P2 = [(x0 + x1) / 2, (y0 + y1) / 2];
  return { center, half: [(x1 - x0) / 2, (y1 - y0) / 2] };
}

/** `unit`'s steps, each garrison and exit step with its building when
 *  `buildings` places it. */
export function stepBuildings(
  unit: OwnUnitView,
  buildings: StepBuildingIndex | null,
): readonly OrderStep[] {
  if (!buildings) return unit.steps;
  const host = unit.hold?.host;
  let inside: Footprint | null =
    host?.kind === "building" ? { center: host.center, half: host.half } : null;
  return unit.steps.map((step) => {
    if (step.kind === "garrison" && step.host) {
      inside = footprint(step.host.id, buildings);
      return inside ? { ...step, building: inside } : step;
    }
    if (step.kind !== "exit") return step;
    const building = inside;
    inside = null;
    return building ? { ...step, building } : step;
  });
}
