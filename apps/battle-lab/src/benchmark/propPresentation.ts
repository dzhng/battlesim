// What a second client needs besides the map to stand a side's buildings as
// this page does (the `native-visual-presentation` manifest's `props`,
// `web/src/battle/benchmark/propManifest.ts`): which map props are each
// building's parts, and what a gutted building's parts become. The props and
// the fallen buildings are drawn by the presentation's rules
// (`crates/presentation/src/static_world`), which both clients run.
import { buildingRemains } from "@packages/scene-assets/src/authority";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { BuildingIndex } from "@packages/battle-renderer/src/models/buildingReferences";
import type { PropsManifest } from "@web/battle/benchmark/propManifest";

/** The `props` section for a side drawing `buildings` under `units`: per
 *  building its parts' map prop ids, ascending, and the gutted shells' kinds. */
export function describeBuildings(buildings: BuildingIndex, units: UnitCatalog): PropsManifest {
  const parts: number[][] = buildings.parts.map(() => []);
  for (const [prop, building] of buildings.partBuilding) parts[building].push(prop);
  return {
    buildings: {
      parts: parts.map((list) => list.sort((a, b) => a - b)),
      shells: [...buildingRemains(units).shells].sort(),
    },
  };
}
