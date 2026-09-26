// Scenery: every drawn thing that is not a soldier, a vehicle or a building —
// props, trees, hedgerows, grass. Each scenery kind is a static bundle (one
// mesh per state, four LOD tiers, instanced at draw) whose appearance entry is
// `{unit: "scenery", scenery: "<kind>"}`.
//
// This table is the extension point. A slice that adds a kind (a grass kind
// per biome plot, a fence) adds one row: the states its art must carry, and
// what the simulation knows of it, which the workbench draws beside the model.
// Nothing else in scene-assets or the workbench changes.

/** What the simulation knows of a scenery kind, for the workbench's overlay. */
export type SceneryFootprint =
  /** A map prop: its box (from the fixture map's props of that kind) and its
   *  blocking and sight classes (from the simulation's `world_layout()`). */
  | { kind: "prop"; prop: string }
  /** A forest's tree: trunk radius and height, canopy height and spacing
   *  (from the fixture map's forests). */
  | { kind: "tree" }
  /** No simulation body: grass and pure decoration. */
  | { kind: "none" };

export interface SceneryRule {
  /** States the art must carry, one GLB each; the first is what an impostor
   *  and a sheet show. */
  states: readonly string[];
  footprint: SceneryFootprint;
}

const prop = (name: string): SceneryRule => ({
  states: ["default"],
  footprint: { kind: "prop", prop: name },
});

/** Every scenery kind. Buildings keep their own unit (`intact` and `ruin`). */
export const SCENERY_KINDS: Record<string, SceneryRule> = {
  wall: prop("wall"),
  crate: prop("crate"),
  trunk: prop("trunk"),
  bridge_deck: prop("bridgedeck"),
  wreck: prop("wreck"),
  ruin: prop("ruin"),
  // Trees and hedgerows carry one state per biome season (summer; winter is
  // the next biome spec). A tree also stands inside the forests' canopy
  // (`fit.canopy`); hedgerows stand only past the map.
  tree: { states: ["summer"], footprint: { kind: "tree" } },
  hedgerow: { states: ["summer"], footprint: { kind: "none" } },
};

/** The states a static appearance must carry, or null for an unknown scenery kind. */
export function requiredStates(
  unit: string,
  scenery: string | undefined,
  buildingStates: readonly string[],
): readonly string[] | null {
  if (unit === "building") return buildingStates;
  return scenery !== undefined && SCENERY_KINDS[scenery] ? SCENERY_KINDS[scenery].states : null;
}
