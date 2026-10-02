// Scenery: every drawn thing that is not a soldier, a vehicle or a building —
// props, trees, hedgerows, grass. Each scenery kind is a static bundle (one
// mesh per state, four LOD tiers, instanced at draw) whose appearance entry is
// `{unit: "scenery", scenery: "<kind>"}`.
//
// This table is the art side's: a new kind of drawn thing (a grass kind per
// biome plot, a fence) adds one row, the states its art must carry and what
// the simulation knows of it, which the workbench draws beside the model.
// Which kind draws a prop is the prop type's `appearance.drawn_by`, in the
// prop catalog (fixtures/props/), not this table.

/** What the simulation knows of a scenery kind, for the workbench's overlay. */
export type SceneryFootprint =
  /** It draws prop types: those whose `appearance.drawn_by` names the kind
   *  (`propsDrawnBy`), each a box and blocking and sight classes. */
  | { kind: "prop" }
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
  /** The art is blade strips the grass field instances and bends in the
   *  wind: every tier the same blades in one layout (`grass.ts`). */
  blades?: true;
  /** The most triangles each tier may draw, finest first. The kind is
   *  instanced by the hundred, so its tiers are the forest's frame cost
   *  (`budget.tier_triangles`). */
  tier_triangles?: readonly [number, number, number, number];
  /** The one size every appearance of the kind is built to, so species
   *  differ in shape and never in size (`fit.tree_size`): its top, and its
   *  bole's radius at `breast_m` above the foot (that of a round bole of the
   *  same cross-section), each within the fraction `within`. */
  size?: { top_m: number; bole_radius_m: number; breast_m: number; within: number };
}

const prop: SceneryRule = { states: ["default"], footprint: { kind: "prop" } };

/** Every scenery kind. Buildings keep their own unit (`intact` and `ruin`). */
export const SCENERY_KINDS: Record<string, SceneryRule> = {
  wall: prop,
  crate: prop,
  bridge_deck: prop,
  wreck: prop,
  ruin: prop,
  fence: prop,
  sandbags: prop,
  /** One dragon's tooth: a line of them is an anti-tank wall. */
  tooth: prop,
  // Trees and hedgerows carry one state per biome season (summer; winter is
  // the next biome spec). A tree also stands inside the forests' canopy
  // (`fit.canopy`); hedgerows stand only past the map. A tree's tiers are
  // the forest's frame cost: the budget is what a paired run measured to fit
  // (specs/city-maps/slices/C73-tree-skeleton.md). Its size is the common
  // broadleaf's: placement scales it to the forest rule's canopy, where its
  // bole comes out near the simulation's trunk.
  tree: {
    states: ["summer"],
    footprint: { kind: "tree" },
    tier_triangles: [10000, 2500, 500, 80],
    size: { top_m: 11, bole_radius_m: 0.4, breast_m: 1.3, within: 0.05 },
  },
  hedgerow: { states: ["summer"], footprint: { kind: "none" } },
  /** A clump of blades, one per grass kind (meadow, wheat, stubble), in its
   *  season's state like trees: the biome names which grows on each plot kind. */
  grass: { states: ["summer"], footprint: { kind: "none" }, blades: true },
};

/** The prop types a scenery kind (or `building`) draws: those whose
 *  `appearance.drawn_by` names it in the resolved prop catalog, in id order. */
export function propsDrawnBy(
  props: Readonly<Record<string, { appearance: { drawn_by: string } }>>,
  scenery: string,
): string[] {
  return Object.keys(props)
    .filter((id) => props[id].appearance.drawn_by === scenery)
    .sort();
}

/** The states a static appearance must carry, or null for an unknown scenery kind. */
export function requiredStates(
  unit: string,
  scenery: string | undefined,
  buildingStates: readonly string[],
): readonly string[] | null {
  if (unit === "building") return buildingStates;
  return scenery !== undefined && SCENERY_KINDS[scenery] ? SCENERY_KINDS[scenery].states : null;
}
