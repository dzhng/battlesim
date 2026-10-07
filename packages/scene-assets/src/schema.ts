// The appearance-bundle contract: what a baked bundle holds, what the catalog
// declares, and what each unit type requires of its art. Engine space is Z up,
// +X forward, +Y left, metres, origin on the ground.

import type { Mat4, Vec3 } from "math";
import type { Trs } from "./trs.ts";
import type { MountDraws, UnitCatalog } from "./units.ts";

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
  "structure.texture",
  "structure.grass",
  // textures
  "texture.size",
  "texture.mips",
  "texture.tangents",
  // materials (`material.ts`): coverage and interior metadata
  "material.coverage",
  "material.coverage_source",
  "material.wear",
  "material.interior",
  // basis
  "basis.ground",
  "basis.forward",
  "basis.up",
  // fit to authority
  "fit.soldier_height",
  "fit.eye",
  "fit.muzzle",
  "fit.hull_extents",
  "fit.vehicle_muzzle",
  "fit.muzzle_arc",
  "fit.canopy",
  "fit.tree_size",
  "fit.dressing",
  "fit.footprint",
  /** A scenery piece (a wreck's thrown turret) reaches outside the whole it is part of. */
  "fit.piece",
  /** A type listing a part must draw that part's hardware nodes. */
  "fit.part_nodes",
  "fit.mount_draw",
  /** A unit type names an appearance the catalog lacks, or of the wrong kind. */
  "fit.type_appearance",
  // frame cost
  /** A tier draws more triangles than its scenery kind or unit class allows. */
  "budget.tier_triangles",
  /** A unit's encoded bundle is over its class's bytes. */
  "budget.bundle_bytes",
  /** A unit's bundle carries more textures than its class's layers. */
  "budget.texture_layers",
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
  // physical contract, fit to the descriptor, and the catalogue it covers
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

/** Mesh tiers, finest first. Source meshes carry them as a `_LOD<n>` name suffix. */
export const TIER_COUNT = 4;

export type BundleKind = "skinned" | "articulated" | "static";
/** What an appearance draws. A soldier kind names soldier appearances as its
 *  set and a hull type its vehicle appearance (the unit catalog); "scenery"
 *  is every prop, tree, hedgerow and grass kind, which one the entry's
 *  `scenery` (`scenery.ts`); a "kit" is a city set's shared modules, which
 *  the template art library's rows place on buildings (`templateLibrary.ts`).
 *  A building is never an appearance: it is its template's rows. */
export type AppearanceUnit = "soldier" | "vehicle" | "scenery" | "kit";

/** Whether an appearance is fetched only when something that draws it asks:
 *  a kit, by a map whose buildings place it or a lab that shows it; a
 *  regional look, by a map of its family. Such an appearance travels as gzip,
 *  and what a map asks for of them is its download, held to
 *  `MAP_DOWNLOAD_MAX_BYTES`. Every other appearance loads with the catalog. */
export const fetchedOnRequest = (entry: { unit: AppearanceUnit; regional_family?: string }) =>
  entry.unit === "kit" || entry.regional_family !== undefined;

/** Every appearance of `catalog` fetched on request, by name: the regional
 *  family it is a look of, or null for a kit. */
export const onRequestOf = (catalog: Pick<RuntimeCatalog, "appearances">) =>
  new Map(
    Object.entries(catalog.appearances ?? {})
      .filter(([, entry]) => fetchedOnRequest(entry))
      .map(([name, entry]) => [name, entry.regional_family ?? null]),
  );

/** The regional looks a map of `family` fetches, of those fetched on request
 *  (`onRequestOf`): its own family's, no other's; none for a map of no family. */
export const familyLooks = (
  onRequest: ReadonlyMap<string, string | null>,
  family: string | null,
): string[] =>
  family === null ? [] : [...onRequest].filter(([, of]) => of === family).map(([name]) => name);

export const UNIT_BUNDLE_KIND: Record<AppearanceUnit, BundleKind> = {
  soldier: "skinned",
  vehicle: "articulated",
  scenery: "static",
  kit: "static",
};

/** Clip roles every infantry skeleton carries (spike 03), plus the fit reference pose. */
export const INFANTRY_CLIPS = [
  "idle",
  "walk",
  "run",
  "kneel_fire",
  "prone_pinned",
  "death",
] as const;
/** Skinned sockets: nodes under a joint, named exactly. */
export const INFANTRY_SOCKETS = ["eye", "muzzle"] as const;

/** A merged, material-ranged triangle mesh in its owner's space. */
export interface MeshData {
  positions: Float32Array; // xyz
  normals: Int16Array; // snorm16 xyzw, w = 0
  /** snorm16 xyzw: the UV's +u direction, w the bitangent's sign (glTF
   *  TANGENT). Present when any source primitive had one; a vertex without
   *  one is (0, 0, 0, 0). A normal-mapped material needs it (`texture.tangents`). */
  tangents?: Int16Array;
  uvs: Float32Array; // uv
  colors: Uint8Array; // unorm8 rgba
  joints?: Uint8Array; // skinned only: 4 joint indices per vertex
  weights?: Uint16Array; // skinned only: unorm16, each vertex sums to 65535
  indices: Uint16Array | Uint32Array;
  /** Contiguous index ranges, one per material. */
  draws: { material: number; first: number; count: number }[];
}

/**
 * How much of a surface is there. `opaque` is all of it. A `cutout` is there
 * or not, texel by texel: nothing is drawn where its coverage value is under
 * `cutoff`. A `blended` surface is partly there, and what is behind it shows
 * through.
 *
 * The coverage value is the base colour's alpha times the normal texture's
 * alpha (1 without one), and nothing else. The albedo texture's alpha stays
 * the wear threshold, the ORM texture's the tint mask and the vertex colour's
 * how worn the surface is, whatever the coverage. An opaque material has no
 * coverage value: both alphas are ignored.
 */
export type Coverage =
  | { kind: "opaque" }
  | { kind: "cutout"; cutoff: number }
  | { kind: "blended" };

/** The interior atlas sheets (`blender/city/README.md`, "Interiors"):
 *  apartment rooms on any floor, and shops on ground floors. */
export const INTERIOR_SHEETS = ["rooms", "shops"] as const;
export type InteriorSheet = (typeof INTERIOR_SHEETS)[number];
/** A sheet's cells: how many, their edge, how many to a row in the source
 *  picture, and how many to a row in the square texture a bundle carries
 *  (`interior.ts`); cell `i` is at column `i mod columns`, row
 *  `floor(i / columns)`, from the top-left. And what a cell's picture was
 *  taken of: a room `depth_m` deep, from a pinhole `pinhole_m` before its
 *  open face. */
export const INTERIOR_ATLAS = {
  cells: 10,
  cell_px: 128,
  source_columns: 2,
  columns: 4,
  depth_m: 4.5,
  pinhole_m: 16,
} as const;

export interface Material {
  name: string;
  /** Linear rgb, and the coverage value's factor in alpha (`Coverage`). */
  base_color: [number, number, number, number];
  metallic: number;
  roughness: number;
  /** The tint mask: how much of its instance's tint this surface takes, 0..1
   *  (glTF material extras `tint`), times the ORM texture's alpha where the
   *  material has one: a body's side (blue and red share one mesh), or a
   *  prop's paint (the catalog's `paints`). */
  tint: number;
  /** Baked textures, as indices into the bundle's `textures`. */
  textures?: MaterialTextures;
  /** What the vertex colour is multiplied by before it tints the surface
   *  (glTF extras `colour_scale`, default 1). A textured material's vertex
   *  colour is relative to its albedo texture and stored at a third (3), so dust,
   *  mud and ash can lighten the texture as well as shade it. */
  colour_scale?: number;
  /** The worn surface (linear rgb, roughness) that shows where the vertex
   *  colour's alpha (how worn: chips on edges, mud low down) rises past the
   *  albedo texture's alpha (where it breaks first). glTF extras `wear`. */
  wear?: [number, number, number, number];
  /** glTF `alphaMode` and `alphaCutoff`; opaque when the source names none. */
  coverage: Coverage;
  /** The surface is a wall of the room box behind a window: it shows a cell
   *  of this interior atlas sheet, at its own UVs, in place of a look of its
   *  own. glTF extras `interior`. A baked bundle carries the sheet as this
   *  material's albedo texture (`interior.ts`). */
  interior?: InteriorSheet;
}

/**
 * A material's texture channels. Every one samples the mesh's UVs and is
 * shared by every tier.
 * - `albedo`: sRGB colour; alpha is the wear threshold (low wears first).
 * - `normal`: tangent-space normal, xyz in 0..1 (glTF `normalTexture`); alpha
 *   scales the coverage value of a cutout or blended material (`Coverage`).
 * - `orm`: occlusion, roughness, metalness (glTF's packed occlusion and
 *   metallic-roughness image); alpha multiplies the material's tint mask.
 */
export const TEXTURE_CHANNELS = ["albedo", "normal", "orm"] as const;
export type TextureChannel = (typeof TEXTURE_CHANNELS)[number];
export type MaterialTextures = Partial<Record<TextureChannel, number>>;

/** GPU-ready texel formats a bundle carries (WebGPU names). */
export type TextureFormat = "rgba8unorm-srgb" | "rgba8unorm";
export const CHANNEL_FORMAT: Record<TextureChannel, TextureFormat> = {
  albedo: "rgba8unorm-srgb",
  normal: "rgba8unorm",
  orm: "rgba8unorm",
};
/** Texture edge lengths the validator accepts: square, a power of two. */
export const TEXTURE_MIN_PX = 4;
export const TEXTURE_MAX_PX = 1024;

/**
 * One baked texture: every mip level down to 1×1, finest first, tightly
 * packed rows of RGBA8. `id` is its content address (sha256 of format, size
 * and every level), so a texture shared by materials or bundles is one
 * texture on the GPU.
 */
export interface Texture {
  id: string;
  format: TextureFormat;
  width: number;
  height: number;
  levels: Uint8Array[];
}

export interface Bounds {
  min: Vec3;
  max: Vec3;
}

export interface PoseRef {
  clip: string;
  phase: number; // 0..1 of the clip's duration
}

export interface Joint {
  name: string;
  parent: number; // -1 for a root
  bind: Trs;
  inverse_bind: Mat4; // column-major
}

export interface Socket {
  name: string;
  joint: number;
  offset: Trs; // in the joint's space
}

export interface SkinnedBundle {
  kind: "skinned";
  skeleton: string;
  joints: Joint[]; // in the skeleton clips' joint order
  tiers: MeshData[];
  materials: Material[];
  textures: Texture[];
  bounds: Bounds; // over every clip of the skeleton
  far_pose: PoseRef;
  corpse_pose: PoseRef;
  sockets: Socket[];
}

export interface ArticulatedNode {
  name: string;
  parent: number; // -1 for the root
  pivot: Vec3; // node origin at rest, model space
  bind: Trs; // local to the parent node
  extras: Record<string, number>;
  tiers: MeshData[]; // geometry in the node's own space
}

export interface ArticulatedBundle {
  kind: "articulated";
  nodes: ArticulatedNode[];
  materials: Material[];
  textures: Texture[];
  bounds: Bounds; // over every pose the pose driver reaches (`posedBounds`)
}

/** One mesh per state. A scenery kind's states are its row's (`scenery.ts`);
 *  a kit's states are its modules, named by module id, each in its own frame. */
export interface StaticBundle {
  kind: "static";
  states: { name: string; tiers: MeshData[]; bounds: Bounds }[];
  materials: Material[];
  textures: Texture[];
  bounds: Bounds; // over every state
}

/** Channel mode per joint: absent (use the body's bind), constant (one sample), animated. */
export const CHANNEL_ABSENT = 0;
export const CHANNEL_CONSTANT = 1;
export const CHANNEL_ANIMATED = 2;

export interface Clip {
  name: string;
  loop: boolean;
  duration: number;
  frames: number; // samples at sample_hz, including both ends
  stride_m?: number;
  markers: { name: string; phase: number }[];
  rotation_modes: Uint8Array; // per joint
  translation_modes: Uint8Array; // per joint
  rotations: Int16Array; // snorm16 xyzw, joint-major, one or `frames` samples per non-absent joint
  translations: Float32Array; // xyz, same layout
}

export interface SkeletonClips {
  kind: "clips";
  id: string;
  joints: { name: string; parent: number }[];
  sample_hz: number;
  clips: Clip[];
}

export type Bundle = SkinnedBundle | ArticulatedBundle | StaticBundle | SkeletonClips;

/** The two armies. Both draw the same meshes; a side differs only by its tint. */
export const SIDES = ["blue", "red"] as const;
export type Side = (typeof SIDES)[number];
/** Per side, the linear RGB multiplier on tint-masked surfaces (`Material.tint`). */
export type SideTints = Record<Side, Vec3>;

/** Catalog tolerances: how far art may sit from the simulation's numbers. */
export interface Tolerances {
  ground_m: number;
  soldier_height_m: number;
  eye_m: number;
  muzzle_m: number;
  /** Every face of a vehicle's hull box, its top too. What may stand
   *  outside it is dressing, with its own allowance (`unitArt.ts`). */
  hull_extent_m: number;
  /** A static appearance against its simulation box (roof overhangs, rubble). */
  footprint_m: number;
  /** A vehicle mount's muzzle at rest against its `mounts` row (pivot plus muzzle). */
  vehicle_muzzle_m: number;
  /** A mount's drawn muzzle against the simulation's as its turret and gun turn. */
  muzzle_arc_m: number;
}

/** The simulation's bodies, read as the fit authority: the one soldier frame
 *  every squad shares (the fixture's `physics`), and every unit type, whose
 *  own resolved numbers its model is fitted to. */
export interface Authority {
  soldier_height_m: number;
  infantry_eye_m: number;
  infantry_muzzle_m: number;
  units: UnitCatalog;
  /** The forest canopy (`forests.rule.canopy_height_m` and
   *  `canopy_radius_m`): a tree, unscaled, stands inside it. */
  canopy_height_m: number;
  canopy_radius_m: number;
  /** How a placed building of a template ends (the building prop types'
   *  `destroyed.into.building`), or null when no prop type has the rule. */
  collapse: BuildingCollapse | null;
}

/** The simulation's rule for a destroyed template building: one of
 *  `max_floors` floors or fewer collapses, each part to remains on its own
 *  plan, as tall as `height_fraction` of the building's height held between
 *  `min_height_m` and `max_height_m`; a taller one stands, gutted, at its
 *  full height. */
export interface BuildingCollapse {
  min_height_m: number;
  height_fraction: number;
  max_height_m: number;
  max_floors: number;
}

export interface ClipDeclaration {
  loop: boolean;
  stride_m?: number;
  markers?: Record<string, number>;
}

export interface SkeletonEntry {
  source: string;
  basis_yaw_deg: number;
  sample_hz: number;
  /** The standing-aim reference pose infantry fit is measured in. */
  aim_reference: PoseRef;
  clips: Record<string, ClipDeclaration>;
}

export interface AppearanceEntry {
  unit: AppearanceUnit;
  /** For `unit: "scenery"`: the scenery kind, a key of `SCENERY_KINDS`. */
  scenery?: string;
  /** Skinned, articulated and kit: one GLB. Static: one GLB per state. */
  source?: string;
  states?: Record<string, string>;
  basis_yaw_deg: number;
  skeleton?: string;
  far_pose?: PoseRef;
  corpse_pose?: PoseRef;
  tolerances?: Partial<Tolerances>;
  /** A vehicle: which of its rigs draws each of its unit type's mounts, by
   *  mount name (`{ "cannon": "gun", "HMG": "hmg" }`). The validator checks
   *  every mount is declared and its rig's nodes exist; the battle poses
   *  and places muzzles by it. */
  mounts?: MountDraws;
  /**
   * Static appearances that stand for a simulation prop (scenery kinds with
   * a `prop` footprint): the half extents of the box the art is authored to,
   * bottom on the ground. It must be a box the simulation places (a map
   * placement, a wreck's hull); the battle fits each placed box from it.
   */
  footprint_half_m?: Vec3;
  /** A scenery look of one region: one of the presets' families
   *  (`parcels.regional_families` in `fixtures/map-presets.json`). It is
   *  fetched only for a map of that family, which draws it in place of the
   *  looks of no region for its scenery kind. Absent: every map's. */
  regional_family?: string;
  /** A prop's paints (sRGB): each placed body wears one on its paintable
   *  surfaces (materials marked `tint`), chosen by which body it is. Absent:
   *  every body as authored. */
  paints?: Vec3[];
  /** A generated grass kind's spec: `asset grass` writes its one state's
   *  GLB from it (`grass.ts`). The bake reads the GLB, never the spec. */
  grass?: GrassSpec;
}

/** A grass clump, as the generator builds it. Colours are sRGB. */
export interface GrassSpec {
  seed: number;
  blades: number;
  /** The clump's spread: blade roots scatter within this radius of its
   *  origin, a tight tuft or a loose stand. */
  radius_m: number;
  /** Each blade's height, drawn from this range: no blade stands taller. */
  height_m: [number, number];
  /** Blade width at the root. */
  width_m: number;
  /** How far the tip leans out, as a fraction of the blade's height. */
  lean: [number, number];
  /** How far the top hangs over: the tip falls this fraction of the blade's
   *  height below a straight blade's. Absent: none. */
  droop?: number;
  /** The blade's outline. Absent: a grass blade, narrowing from the root to
   *  a point. */
  shape?: {
    /** How late the blade narrows: 1.6 is a grass blade; larger holds the
     *  root's width longer, as a stalk does. */
    taper: number;
    /** A leaf's swell: mid-blade is this many root widths wider. */
    belly: number;
  };
  /** The blade's colour at its root, middle and tip. */
  colors: {
    root: [number, number, number];
    mid: [number, number, number];
    tip: [number, number, number];
  };
  /** Per-blade brightness jitter, as a fraction. */
  jitter: number;
  /** How far the blades answer the field's one wind: 1 sways as the biome's
   *  wind says, 0 stands still. */
  wind: number;
  /** A seed head (wheat, grasses in flower) on a `chance` of the blades: from
   *  `from` of the height the blade swells to `width` times its root width,
   *  closing at the tip, and takes `color` when it has one (an ear, a flower). */
  head?: { from: number; width: number; chance: number; color?: [number, number, number] };
  /** Dry stems among the green: a `chance` of the blades take these colours. */
  dry?: {
    chance: number;
    colors: {
      root: [number, number, number];
      mid: [number, number, number];
      tip: [number, number, number];
    };
  };
}

/** One city source set (`blender/city/README.md`): its `templates.json`,
 *  the kit appearance whose modules its rows place, and the physical
 *  catalogue its templates are rows of, by the name the bake has it under
 *  (`TemplateCatalogue`). */
export interface CitySetEntry {
  templates: string;
  kit: string;
  catalogue: string;
}

/** `assets/catalog.json`, authored. The bake writes the runtime projection. */
export interface Catalog {
  tolerances: Tolerances;
  sides: SideTints;
  skeletons: Record<string, SkeletonEntry>;
  appearances: Record<string, AppearanceEntry>;
  /** The city sets, by set name. The bake packs them all, whichever
   *  catalogue each dresses, into the one template art library. */
  city_sets?: Record<string, CitySetEntry>;
  /** The interior atlas sheets' source pictures (`blender/city/interiors.py`),
   *  by sheet: what a room material's bundle is given to show. */
  interiors?: Partial<Record<InteriorSheet, string>>;
}

/** `assets/runtime/catalog.json`, written by the bake: names to content hashes. */
export interface RuntimeCatalog {
  sides: SideTints;
  /** Kits and the template library travel as gzip. Keys remain their raw
   * content hashes; encoded hashes name the immutable files actually served. */
  gzip?: Record<string, GzipTransport>;
  skeletons: Record<string, string>;
  appearances: Record<
    string,
    {
      unit: AppearanceUnit;
      kind: BundleKind;
      bundle: string;
      skeleton?: string;
      scenery?: string;
      footprint_half_m?: Vec3;
      mounts?: MountDraws;
      regional_family?: string;
      paints?: Vec3[];
    }
  >;
  /** The template art library (`templateLibrary.ts`), when the catalog has
   *  city sets: its file's content hash, its art identity, and the hashes of
   *  the physical catalogues it covers. */
  templates?: { library: string; art_hash: string; covers: string[] };
}

export interface GzipTransport {
  hash: string;
  bytes: number;
  raw_bytes: number;
}

/** The file a bundle lives in, under its content hash's directory. */
export const BUNDLE_FILE = "bundle.bin";
export const bundlePath = (hash: string) => `${hash}/${BUNDLE_FILE}`;
/** The file the template art library lives in, under its content hash's directory. */
export const TEMPLATE_LIBRARY_FILE = "templates.bin";
export const templateLibraryPath = (hash: string) => `${hash}/${TEMPLATE_LIBRARY_FILE}`;

/** A kit bundle's byte budget: every module's four tiers and its textures.
 * A loose tripwire against a runaway bundle, not a target: raise it when real
 * art outgrows it and nothing visibly suffers. */
export const KIT_BUNDLE_MAX_BYTES = 256 * 1024 * 1024;
/** Aggregate wire bytes of what a map fetches on request (`fetchedOnRequest`):
 * the kits its buildings place and its family's regional looks, plus the
 * template library. Separate from decoded per-kit and resident/GPU-memory
 * budgets; the looks of no region load with the catalog and are not counted.
 * Like the kit budget, a loose tripwire, not a target. */
export const MAP_DOWNLOAD_MAX_BYTES = 256 * 1024 * 1024;
