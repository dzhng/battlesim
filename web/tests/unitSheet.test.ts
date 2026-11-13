// @vitest-environment node
// The Armory's sheet model: what a card's sheet says, read from the game's
// own owners (the catalog, its published weapon facts and the panel's mount
// names). Pure, so it is tested without a page. Every game card, or
// fabricated catalogs; never one roster entry or one stat.
import { expect, test } from "vitest";
import {
  UnitCatalog,
  type CatalogView,
  type UnitCard,
  type UnitType,
  type WeaponRow,
} from "@packages/scene-assets/src/units";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { equipmentName } from "@web/battle/present/panelRows";
import { unitSheet, type SheetStat } from "@web/battle/present/unitSheet";
import { TEST_CATALOG, UNITS, WEAPONS } from "./catalog";

/** The test catalog with `units`, `cards`, soldiers and weapons added. */
function fabricated(
  extra: {
    units?: UnitType[];
    cards?: UnitCard[];
    soldiers?: CatalogView["soldiers"];
    weapons?: Record<string, WeaponRow>;
  },
  base: SessionCatalog = TEST_CATALOG,
): SessionCatalog {
  const view = base.units.view;
  return {
    ...base,
    units: new UnitCatalog({
      ...view,
      units: [...view.units, ...(extra.units ?? [])],
      cards: [...view.cards, ...(extra.cards ?? [])],
      soldiers: { ...view.soldiers, ...extra.soldiers },
    }),
    weapons: { ...base.weapons, ...extra.weapons },
  };
}

const cardOf = (type: UnitType, over: Partial<UnitCard> = {}): UnitCard => ({
  id: type.id,
  name: type.name,
  description: type.description,
  family: type.family,
  cost: type.cost,
  roster: { factions: ["us"], family_name: type.name, category: "veh", variant: "Base" },
  disabled_reason: null,
  planned_weapons: [],
  ...over,
});

const squadId = UNITS.ids.find((id) => UNITS.isInfantry(id))!;
const hullId = UNITS.ids.find((id) => UNITS.hull(id) !== null && UNITS.type(id).mounts.length)!;

test("every card in the game catalog yields a sheet", () => {
  for (const card of UNITS.cards) {
    const sheet = unitSheet(TEST_CATALOG, card);
    expect(sheet.card, card.id).toBe(card);
    expect(sheet.planned, card.id).toBe(!UNITS.has(card.id));
    if (sheet.planned) continue;
    expect(sheet.body, card.id).toBe(UNITS.hull(card.id) ? "hull" : "squad");
    for (const stat of [
      ...sheet.movement,
      ...sheet.senses,
      ...sheet.mounts.flatMap((m) => m.kinds.flatMap((k) => k.stats)),
    ]) {
      expect(Number.isFinite(stat.value), `${card.id} ${stat.key}`).toBe(true);
      expect(stat.scale, `${card.id} ${stat.key}`).toBeGreaterThanOrEqual(0);
      expect(stat.scale, `${card.id} ${stat.key}`).toBeLessThanOrEqual(1);
    }
  }
});

test("each mount lists exactly its weapons' kinds under its equipment name", () => {
  for (const id of UNITS.ids) {
    const type = UNITS.type(id);
    const sheet = unitSheet(TEST_CATALOG, cardOf(type));
    expect(sheet.mounts.map((m) => m.name)).toEqual(
      type.mounts.map((_, k) => equipmentName(type.mounts, k, TEST_CATALOG)),
    );
    sheet.mounts.forEach((mount, k) => {
      expect(mount.kinds.map((kind) => kind.weapon)).toEqual(type.mounts[k].weapons);
      for (const kind of mount.kinds) {
        const row = WEAPONS[kind.weapon];
        expect(kind.name).toBe(row.name);
        expect(kind.description).toBe(row.description);
        expect(kind.ammo).toBe(row.ammo);
        expect(kind.reach).toEqual(row.derived.reach);
      }
    });
  }
});

test("the fires-regardless mark follows the published fact, not default", () => {
  // A finite default gun: the rules flag it default, but it does not plink,
  // so its published fact says no, and so does the sheet.
  const tank = UNITS.type(hullId);
  const marks = (over: Partial<WeaponRow>, fires: boolean) => {
    const rows = Object.fromEntries(
      tank.mounts
        .flatMap((m) => m.weapons)
        .map((id): [string, WeaponRow] => [
          id,
          {
            ...WEAPONS[id],
            ...over,
            derived: { ...WEAPONS[id].derived, fires_regardless: fires },
          },
        ]),
    );
    return unitSheet(fabricated({ weapons: rows }), cardOf(tank))
      .mounts.flatMap((m) => m.kinds)
      .map((kind) => kind.firesRegardless);
  };
  const finite = marks({ default: true, ammo: 240 }, false);
  expect(finite.length).toBeGreaterThan(0);
  expect(finite.every((mark) => !mark)).toBe(true);
  // The sheet never second-guesses the fact from the row's own fields.
  expect(marks({ default: true, ammo: "unlimited" }, false).every((mark) => !mark)).toBe(true);
  expect(marks({ default: false, ammo: 240 }, true).every((mark) => mark)).toBe(true);
  // And every published fact reaches the sheet as published.
  for (const id of UNITS.ids)
    for (const kind of unitSheet(TEST_CATALOG, cardOf(UNITS.type(id))).mounts.flatMap(
      (m) => m.kinds,
    ))
      expect(kind.firesRegardless, kind.weapon).toBe(WEAPONS[kind.weapon].derived.fires_regardless);
});

test("squad HP per man is a range when soldiers differ", () => {
  const squad = UNITS.type(squadId);
  const [first, ...rest] = UNITS.slots(squadId);
  const soldier = UNITS.soldier(first);
  const tough = { ...soldier, hp: soldier.hp * 2 };
  const mixed: UnitType = {
    ...squad,
    id: "fabricated_mixed_squad",
    body: { squad: { slots: ["fabricated_tough", ...rest] } },
  };
  const sheet = unitSheet(
    fabricated({ units: [mixed], soldiers: { fabricated_tough: tough } }),
    cardOf(mixed),
  );
  const hps = [tough.hp, ...rest.map((s) => UNITS.soldier(s).hp)];
  expect(sheet.body).toBe("squad");
  expect(sheet.men).toBe(rest.length + 1);
  expect(sheet.hpPerMan).toEqual([Math.min(...hps), Math.max(...hps)]);
  // Alike soldiers read one value: the range closes on it.
  const alike = unitSheet(TEST_CATALOG, cardOf(squad));
  const alikeHp = UNITS.soldier(first).hp;
  if (UNITS.slots(squadId).every((s) => UNITS.soldier(s).hp === alikeHp))
    expect(alike.hpPerMan).toEqual([alikeHp, alikeHp]);
});

test("the best unit of a body kind fills its bar, and squads scale against squads", () => {
  const road = (stats: SheetStat[]) => stats.find((s) => s.key === "road_kmh");
  const squad = UNITS.type(squadId);
  const hull = UNITS.type(hullId);
  const speed = (t: UnitType) => ("air" in t.mobility ? 0 : Object.values(t.mobility)[0].road_kmh);
  // A squad faster than every squad, yet slower than the fastest hull.
  const fastestHull = Math.max(
    ...UNITS.ids.filter((id) => UNITS.hull(id)).map((id) => speed(UNITS.type(id))),
  );
  const fastestSquad = Math.max(
    ...UNITS.ids.filter((id) => UNITS.isInfantry(id)).map((id) => speed(UNITS.type(id))),
  );
  const sprint = fastestSquad + 1;
  expect(sprint).toBeLessThan(fastestHull);
  const sprinter: UnitType = {
    ...squad,
    id: "fabricated_sprinters",
    mobility: { foot: { road_kmh: sprint, offroad_kmh: sprint } },
  };
  const catalog = fabricated({ units: [sprinter] });
  expect(road(unitSheet(catalog, cardOf(sprinter)).movement)).toEqual({
    key: "road_kmh",
    value: sprint,
    scale: 1,
  });
  // The old fastest squad now fills part of its bar, by the sprinters alone.
  const before = road(unitSheet(catalog, cardOf(squad)).movement)!;
  expect(before.scale).toBeCloseTo(speed(squad) / sprint);
  // A hull is measured against hulls, untouched by the squad.
  const hullRoad = road(unitSheet(catalog, cardOf(hull)).movement);
  if (hullRoad) expect(hullRoad.scale).toBeCloseTo(speed(hull) / fastestHull);
});

test("a card with no type yields a planned sheet with its planned weapons", () => {
  const planned: UnitCard = {
    id: "fabricated_planned_drone",
    name: "Drone AT",
    description: "Anti-armor",
    family: "fabricated_drone",
    cost: 90,
    roster: { factions: ["us"], family_name: "Drone", category: "sup", variant: "AT" },
    disabled_reason: "Drone mechanics deferred",
    planned_weapons: [
      { name: "AT Warhead", icon: "ap_shell" },
      { name: "Camera", icon: "rifle" },
    ],
  };
  const sheet = unitSheet(fabricated({ cards: [planned] }), planned);
  expect(sheet.planned).toBe(true);
  expect(sheet.body).toBeNull();
  expect(sheet.movement).toEqual([]);
  expect(sheet.senses).toEqual([]);
  expect(sheet.mounts.map((m) => [m.name, m.icon, m.kinds])).toEqual(
    planned.planned_weapons.map((w) => [w.name.toUpperCase(), w.icon, []]),
  );
});
