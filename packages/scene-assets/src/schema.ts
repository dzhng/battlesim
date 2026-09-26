// The appearance-bundle contract: what a baked bundle holds, what the catalog
// declares, and what each unit kind requires of its art. Engine space is Z up,
// +X forward, +Y left, metres, origin on the ground.

import type { Mat4, Vec3 } from "math";
import type { Trs } from "./trs.ts";

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
  "structure.skeleton",
  "structure.loop_flags",
  "structure.clips",
  "structure.states",
  "structure.scenery_kind",
  "structure.texture",
  // basis
  "basis.ground",
  "basis.forward",
  "basis.up",
  // fit to authority
  "fit.soldier_height",
  "fit.eye",
  "fit.muzzle",
  "fit.hull_extents",
  "fit.tank_muzzle",
  "fit.muzzle_arc",
  // required nodes
  "nodes.missing",
  "nodes.hierarchy",
  "nodes.duplicate",
  "nodes.track_properties",
  "nodes.deploy_motion",
  // provenance
  "provenance.unlisted",
  "provenance.licence",
] as const;
export type FindingCode = (typeof FINDING_CODES)[number];

/** Mesh tiers, finest first. Source meshes carry them as a `_LOD<n>` name suffix. */
export const TIER_COUNT = 4;

export type BundleKind = "skinned" | "articulated" | "static";
/** Who an appearance is. "scenery" is every prop, tree, hedgerow and grass
 *  kind; which one is the entry's `scenery` (`scenery.ts`). */
export type UnitKind = "rifle" | "recon" | "at" | "tank" | "supply" | "building" | "scenery";

export const UNIT_BUNDLE_KIND: Record<UnitKind, BundleKind> = {
  rifle: "skinned",
  recon: "skinned",
  at: "skinned",
  tank: "articulated",
  supply: "articulated",
  building: "static",
  scenery: "static",
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
/** Static bundle states every building carries. */
export const BUILDING_STATES = ["intact", "ruin"] as const;

/** A merged, material-ranged triangle mesh in its owner's space. */
export interface MeshData {
  positions: Float32Array; // xyz
  normals: Int16Array; // snorm16 xyzw, w = 0
  uvs: Float32Array; // uv
  colors: Uint8Array; // unorm8 rgba
  joints?: Uint8Array; // skinned only: 4 joint indices per vertex
  weights?: Uint16Array; // skinned only: unorm16, each vertex sums to 65535
  indices: Uint16Array | Uint32Array;
  /** Contiguous index ranges, one per material. */
  draws: { material: number; first: number; count: number }[];
}

export interface Material {
  name: string;
  base_color: [number, number, number, number];
  metallic: number;
  roughness: number;
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
  bounds: Bounds; // over every pose the pose driver reaches (`posedBounds`)
}

export interface StaticBundle {
  kind: "static";
  states: { name: string; tiers: MeshData[]; bounds: Bounds }[];
  materials: Material[];
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

/** Catalog tolerances: how far art may sit from the simulation's numbers. */
export interface Tolerances {
  ground_m: number;
  soldier_height_m: number;
  eye_m: number;
  muzzle_m: number;
  hull_extent_m: number;
  tank_muzzle_m: number;
  muzzle_arc_m: number;
}

/** The fixture's `physics` block: the simulation's bodies, read as the fit authority. */
export interface Authority {
  soldier_height_m: number;
  infantry_eye_m: number;
  infantry_muzzle_m: number;
  tank_half_extents_m: Vec3;
  tank_muzzle_local_m: Vec3;
  supply_half_extents_m: Vec3;
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
  unit: UnitKind;
  /** For `unit: "scenery"`: the scenery kind, a key of `SCENERY_KINDS`. */
  scenery?: string;
  /** Skinned and articulated: one GLB. Static: one GLB per state. */
  source?: string;
  states?: Record<string, string>;
  basis_yaw_deg: number;
  skeleton?: string;
  far_pose?: PoseRef;
  corpse_pose?: PoseRef;
  tolerances?: Partial<Tolerances>;
}

/** `assets/catalog.json`, authored. The bake writes the runtime projection. */
export interface Catalog {
  tolerances: Tolerances;
  skeletons: Record<string, SkeletonEntry>;
  appearances: Record<string, AppearanceEntry>;
}

/** `assets/runtime/catalog.json`, written by the bake: names to content hashes. */
export interface RuntimeCatalog {
  skeletons: Record<string, string>;
  appearances: Record<
    string,
    { unit: UnitKind; kind: BundleKind; bundle: string; skeleton?: string; scenery?: string }
  >;
}

/** Licences a source may carry. Third-party licences need the user's acceptance first. */
export const ALLOWED_LICENCES = ["CC0-1.0", "MIT", "project-owned"] as const;

/** A provenance record: the reuse manifest's `third_party` entries. */
export interface ProvenanceEntry {
  path: string;
  sha256: string;
  licence: string;
  accepted_by: string;
}

/** The file a bundle lives in, under its content hash's directory. */
export const BUNDLE_FILE = "bundle.bin";
export const bundlePath = (hash: string) => `${hash}/${BUNDLE_FILE}`;
