import { expect, test } from "vitest";
import type { MountRow, UnitCatalog } from "@packages/scene-assets/src/units";
import { rangeRuler, type RulerRules } from "@web/battle/present/rangeRuler";

const physics = { infantry_muzzle_m: 2, infantry_aim_m: 5 };
const unit = { id: 1, kind: "fake", position: [0, 0, 0] as const };
const catalog = (mounts: Partial<MountRow>[]) =>
  ({ type: () => ({ mounts }) }) as unknown as UnitCatalog;

test("equal reaches share one sorted mark, using weapon or multi-row mount captions without duplicates", () => {
  const rules: RulerRules = {
    physics,
    weapons: {
      long: { name: "Long weapon", range_m: 30 },
      ap: { name: "AP round", range_m: 10 },
      he: { name: "HE round", range_m: 10 },
      rifle: { name: "Rifle", range_m: 10 },
      copy: { name: "Rifle", range_m: 10 },
    },
  };
  const ruler = rangeRuler(
    unit,
    [0, 0, 0],
    rules,
    catalog([
      { name: "Long mount", weapons: ["long"] },
      { name: "Cannon", weapons: ["ap", "he"] },
      { name: "Infantry", weapons: ["rifle"] },
      { name: "Duplicate caption", weapons: ["copy"] },
    ]),
  );
  expect(ruler.marks).toEqual([
    { range_m: 10, names: ["Cannon", "Rifle"], inRange: true, along_m: null },
    { range_m: 30, names: ["Long weapon"], inRange: true, along_m: null },
  ]);
});

test.each([
  {
    name: "infantry muzzle and aim heights at a raised cursor",
    mount: {},
    position: [0, 0, 0] as const,
    cursor: [3, 4, 9] as const,
  },
  {
    name: "mounted muzzle height above its unit",
    mount: { pivot_m: [0, 0, 3], muzzle_m: [0, 0, 4] },
    position: [0, 0, 10] as const,
    cursor: [3, 4, 0] as const,
  },
])("range and ground ticks use 3D distance from $name", ({ mount, position, cursor }) => {
  // A 5 m ground line and 12 m muzzle-to-aim height difference make 13 m.
  const rules: RulerRules = {
    physics,
    weapons: { short: { name: "Short", range_m: 5 }, exact: { name: "Exact", range_m: 13 } },
  };
  const ruler = rangeRuler(
    { ...unit, position },
    cursor,
    rules,
    catalog([
      { name: "Short mount", weapons: ["short"], ...mount } as Partial<MountRow>,
      { name: "Exact mount", weapons: ["exact"] },
    ]),
  );
  expect(ruler.distance_m).toBe(13);
  expect(ruler.marks).toEqual([
    { range_m: 5, names: ["Short"], inRange: false, along_m: 25 / 13 },
    { range_m: 13, names: ["Exact"], inRange: true, along_m: null },
  ]);
});

test("weapons without a positive reach, including unpowered bombs, mark no range", () => {
  const ruler = rangeRuler(
    unit,
    [100, 0, 0],
    {
      physics,
      weapons: {
        bomb: { name: "Bomb", range_m: 0 },
        disabled: { name: "Disabled", range_m: -1 },
        gun: { name: "Gun", range_m: 20 },
      },
    },
    catalog([{ name: "Weapons", weapons: ["bomb", "disabled", "missing", "gun"] }]),
  );
  expect(ruler.marks.map(({ range_m, names }) => ({ range_m, names }))).toEqual([
    { range_m: 20, names: ["Weapons"] },
  ]);
});
