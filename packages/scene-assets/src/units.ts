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
        reverse_speed_factor: number;
      };
    }
  | {
      wheeled: {
        offroad_kmh: number;
        road_kmh: number;
        turn_deg_s: number;
        turning_radius_m: number;
        reverse_speed_factor: number;
      };
    }
  | {
      /** Flies in its layer (low air when unnamed) at the heights the rules'
       *  `air` section sets, by its flight model. */
      air: {
        cruise_kmh: number;
        turn_deg_s: number;
        climb_mps: number;
        layer?: "low_air" | "high_air";
        flight: "rotor" | "fixed_wing";
        /** A bought one's wait off the map, seconds (D38). */
        arrival?: { delay_s: [number, number] };
        /** Its own cruise height, overriding its layer's (D40). */
        cruise_agl_m?: number;
      };
    };

/** Whether a type flies (`contract::catalog::Mobility::Air`): its hull is
 *  in the air, so it rolls on nothing and raises no dust. */
export const airborne = (type: Pick<UnitType, "mobility">): boolean => "air" in type.mobility;

/** It flies in the high-air layer: its card sits on top, and its shadow lies
 *  beneath it. */
export const highAir = (type: Pick<UnitType, "mobility">): boolean =>
  "air" in type.mobility && type.mobility.air.layer === "high_air";

/** A type's altitude layer, as the simulation names it. */
export const layerOf = (type: Pick<UnitType, "mobility">): "ground" | "low_air" | "high_air" =>
  "air" in type.mobility ? (highAir(type) ? "high_air" : "low_air") : "ground";

/** It flies on wings (`contract::catalog::FlightModel`): it never stops,
 *  circles its goal and attacks in passes, so it has no ground marker, drop
 *  line or destination circle. */
export const fixedWing = (type: Pick<UnitType, "mobility">): boolean =>
  "air" in type.mobility && type.mobility.air.flight === "fixed_wing";

/** The air war's roles: ground air defence, SEAD (a fixed-wing that finds
 *  radars and carries an anti-radiation row), and the other fixed-wing. A
 *  roster grouping for the labs; no rule reads it. */
export const AIR_ROLES = ["air_defense", "sead", "fixed_wing"] as const;
export type AirRole = (typeof AIR_ROLES)[number];

/** A type's air-war role, or null for one with none. Air defence is a
 *  ground type whose weapons reach high air; how short- or long-range it is
 *  is its `airReach`, not a role. */
export function airRole(
  type: Pick<UnitType, "mobility" | "capabilities" | "mounts">,
  weapons: Record<string, WeaponRow>,
): AirRole | null {
  if (fixedWing(type))
    return type.capabilities.radar_receiver &&
      type.mounts.some((m) => m.weapons.some((id) => weapons[id]?.anti_radiation))
      ? "sead"
      : "fixed_wing";
  if (layerOf(type) === "ground" && airReach(type, weapons).high_air > 0) return "air_defense";
  return null;
}

/** How far a type's weapons reach into each air layer, metres: the farthest
 *  of its rows' explicit `targets.<layer>.range_m`, and 0 where none reaches. */
export function airReach(
  type: Pick<UnitType, "mounts">,
  weapons: Record<string, WeaponRow>,
): { low_air: number; high_air: number } {
  const reach = { low_air: 0, high_air: 0 };
  for (const w of type.mounts.flatMap((m) => m.weapons).map((id) => weapons[id]))
    for (const layer of ["low_air", "high_air"] as const) {
      const fire = w?.targets?.[layer];
      if (fire) reach[layer] = Math.max(reach[layer], fire.range_m);
    }
  return reach;
}

/** A mount row, as a hull lists it or a soldier kind carries it. */
export interface MountRow {
  /** What other fields name it by (`sensors.on`, `on`, a model's rig
   *  declarations); never shown. */
  id: string;
  /** What the player reads: a concise label (`contract::labels`). */
  name: string;
  weapons: string[];
  squad: boolean;
  special: boolean;
  /** Appearance set worn by this single infantry weapon's current operator. */
  operator_appearance?: {
    active: string[];
    carried: string[];
    /** Authored stationary hold. Movement uses the carried kit. */
    active_pose?: { clip: string; phase: number };
  };
  turret: boolean;
  /** The earlier turret mount that carries it; null, the hull. */
  on: string | null;
  pivot_m: Vec3;
  /** Its muzzle from the pivot along its own bearing; null, a hand weapon. */
  muzzle_m: Vec3 | null;
}

/** The factions a battle fields. */
export const FACTIONS = ["us", "europe", "eastern"] as const;
export type Faction = (typeof FACTIONS)[number];
export type UnitCategory = "rec" | "inf" | "veh" | "sup" | "hel" | "air";

export interface RosterMembership {
  factions: Faction[];
  family_name: string;
  category: UnitCategory;
  variant: string;
}

export interface UnitCard {
  id: string;
  name: string;
  description: string;
  family: string;
  cost: number;
  roster: RosterMembership;
  disabled_reason: string | null;
  /** A disabled card's planned weapons, each a label and its weapon icon;
   *  empty for a unit type, whose weapons are its mounts. */
  planned_weapons: { name: string; icon: string }[];
}

export interface UnitType {
  id: string;
  name: string;
  description: string;
  faction: string;
  family: string;
  roster?: RosterMembership;
  roles: string[];
  cost: number;
  body: Body;
  mobility: Mobility;
  /** Its own concealment, multiplying what its ground gives it. */
  concealment: number;
  sensors: {
    ground_m: number;
    low_air_m: number;
    high_air_m: number;
    sight_shape: { front: number; side: number; rear: number };
    /** Above 1 cancels a target's concealment, below 1 deepens it; omitted is 1. */
    spotting?: number;
    on?: string;
  };
  mounts: MountRow[];
  /** Its presentation class, as the catalog derives it from its physics
   *  (`UnitType::vehicle_class`): how it moves, its hull's weight class and
   *  `_logistics` for a supply hauler, as `tracked_heavy`, `air_light` or
   *  `wheeled_medium_logistics`. Vehicle sound, running gear and art rules
   *  are keyed by it, never by unit id. Null for a squad. */
  vehicle_class: string | null;
  capabilities: {
    carriage?: { seats: number };
    deploy?: { seconds: number; pack_seconds: number };
    supply?: { stock: number };
    active_protection?: {
      /** What the card's protection row reads, and its weapon icon. */
      name: string;
      icon: string;
      capacity: number;
      cooldown_s: number;
      standoff_m: number;
      service_s: number;
      stock_per_charge: number;
    };
    air?: {
      fuel_capacity_s: number;
      afterburner?: { speed_factor: number; fuel_factor: number };
    };
    flares?: { capacity: number; reload_s: number };
    radar_emitter?: boolean;
    /** It finds enemy radars that emit, within `range_m`. */
    radar_receiver?: { range_m: number };
  };
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
    hp_scale?: "fixed" | "building_floor_bands";
    armor: number;
    topples: boolean;
    garrison: boolean;
  };
  destroyed?:
    | "removed"
    | "cleared"
    | {
        into: {
          prop: string;
          height_m: number;
          building?: {
            height_fraction: number;
            max_height_m: number;
            collapse_max_floors: number;
            gutted_prop: string;
          };
        };
      };
  appearance: {
    drawn_by: string;
    status?: "systems_only";
    modular?: boolean;
    map_only?: boolean;
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
  /** Each layer it engages owns its range and launch spread. */
  targets?: Partial<
    Record<"ground" | "low_air" | "high_air", { range_m: number; scatter_mrad: number }>
  >;
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
  /** Player cards include planned variants that have no physical type index. */
  cards: UnitCard[];
  /** The prop types, by id. */
  props: Record<string, PropType>;
}

/** A model's rigs that can draw a mount: the turret and gun, or the HMG on
 *  its own ring. An appearance names, per mount, the rig that draws it
 *  (`assets/catalog.json` `appearances.<name>.mounts`). */
export type Articulation = "gun" | "hmg";

/** How a model draws a mount: by one of its rigs; by its hull, for a mount
 *  fixed in the hull (neither a turret nor carried by one: a helicopter's
 *  rocket pod), which never turns apart from the body, so its muzzle is the
 *  published one; or by hand (a soldier's weapon, drawn with him). */
export type MountRole = Articulation | "hull" | "hand";

/** A vehicle mount fixed in its hull: not a turret, and carried by none
 *  (the simulation's `weapons::hull_fixed`). */
export const hullFixed = (m: Pick<MountRow, "turret" | "on">): boolean =>
  !m.turret && m.on === null;

/** An appearance's mount declarations: mount id to the rig that draws it,
 *  or `hull` for a turning mount its model has no rig for (a launcher drawn
 *  fixed on the hull): it never turns on screen, and its muzzle is the
 *  published one. */
export type MountDraws = Readonly<Record<string, Articulation | "hull">>;

/** Each rig's nodes: the node it yaws on, the one it pitches on, and its muzzle. */
export const MOUNT_NODES: Record<Articulation, { yaw: string; pitch: string; muzzle: string }> = {
  gun: { yaw: "turret", pitch: "gun", muzzle: "muzzle" },
  hmg: { yaw: "hmg", pitch: "hmg_gun", muzzle: "hmg_muzzle" },
};

export const isArticulation = (name: string): name is Articulation =>
  Object.hasOwn(MOUNT_NODES, name);

/** How a model declaring `draws` draws each of `type`'s mounts, in mount
 *  order: the rig it names for the mount, else by its hull for a mount fixed
 *  in it, else by hand. A squad's mounts are always by hand: its soldiers
 *  carry them. The validator refuses a hull model that leaves a turning
 *  mount undeclared (`fit.mount_draw`). */
export function mountRoles(
  type: Pick<UnitType, "mounts" | "body">,
  draws: MountDraws | null | undefined,
): MountRole[] {
  if (!("hull" in type.body)) return type.mounts.map(() => "hand");
  return type.mounts.map((m) => draws?.[m.id] || (hullFixed(m) ? "hull" : "hand"));
}

export class UnitCatalog {
  private readonly byId: Map<string, UnitType>;
  private worn: ReadonlySet<string> | undefined;

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

  /** Every appearance its units wear: each hull's model, each soldier kind's
   *  set, and the equipment sets their weapons' operators wear. What a page
   *  running these units loads of unit art (`catalogLoadNames`). */
  get appearances(): ReadonlySet<string> {
    if (!this.worn) {
      const worn = new Set(this.appearancesFor(this.ids));
      // Preserve the catalog-wide view used by tools: it also validates and
      // preloads a soldier entry that is not currently referenced by a unit.
      for (const row of Object.values(this.view.soldiers)) {
        for (const name of row.appearance) worn.add(name);
        for (const { operator_appearance: kit } of row.mounts)
          for (const name of [...(kit?.active ?? []), ...(kit?.carried ?? [])]) worn.add(name);
      }
      this.worn = worn;
    }
    return this.worn;
  }

  /** Every appearance worn by the given unit kinds, including their soldier
   *  slots and operator kits. The loader adds faction looks and wrecks. */
  appearancesFor(kinds: Iterable<string>): ReadonlySet<string> {
    const worn = new Set<string>();
    const addMounts = (mounts: readonly MountRow[]) => {
      for (const { operator_appearance: kit } of mounts)
        for (const name of [...(kit?.active ?? []), ...(kit?.carried ?? [])]) worn.add(name);
    };
    const soldiers = new Set<string>();
    for (const kind of kinds) {
      const row = this.type(kind);
      for (const name of [row.appearance ?? []].flat()) worn.add(name);
      addMounts(row.mounts);
      for (const soldier of this.slots(kind)) soldiers.add(soldier);
    }
    for (const kind of soldiers) {
      const row = this.soldier(kind);
      for (const name of row.appearance) worn.add(name);
      addMounts(row.mounts);
    }
    return worn;
  }

  get cards(): readonly UnitCard[] {
    return this.view.cards;
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
