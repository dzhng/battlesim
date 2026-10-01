// The unit catalog as the browser and the asset tools read it: the
// simulation's resolved view (`fixtures/catalog.json`, written by
// `sim::fixtures::catalog_view`, or the wasm `resolve_catalog` for a test
// catalog). Nothing here resolves `extends` or lists unit types: the
// simulation owns resolution, and each derived question below has this one
// owner on the TypeScript side, answered from a type's components.

import type { Vec3 } from "math";

export type WeightClass = "light" | "medium" | "heavy" | "immovable";

export interface FaceValues {
  front: number;
  side: number;
  rear: number;
  roof: number;
}

export interface Hull {
  half_extents_m: Vec3;
  eye_m: number;
  hp: number;
  armor: FaceValues & { ricochet: FaceValues };
  weight_class: WeightClass;
  push_class: string;
  wreck: string;
}

export type Body = { squad: { slots: string[] } } | { hull: Hull };

export type Mobility =
  | { foot: { offroad_kmh: number; road_kmh: number } }
  | {
      tracked: {
        offroad_kmh: number;
        road_kmh: number;
        turn_deg_s: number;
        reverse_fraction: number;
      };
    }
  | {
      wheeled: {
        offroad_kmh: number;
        road_kmh: number;
        turn_deg_s: number;
        turning_radius_m: number;
        reverse_fraction: number;
      };
    };

/** A mount row, as a hull lists it or a soldier kind carries it. */
export interface MountRow {
  name: string;
  weapons: string[];
  squad: boolean;
  special: boolean;
  turret: boolean;
  /** The earlier turret mount that carries it; null, the hull. */
  on: string | null;
  pivot_m: Vec3;
  /** Its muzzle from the pivot along its own bearing; null, a hand weapon. */
  muzzle_m: Vec3 | null;
}

export interface UnitType {
  id: string;
  name: string;
  description: string;
  faction: string;
  family: string;
  roles: string[];
  cost: number;
  body: Body;
  mobility: Mobility;
  sensors: {
    ground_m: number;
    sight_shape: { front: number; side: number; rear: number };
    on?: string;
  };
  mounts: MountRow[];
  capabilities: { deploy?: { seconds: number }; supply?: { stock: number } };
  sound: { profile: "infantry" | "vehicle"; loudness_m: number };
  /** A hull's model; a squad draws its soldiers' appearance sets. */
  appearance?: string;
  parts?: string[];
}

export interface SoldierKind {
  name: string;
  description: string;
  hp: number;
  /** The appearances a soldier of this kind may wear, one picked per soldier. */
  appearance: string[];
  mounts: MountRow[];
}

export interface Role {
  name: string;
  description: string;
  /** The NATO-style symbol's modifiers, drawn in order inside its frame. */
  symbol: string[];
}

export interface Part {
  name: string;
  description: string;
  /** Model node names a type listing the part must draw (`*` ends a prefix). */
  nodes: string[];
}

/** A prop type as the resolved catalog holds it: its body row, what it
 *  becomes when destroyed and what draws it (`contract::catalog::PropType`). */
export interface PropType {
  body: {
    blocks: { infantry: boolean; vehicle: boolean };
    stops_rounds: boolean;
    occludes: boolean;
    weight_class: WeightClass;
    cover_tier: "light" | "medium" | "heavy" | null;
    lifetime_s: number | null;
    conceals: number;
    hp: number | null;
    armor: number;
    topples: boolean;
    garrison: boolean;
  };
  destroyed?: "removed" | "cleared" | { into: { prop: string; height_m: number } };
  appearance: {
    drawn_by: string;
    modular?: boolean;
    map_only?: boolean;
    remains_state?: string;
  };
}

/** One resolved weapon row (`contract::weapons::WeaponDefinition`), as the
 *  shipped view carries it; the fields presentation reads are typed. */
export interface WeaponRow {
  name: string;
  description: string;
  icon: string;
  speed_mps: number;
  range_m: number;
  aim_s: number;
  reload_s: number;
  ammo: number | "unlimited";
  penetration: number;
  [field: string]: unknown;
}

/** `Catalog::view`: the resolved catalog. */
export interface CatalogView {
  /** What a scenario's rules carry as `catalog`. */
  documents: unknown[];
  roles: Record<string, Role>;
  parts: Record<string, Part>;
  soldiers: Record<string, SoldierKind>;
  /** In the simulation's index order: the publication's `unitKinds`. */
  units: UnitType[];
  /** The prop types, by id. */
  props: Record<string, PropType>;
}

/** A model's rigs that can draw a mount: the turret and gun, or the HMG on
 *  its own ring. An appearance names, per mount, the rig that draws it
 *  (`assets/catalog.json` `appearances.<name>.mounts`). */
export type Articulation = "gun" | "hmg";

/** How a model draws a mount: by one of its rigs, or by hand (a soldier's
 *  weapon, drawn with him). */
export type MountRole = Articulation | "hand";

/** An appearance's mount declarations: mount name to the rig that draws it. */
export type MountDraws = Readonly<Record<string, Articulation>>;

/** Each rig's nodes: the node it yaws on, the one it pitches on, and its muzzle. */
export const MOUNT_NODES: Record<Articulation, { yaw: string; pitch: string; muzzle: string }> = {
  gun: { yaw: "turret", pitch: "gun", muzzle: "muzzle" },
  hmg: { yaw: "hmg", pitch: "hmg_gun", muzzle: "hmg_muzzle" },
};

export const isArticulation = (name: string): name is Articulation =>
  Object.hasOwn(MOUNT_NODES, name);

/** How a model declaring `draws` draws each of `type`'s mounts, in mount
 *  order: the rig it names for the mount, else by hand. A squad's mounts
 *  are always by hand: its soldiers carry them. The validator refuses a
 *  hull model that leaves a mount undeclared (`fit.mount_draw`). */
export function mountRoles(
  type: Pick<UnitType, "mounts" | "body">,
  draws: MountDraws | null | undefined,
): MountRole[] {
  const hull = "hull" in type.body;
  return type.mounts.map((m) => (hull && draws?.[m.name]) || "hand");
}

export class UnitCatalog {
  private readonly byId: Map<string, UnitType>;

  // A plain field, not a parameter property: the asset CLI runs this file
  // under Node's type stripping, which has no parameter properties.
  readonly view: CatalogView;

  constructor(view: CatalogView) {
    this.view = view;
    this.byId = new Map(view.units.map((t) => [t.id, t]));
  }

  /** Every type id, in the simulation's index order. */
  get ids(): string[] {
    return this.view.units.map((t) => t.id);
  }

  get documents(): unknown[] {
    return this.view.documents;
  }

  has(id: string): boolean {
    return this.byId.has(id);
  }

  type(id: string): UnitType {
    const t = this.byId.get(id);
    if (!t) throw new Error(`no unit type ${id}`);
    return t;
  }

  /** A vehicle's hull; null for a squad. "Is it a vehicle" is `hull(id) !== null`. */
  hull(id: string): Hull | null {
    const body = this.type(id).body;
    return "hull" in body ? body.hull : null;
  }

  /** A squad's soldier kinds, one per slot; empty for a hull. */
  slots(id: string): string[] {
    const body = this.type(id).body;
    return "squad" in body ? body.squad.slots : [];
  }

  /** Infantry: it has soldier slots. */
  isInfantry(id: string): boolean {
    return this.slots(id).length > 0;
  }

  soldier(kind: string): SoldierKind {
    const s = this.view.soldiers[kind];
    if (!s) throw new Error(`no soldier kind ${kind}`);
    return s;
  }
}
