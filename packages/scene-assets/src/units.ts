// The unit catalog as the browser and the asset tools read it: the
// simulation's resolved view (`fixtures/unit-catalog.json`, written by
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
  | { foot: { mps: number; road_multiplier: number } }
  | { tracked: { mps: number; road_mps: number; turn_deg_s: number; reverse_fraction: number } }
  | {
      wheeled: {
        mps: number;
        road_mps: number;
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

/** A mount as a unit carries it: for a squad, the slots whose soldiers carry it. */
export interface CarriedMount extends MountRow {
  carriers: number[];
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
  mounts: CarriedMount[];
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

/** `Catalog::view`: the resolved catalog. */
export interface CatalogView {
  /** What a scenario's rules carry as `catalog`. */
  documents: unknown[];
  roles: Record<string, Role>;
  parts: Record<string, Part>;
  soldiers: Record<string, SoldierKind>;
  /** In the simulation's index order: the publication's `unitKinds`. */
  units: UnitType[];
}

/** How a model draws a mount: the turret and gun, the HMG on its ring, or
 *  nothing (a hand weapon, drawn with the soldier). */
export type MountRole = "gun" | "hmg" | "hand";

/** The rig nodes that draw a mount role: the node it yaws on, the one it
 *  pitches on, and its muzzle. */
export const MOUNT_NODES: Record<
  Exclude<MountRole, "hand">,
  { yaw: string; pitch: string; muzzle: string }
> = {
  gun: { yaw: "turret", pitch: "gun", muzzle: "muzzle" },
  hmg: { yaw: "hmg", pitch: "hmg_gun", muzzle: "hmg_muzzle" },
};

export class UnitCatalog {
  private readonly byId: Map<string, UnitType>;
  private readonly roles = new Map<string, readonly MountRole[]>();

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

  /** Its first turret mount's index, or -1. */
  turret(id: string): number {
    return this.type(id).mounts.findIndex((m) => m.turret);
  }

  /** How the model draws each mount, in order: the first turret mount is the
   *  gun unless it is a machine gun, a machine gun on a turret is the HMG,
   *  anything else is carried by hand. */
  mountRoles(id: string): readonly MountRole[] {
    let roles = this.roles.get(id);
    if (!roles) {
      let gun = false;
      roles = this.type(id).mounts.map((m) => {
        if (!m.turret) return "hand";
        if (/hmg/i.test(m.name)) return "hmg";
        if (gun) return "hand";
        gun = true;
        return "gun";
      });
      this.roles.set(id, roles);
    }
    return roles;
  }
}
