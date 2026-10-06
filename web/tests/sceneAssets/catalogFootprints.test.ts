// @vitest-environment node
// The shipped catalog against the simulation: every scenery appearance that
// stands for a prop is authored to a box the simulation actually places, so the
// validator's fit check measures the art against the rule, not against itself.
// Buildings are no appearance: the asset check holds each one's template art
// to its template (`templateSource.ts`).
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { fixtureAuthority } from "@packages/scene-assets/src/authority.ts";
import { SCENERY_KINDS, propsDrawnBy } from "@packages/scene-assets/src/scenery.ts";
import { WEAPONS } from "@packages/scene-assets/src/shippedUnits.ts";
import type { Catalog } from "@packages/scene-assets/src/schema.ts";
import { UnitCatalog, type CatalogView } from "@packages/scene-assets/src/units.ts";
import { loadMap } from "@web/maps/node";

const read = (path: string) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const catalog = read("../../../assets/catalog.json") as Catalog;
const game = read("../../../fixtures/game.json");
const same = (a: readonly number[], b: readonly number[]) =>
  a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) < 1e-9);

const villageMap = loadMap("village").definition;
const buildings = villageMap.buildings!.flatMap((b) => b.geometry.parts.map((p) => p.half_extents));
/** Every unit type with a hull leaves a wreck on its hull box. */
const units = new UnitCatalog(read("../../../fixtures/catalog.json") as CatalogView);
const hulls = units.ids.flatMap((id) => {
  const hull = units.hull(id);
  return hull ? [hull.half_extents_m] : [];
});
/** The remains a building leaves, at the ruin height. */
const ruin = units.view.props.building.destroyed;
const ruinHalf = typeof ruin === "object" ? ruin.into.height_m / 2 : NaN;

const propEntries = Object.entries(catalog.appearances).filter(
  ([, e]) => e.unit === "scenery" && SCENERY_KINDS[e.scenery ?? ""]?.footprint.kind === "prop",
);

test("the catalog ships an appearance for every accepted prop kind a scenery kind draws", () => {
  const kinds = new Set(propEntries.map(([, e]) => e.scenery));
  // What the catalog's prop types are drawn by. Trees are the forest's, and a
  // building's parts its template's art.
  const simulated = Object.values(units.view.props)
    .filter((t) => t.appearance.status !== "systems_only")
    .map((t) => t.appearance.drawn_by)
    .filter((by) => by !== "forest" && by !== "building");
  expect([...kinds].sort()).toEqual([...new Set(simulated)].sort());
});

test("every vehicle's wreck has an appearance on its hull box", () => {
  const wrecks = propEntries
    .filter(([, e]) => e.scenery === "wreck")
    .map(([name, e]) => ({
      name,
      half: e.footprint_half_m!,
      tolerance: e.tolerances?.footprint_m ?? 0,
    }));
  for (const h of hulls)
    expect(
      wrecks.some(({ half, tolerance }) => h.every((v, i) => Math.abs(v - half[i]) <= tolerance)),
      `${h}`,
    ).toBe(true);
});

test("a wreck is its vehicle's hull box and a ruin a building's plan at the ruin height", () => {
  for (const [name, e] of propEntries) {
    if (e.scenery === "wreck")
      expect(
        hulls.some((h) => same(h, e.footprint_half_m!)),
        name,
      ).toBe(true);
    if (e.scenery === "ruin")
      expect(
        buildings.some((b) => same([b[0], b[1], ruinHalf], e.footprint_half_m!)),
        name,
      ).toBe(true);
  }
});

test("remains with art of their own stand on the destroyed body's plan at the remains' height", () => {
  // A parked car is fitted from the car's box and its wreck from the same plan at
  // the height the catalog leaves, so the swap neither grows nor moves the body.
  const boxesOf = (kind: string) =>
    propEntries.filter(([, e]) => e.scenery === kind).map(([, e]) => e.footprint_half_m!);
  let checked = 0;
  for (const [id, t] of Object.entries(units.view.props)) {
    if (typeof t.destroyed !== "object") continue;
    const { prop, height_m } = t.destroyed.into;
    const remains = units.view.props[prop].appearance.drawn_by;
    if (propsDrawnBy(units.view.props, remains).length !== 1) continue; // shared art is stretched
    for (const b of boxesOf(t.appearance.drawn_by)) {
      checked++;
      expect(
        boxesOf(remains).some((r) => same(r, [b[0], b[1], height_m / 2])),
        `${id} -> ${prop}`,
      ).toBe(true);
    }
  }
  expect(checked).toBeGreaterThan(0);
});

test("which prop types a scenery kind draws is the prop catalog's drawn_by, both ways", () => {
  // Every scenery kind that stands for a prop draws some prop type, and every
  // accepted prop type is drawn by such a kind, by its building or by a forest.
  for (const [kind, rule] of Object.entries(SCENERY_KINDS))
    if (rule.footprint.kind === "prop")
      expect(propsDrawnBy(units.view.props, kind), kind).not.toEqual([]);
  for (const [id, t] of Object.entries(units.view.props)) {
    if (t.appearance.status === "systems_only") continue;
    const by = t.appearance.drawn_by;
    const drawn =
      by === "building" || by === "forest" || SCENERY_KINDS[by]?.footprint.kind === "prop";
    expect(drawn, `${id} drawn by ${by}`).toBe(true);
  }
  expect(propsDrawnBy(units.view.props, "wreck")).toEqual([
    "heavy_wreck",
    "light_wreck",
    "medium_wreck",
  ]);
});

test("a template building ends by the building row's own rule, and by one rule only", () => {
  const fixture = { physics: game.physics, forests: game.forests };
  const into = typeof ruin === "object" ? ruin.into : undefined;
  expect(fixtureAuthority(fixture, units).collapse).toEqual({
    min_height_m: into!.height_m,
    height_fraction: into!.building!.height_fraction,
    max_height_m: into!.building!.max_height_m,
    max_floors: into!.building!.collapse_max_floors,
  });
  const taller = structuredClone(units.view.props.building);
  if (typeof taller.destroyed === "object")
    taller.destroyed.into.building!.collapse_max_floors += 2;
  const both = new UnitCatalog({ ...units.view, props: { ...units.view.props, taller } });
  expect(() => fixtureAuthority(fixture, both)).toThrow(/building.*taller|taller.*building/);
});

test("the shipped view carries every weapon row a mount names, resolved", () => {
  for (const t of units.view.units)
    for (const m of t.mounts)
      for (const w of m.weapons) {
        expect(WEAPONS[w], `${t.id} ${m.name} ${w}`).toBeDefined();
        expect(WEAPONS[w], w).not.toHaveProperty("extends");
        expect(WEAPONS[w].speed_mps, w).toBeGreaterThan(0);
      }
});

test("every prop appearance declares a positive box", () => {
  for (const [name, e] of propEntries) {
    expect(e.footprint_half_m, name).toHaveLength(3);
    expect(
      e.footprint_half_m!.every((v) => v > 0),
      name,
    ).toBe(true);
  }
});

// Physical prototypes cannot pass the appearance gate by borrowing an existing
// scenery binding. Removing this status makes the ordinary strict gate apply.
test("systems-only bodies have no accepted appearance binding", () => {
  for (const [id, t] of Object.entries(units.view.props)) {
    if (t.appearance.status !== "systems_only") continue;
    expect(
      t.appearance.drawn_by === "building" ||
        t.appearance.drawn_by === "forest" ||
        t.appearance.drawn_by in SCENERY_KINDS,
      id,
    ).toBe(false);
    expect(
      propEntries.some(([, e]) => e.scenery === t.appearance.drawn_by),
      id,
    ).toBe(false);
  }
});

test("the playable village's authored props and their remains have accepted bindings", () => {
  const placed = [
    ...villageMap.props.map((p: { kind: string }) => p.kind),
    ...villageMap.buildings!.map((b: { kind: string }) => b.kind),
  ];
  for (let i = 0; i < placed.length; i++) {
    const id = placed[i];
    const t = units.view.props[id];
    expect(t.appearance.status, id).toBeUndefined();
    if (typeof t.destroyed === "object" && !placed.includes(t.destroyed.into.prop))
      placed.push(t.destroyed.into.prop);
  }
});

test("the playable village enables generated floor blockers only with accepted drawing", () => {
  for (const [kindField, densityField] of [
    ["log", "logs_per_ha"],
    ["boulder", "boulders_per_ha"],
  ]) {
    if (game.forests.rule[densityField] <= 0) continue;
    const kind = game.forests[kindField];
    const t = units.view.props[kind];
    expect(t.appearance.status, `${kind}: active generated cover must draw`).toBeUndefined();
    expect(
      propEntries.some(([, e]) => e.scenery === t.appearance.drawn_by),
      kind,
    ).toBe(true);
  }
});
