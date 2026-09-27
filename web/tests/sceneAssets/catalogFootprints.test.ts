// @vitest-environment node
// The shipped catalog against the simulation: every static appearance that
// stands for a prop is authored to a box the simulation actually places, so the
// validator's fit check measures the art against the rule, not against itself.
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { SCENERY_KINDS } from "@packages/scene-assets/src/scenery.ts";
import type { Catalog } from "@packages/scene-assets/src/schema.ts";

const read = (path: string) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const catalog = read("../../../assets/catalog.json") as Catalog;
const village = read("../../../fixtures/village.json");
const same = (a: readonly number[], b: readonly number[]) =>
  a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) < 1e-9);

const buildings = (village.map.props as { kind: string; half_extents: number[] }[])
  .filter((p) => p.kind === "building")
  .map((p) => p.half_extents);
/** Every vehicle that leaves a wreck (the body table's movers), by its hull. */
const hulls = Object.entries(village.bodies as Record<string, { wreck?: string }>)
  .filter(([, b]) => b.wreck)
  .map(([kind]) => village.physics[`${kind}_half_extents_m`] as number[]);
const ruinHalf = village.buildings.ruin_height_m / 2;

const propEntries = Object.entries(catalog.appearances).filter(
  ([, e]) =>
    e.unit === "building" ||
    (e.unit === "scenery" && SCENERY_KINDS[e.scenery ?? ""]?.footprint.kind === "prop"),
);

test("the catalog ships an appearance for every simulation prop kind but forest trunks", () => {
  const kinds = new Set(
    propEntries.map(([, e]) => (e.unit === "building" ? "building" : e.scenery)),
  );
  // the body table's prop kinds, every wreck drawn as a wreck; trunks are the
  // forest's trees, owned by the trees slice
  const simulated = Object.keys(village.props)
    .filter((k) => k !== "trunk")
    .map((k) => (k.endsWith("_wreck") ? "wreck" : k));
  expect([...kinds].sort()).toEqual([...new Set(simulated)].sort());
});

test("every vehicle's wreck has an appearance on its hull box", () => {
  for (const h of hulls)
    expect(
      propEntries.some(([, e]) => e.scenery === "wreck" && same(h, e.footprint_half_m!)),
      `${h}`,
    ).toBe(true);
});

test("every building is authored to one of the village's placed buildings", () => {
  const houses = propEntries.filter(([, e]) => e.unit === "building");
  for (const [name, e] of houses)
    expect(
      buildings.some((b) => same(b, e.footprint_half_m!)),
      `${name} ${e.footprint_half_m}`,
    ).toBe(true);
  // and every placed building has its own appearance
  for (const b of buildings)
    expect(
      houses.some(([, e]) => same(b, e.footprint_half_m!)),
      `${b}`,
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

test("every prop appearance declares a positive box", () => {
  for (const [name, e] of propEntries) {
    expect(e.footprint_half_m, name).toHaveLength(3);
    expect(
      e.footprint_half_m!.every((v) => v > 0),
      name,
    ).toBe(true);
  }
});
