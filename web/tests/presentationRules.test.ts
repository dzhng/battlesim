// @vitest-environment node
// Panels read weapon rows as the catalog resolves them: a row that extends
// another (a heavier tank's AP shell, a named ATGM) inherits its icon and
// full load, so its panel row draws both.
import game from "@fixtures/game.json";
import { expect, test } from "vitest";
import { presentationRules } from "@web/battle/catalog/compose";
import { arrivingUnit, ownPanel, type PanelRules } from "@web/battle/present/panelRows";
import { weaponIcon } from "@packages/scene-assets/src/icons";
import { TEST_CATALOG } from "./catalog";

const raw = game as unknown as PanelRules & { weapons: Record<string, { extends?: string }> };

/** A unit type with a mount firing a row that extends another. */
const inheriting = TEST_CATALOG.units.ids.find((id) =>
  TEST_CATALOG.units.type(id).mounts.some((m) => m.weapons.some((w) => raw.weapons[w]?.extends)),
);

test("a row that extends another draws its inherited icon and full load", () => {
  expect(inheriting).toBeDefined();
  const unit = arrivingUnit(TEST_CATALOG, inheriting!);
  const panel = ownPanel(TEST_CATALOG.units, unit, [unit], presentationRules(raw, TEST_CATALOG));
  const rows = panel.weapons.filter((w) => w.key !== "protection");
  expect(rows.length).toBeGreaterThan(0);
  // Each row's icon is its loaded (first) kind's, as the catalog resolves
  // it: never a path made of an inherited, absent field.
  const mounts = TEST_CATALOG.units.type(inheriting!).mounts;
  expect(rows.map((w) => w.icon)).toEqual(
    mounts.map((m) => weaponIcon(TEST_CATALOG.weapons[m.weapons[0]].icon)),
  );
  const counted = rows.filter((w) => w.kinds.some((k) => typeof k.count === "number"));
  expect(counted.filter((w) => w.fill === null).map((w) => w.name)).toEqual([]);
});
