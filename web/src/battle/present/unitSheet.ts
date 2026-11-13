/** The Armory's sheet of one card: everything a player reads about a unit
 *  before fielding it, from the game's own owners. The catalog gives the
 *  body, movement, senses and mounts; the catalog's published weapon facts
 *  (`WeaponRow.derived`) give fires-regardless and reach, never recomputed
 *  here; the info panel's `equipmentName` names each mount. Pure, so it is
 *  tested without a page.
 *
 *  Bars measure each figure against the largest of its kind in the
 *  catalog's roster of the same body: squads against squads, hulls against
 *  hulls, and a squad's weapons against squads' weapons. */
import type {
  Mobility,
  Reach,
  UnitCard,
  UnitType,
  WeaponRow,
} from "@packages/scene-assets/src/units";
import type { SessionCatalog } from "../catalog/compose";
import { equipmentName } from "./panelRows";

export type SheetBody = "squad" | "hull";

export interface UnitSheet {
  card: UnitCard;
  /** No unit type yet (L10): the card shows its planned weapons only. */
  planned: boolean;
  body: SheetBody | null;
  /** A squad's soldiers. */
  men?: number;
  /** A squad's hit points per soldier: lowest and highest, equal when
   *  every soldier is alike (U2). */
  hpPerMan?: [number, number];
  /** A hull's hit points and armour per face. */
  hp?: number;
  armor?: Record<"front" | "side" | "rear" | "roof", number>;
  movement: SheetStat[];
  senses: SheetStat[];
  /** What else the unit does, each a short phrase. */
  capabilities: string[];
  /** Its equipment, by mount, its ammunition kinds beneath (D2). A planned
   *  card lists its planned weapons, each with no kinds. */
  mounts: SheetMount[];
}

export interface SheetMount {
  /** The mount's name, as the info panel names it. */
  name: string;
  /** The weapon icon id (`weaponIcon`) of its first kind. */
  icon: string;
  kinds: SheetAmmo[];
}

export interface SheetAmmo {
  /** The weapon row's id. */
  weapon: string;
  name: string;
  description: string;
  ammo: number | "unlimited";
  /** It fires at an enemy it sees even when it can't hurt it (published). */
  firesRegardless: boolean;
  /** Per altitude band it engages (published). */
  reach: Reach[];
  stats: SheetStat[];
}

export interface SheetStat {
  key: SheetStatKey;
  value: number;
  /** 0…1 of the largest value of this key in the roster of the same body. */
  scale: number;
}

/** Every figure a sheet draws with a bar: what it is called, its unit, and
 *  how many decimals it reads with. The one table the page reads. */
export const SHEET_STATS = {
  road_kmh: { label: "Road", unit: "km/h", digits: 0 },
  offroad_kmh: { label: "Off-road", unit: "km/h", digits: 0 },
  cruise_kmh: { label: "Cruise", unit: "km/h", digits: 0 },
  ground_m: { label: "Sight · ground", unit: "m", digits: 0 },
  low_air_m: { label: "Sight · low air", unit: "m", digits: 0 },
  high_air_m: { label: "Sight · high air", unit: "m", digits: 0 },
  visibility: { label: "Visibility", unit: "×", digits: 1 },
  damage: { label: "Damage", unit: "", digits: 0 },
  penetration: { label: "Penetration", unit: "", digits: 0 },
  aim_s: { label: "Aim", unit: "s", digits: 1 },
  reload_s: { label: "Reload", unit: "s", digits: 1 },
} as const;
export type SheetStatKey = keyof typeof SHEET_STATS;

type Figures = Partial<Record<SheetStatKey, number>>;

const movementOf = (mobility: Mobility): Figures => {
  if ("air" in mobility) return { cruise_kmh: mobility.air.cruise_kmh };
  const ground = Object.values(mobility)[0];
  return { road_kmh: ground.road_kmh, offroad_kmh: ground.offroad_kmh };
};

const sensesOf = (t: UnitType): Figures => ({
  ground_m: t.sensors.ground_m,
  low_air_m: t.sensors.low_air_m,
  high_air_m: t.sensors.high_air_m,
  visibility: t.concealment,
});

const ammoOf = (row: WeaponRow): Figures => ({
  damage: row.damage as number,
  penetration: row.penetration,
  aim_s: row.aim_s,
  reload_s: row.reload_s,
});

const bodyOf = (catalog: SessionCatalog, id: string): SheetBody =>
  catalog.units.hull(id) ? "hull" : "squad";

/** The largest value of each figure per body, over every unit type. */
const BESTS = new WeakMap<SessionCatalog, Record<SheetBody, Figures>>();
function bests(catalog: SessionCatalog): Record<SheetBody, Figures> {
  let found = BESTS.get(catalog);
  if (found) return found;
  found = { squad: {}, hull: {} };
  for (const t of catalog.units.view.units) {
    const best = found[bodyOf(catalog, t.id)];
    const rows = t.mounts.flatMap((m) => m.weapons).map((w) => catalog.weapons[w]);
    for (const figures of [movementOf(t.mobility), sensesOf(t), ...rows.map(ammoOf)])
      for (const [key, value] of Object.entries(figures) as [SheetStatKey, number][])
        best[key] = Math.max(best[key] ?? 0, value);
  }
  BESTS.set(catalog, found);
  return found;
}

const stats = (figures: Figures, best: Figures): SheetStat[] =>
  (Object.entries(figures) as [SheetStatKey, number][]).map(([key, value]) => ({
    key,
    value,
    scale: best[key] ? value / best[key] : 0,
  }));

/** What else a type does, in words. */
function capabilitiesOf(t: UnitType): string[] {
  const c = t.capabilities;
  return [
    c.carriage && `Carries ${c.carriage.seats} men`,
    c.deploy && `Deploys in ${c.deploy.seconds} s`,
    c.supply && `Supply ${c.supply.stock}`,
    c.active_protection && `${c.active_protection.name}: ${c.active_protection.capacity} charges`,
    c.flares && `Flares ×${c.flares.capacity}`,
    c.radar_emitter && "Radar",
    c.radar_receiver && `Finds radars within ${c.radar_receiver.range_m} m`,
    c.air?.afterburner && "Afterburner",
  ].filter((phrase): phrase is string => typeof phrase === "string");
}

export function unitSheet(catalog: SessionCatalog, card: UnitCard): UnitSheet {
  const { units, weapons } = catalog;
  if (!units.has(card.id))
    return {
      card,
      planned: true,
      body: null,
      movement: [],
      senses: [],
      capabilities: [],
      mounts: card.planned_weapons.map((w) => ({
        name: w.name.toUpperCase(),
        icon: w.icon,
        kinds: [],
      })),
    };
  const t = units.type(card.id);
  const body = bodyOf(catalog, card.id);
  const best = bests(catalog)[body];
  const hull = units.hull(card.id);
  const hps = units.slots(card.id).map((s) => units.soldier(s).hp);
  return {
    card,
    planned: false,
    body,
    ...(hull
      ? {
          hp: hull.hp,
          armor: {
            front: hull.armor.front,
            side: hull.armor.side,
            rear: hull.armor.rear,
            roof: hull.armor.roof,
          },
        }
      : { men: hps.length, hpPerMan: [Math.min(...hps), Math.max(...hps)] }),
    movement: stats(movementOf(t.mobility), best),
    senses: stats(sensesOf(t), best),
    capabilities: capabilitiesOf(t),
    mounts: t.mounts.map((mount, k) => ({
      name: equipmentName(t.mounts, k, catalog),
      icon: weapons[mount.weapons[0]].icon,
      kinds: mount.weapons.map((id) => {
        const row = weapons[id];
        return {
          weapon: id,
          name: row.name,
          description: row.description,
          ammo: row.ammo,
          firesRegardless: row.derived.fires_regardless,
          reach: row.derived.reach,
          stats: stats(ammoOf(row), best),
        };
      }),
    })),
  };
}
