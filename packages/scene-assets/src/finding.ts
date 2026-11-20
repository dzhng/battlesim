// One validator finding: its codes, the error constructor, and how a
// measurement prints in one.

export type Severity = "error" | "warning";

/** One validation result. `fix` says what to change in the art or the catalog. */
export interface Finding {
  code: FindingCode;
  severity: Severity;
  message: string;
  fix: string;
}

export const FINDING_CODES = [
  // structure
  "structure.unreadable",
  "structure.lfs_pointer",
  "structure.external_buffer",
  "structure.unsupported",
  "structure.root",
  "structure.skin_count",
  "structure.unskinned_mesh",
  "structure.weights",
  "structure.attributes",
  "structure.scale",
  "structure.tier_count",
  "structure.tier_order",
  /** A unit's or wreck's mesh names no tier (`unitArt.ts`). */
  "structure.tier_unsuffixed",
  /** A unit's or wreck's tier draws more than its class's ratio of the one before. */
  "structure.tier_ratio",
  "structure.skeleton",
  "structure.loop_flags",
  "structure.clips",
  "structure.states",
  "structure.scenery_kind",
  "structure.regional_family",
  "structure.paints",
  /** A soldier's faction look is no soldier on its skeleton, or a faction no battle fields. */
  "structure.faction_look",
  "structure.texture",
  "structure.grass",
  // textures
  "texture.size",
  "texture.mips",
  "texture.tangents",
  // materials (`material.ts`): coverage, interior and role metadata
  "material.coverage",
  "material.coverage_source",
  "material.wear",
  "material.interior",
  "material.role",
  /** Rubber that draws lighter than black rubber looks on screen. */
  "material.role_rubber",
  /** Glass that draws lighter, or rougher, than sight glass looks on screen. */
  "material.role_glass",
  // basis
  "basis.ground",
  "basis.forward",
  "basis.up",
  // fit to authority
  "fit.soldier_height",
  /** A corpse that stands taller than a lying body, or sinks into the ground. */
  "fit.lies",
  "fit.eye",
  "fit.muzzle",
  /** A soldier in his aim reference pose not looking level down the sights. */
  "fit.facing",
  /** A shouldered weapon's sight line above the eye: the weapon held up at the brow. */
  "fit.cheek",
  "fit.hull_extents",
  "fit.vehicle_muzzle",
  "fit.deployed_muzzle",
  "fit.muzzle_arc",
  "fit.canopy",
  "fit.tree_size",
  "fit.dressing",
  "fit.footprint",
  "fit.piece",
  /** A wreck's thrown debris lies past its allowance, or its state carries a
   *  part that is not debris. */
  "fit.debris",
  /** A type listing a part must draw that part's hardware nodes. */
  "fit.part_nodes",
  "fit.mount_draw",
  /** A unit type names an appearance the catalog lacks, or of the wrong kind. */
  "fit.type_appearance",
  /** A vehicle names no wreck of its own, or one not its unit's hull. */
  "fit.wreck",
  // frame cost
  /** A tier draws more triangles than its scenery kind or unit class allows. */
  "budget.tier_triangles",
  /** A unit's class has no row of budgets in `UNIT_ART`. */
  "budget.class_row",
  /** A GPU texture array would hold more distinct layers than the device allows. */
  "budget.texture_layers",
  /** A unit's encoded bundle is over its class's bytes. */
  "budget.bundle_bytes",
  /** A unit's bundle carries more distinct textures than its class allows. */
  "budget.unit_textures",
  // required nodes
  "nodes.missing",
  "nodes.hierarchy",
  "nodes.duplicate",
  "nodes.track_properties",
  "nodes.deploy_motion",
  // city kits: a kit's modules, and its bundle's byte budget
  "kit.module",
  "kit.bytes",
  // city template sets (`templateSource.ts`): the source file, its rows, the
  // physical contract, fit to the descriptor, and the catalogue they cover
  "templates.source",
  "templates.kit",
  "templates.row",
  "templates.module",
  "templates.state",
  "templates.physical",
  "templates.fit",
  "templates.catalogue",
  "templates.coverage",
] as const;
export type FindingCode = (typeof FINDING_CODES)[number];

/** An error-severity finding. */
export const finding = (code: Finding["code"], message: string, fix: string): Finding => ({
  code,
  severity: "error",
  message,
  fix,
});

/** A measure in metres as a finding prints it, to the millimetre. */
export const fmt = (n: number) => n.toFixed(3);
