// Scenery: every drawn thing that is not a soldier, a vehicle or a building's
// kit: props, trees, hedgerows, grass. Each scenery kind is a static bundle (one
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
  /** States an appearance may carry besides, each a piece of its first
   *  state where it lies in it (a wreck's burnt hull and its thrown turret),
   *  so the battle can move the pieces apart and then draw the whole. A piece
   *  stands off the ground and short of the box; it is held inside the whole. */
  pieces?: readonly string[];
  /** A state an appearance may carry besides, of what the whole threw round
   *  it as it died (a wreck's armour packs, doors, track runs): every mesh
   *  under a `debris_*` node, and no such node in any other state. The battle
   *  draws it only where it watched the death, and lets it fade, so it is
   *  held to this allowance, never to the box: it is presentation, not cover. */
  debris?: DebrisAllowance;
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
  /** The tallest an appearance of the kind may stand, metres: it has no
   *  body, so it must not look like it hides one (`fit.dressing`). */
  top_m?: number;
}

/** How far thrown debris may lie (`SceneryRule.debris`): its state's name,
 *  how far past the box's sides any of it may reach (metres, across the
 *  ground from the nearest point of the box), and how high it may stand.
 *  Loose tripwires against a piece flung absurdly far or a heap tall enough
 *  to read as cover, not targets. */
export interface DebrisAllowance {
  state: string;
  reach_m: number;
  top_m: number;
}

const prop: SceneryRule = { states: ["default"], footprint: { kind: "prop" } };

/** A wreck's pieces: its burnt hull, and the turret a cook-off throws. */
export const WRECK_PIECES = { hull: "hull", turret: "turret" } as const;

/** The nodes a wreck's thrown debris lies under, in its `debris` state. */
export const DEBRIS_NODE = "debris_";

/** Every scenery kind. */
export const SCENERY_KINDS: Record<string, SceneryRule> = {
  wall: prop,
  crate: prop,
  bridge_deck: prop,
  /** A tank's wreck may carry its pieces: a cook-off throws the turret clear
   *  of the hull before it lands where the wreck has it. */
  wreck: {
    ...prop,
    pieces: [WRECK_PIECES.hull, WRECK_PIECES.turret],
    debris: { state: "debris", reach_m: 8, top_m: 1 },
  },
  ruin: prop,
  fence: prop,
  sandbags: prop,
  /** One dragon's tooth: a line of them is an anti-tank wall. */
  tooth: prop,
  // The street's bodies (fixtures/props/city/street.json), one kind per row.
  // A parked car's destroyed state is the wreck's own row.
  parked_car: prop,
  car_wreck: prop,
  jersey_barrier: prop,
  bollard: prop,
  lamp: prop,
  bench: prop,
  bins: prop,
  hydrant: prop,
  utility_box: prop,
  planter: prop,
  bus_shelter: prop,
  heras_fence: prop,
  skip_bin: prop,
  pallet_stack: prop,
  site_cabin: prop,
  traffic_cone: prop,
  road_barrier: prop,
  scaffold: prop,
  scooter: prop,
  // The gardens' bodies (fixtures/props/city/gardens.json), shared by every region.
  garden_shed: prop,
  hedge: prop,
  garden_fence: prop,
  washing_line: prop,
  garden_table: prop,
  // The courts' bodies (fixtures/props/city/courts.json): shared pieces, then
  // each region's own. A region's look of a shared piece (a Paris bench) is an
  // appearance of the shared kind, tagged with its family.
  bike_rack: prop,
  playground_frame: prop,
  swing: prop,
  bike_shed: prop,
  pingpong_table: prop,
  outdoor_gym: prop,
  laundry_poles: prop,
  courtyard_wall: prop,
  iron_railing: prop,
  plinth_railing: prop,
  chainlink_fence: prop,
  basketball_hoop: prop,
  basketball_court: prop,
  garage_row: prop,
  dumpster: prop,
  kiosk: prop,
  petanque_pitch: prop,
  /** The cover a forest floor holds: a fallen trunk, and a boulder. */
  log: prop,
  boulder: prop,
  // Trees and hedgerows carry one state per biome season (summer; winter is
  // the next biome spec). A tree also stands inside the forests' canopy
  // (`fit.canopy`); hedgerows stand past the map, and under the trees of a
  // tree line, a strip of forest sight does not cross. A tree's tiers are
  // the forest's frame cost: the budget is what a paired run measured to fit
  // (specs/done/city-maps/choices.md). Its size is the common
  // broadleaf's: placement scales it to the forest rule's canopy, where its
  // bole comes out near the simulation's trunk.
  tree: {
    states: ["summer"],
    footprint: { kind: "tree" },
    tier_triangles: [10000, 2500, 500, 80],
    size: { top_m: 11, bole_radius_m: 0.4, breast_m: 1.3, within: 0.05 },
  },
  hedgerow: { states: ["summer"], footprint: { kind: "none" } },
  // What grows and lies under a forest's trees with no body of its own
  // (ferns, bushes, saplings, small rocks, fallen branches), scattered by the
  // thousand: under a man's waist, so nothing drawn hides what the forest
  // does not, and a few hundred triangles at most
  // (specs/done/city-maps/choices.md).
  dressing: {
    states: ["summer"],
    footprint: { kind: "none" },
    tier_triangles: [600, 200, 60, 24],
    top_m: 0.9,
  },
  /** A clump of blades, one per grass kind (meadow, wheat, stubble), in its
   *  season's state like trees: the biome names which grows on each plot kind. */
  grass: { states: ["summer"], footprint: { kind: "none" }, blades: true },
};

/** The prop types a scenery kind draws: those whose `appearance.drawn_by`
 *  names it in the resolved prop catalog, in id order. */
export function propsDrawnBy(
  props: Readonly<Record<string, { appearance: { drawn_by: string } }>>,
  scenery: string,
): string[] {
  return Object.keys(props)
    .filter((id) => props[id].appearance.drawn_by === scenery)
    .sort();
}
