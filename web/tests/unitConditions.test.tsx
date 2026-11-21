// A condition on the whole unit (its fire order, suppression, entering a
// building, a reserve running out) is said once, as the unit's own row and as an icon by its name
// that every card keeps, compact too; never repeated on each weapon's row.
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { InfoPanel } from "../src/battle/present/infoPanel";
import { iconSvg } from "../src/battle/present/icons";
import { LOW_RESERVE, ownPanel, type PanelRules } from "../src/battle/present/panelRows";
import type { OwnUnitView } from "../src/battle/sim/observationView";
import { TEST_CATALOG, TEST_RULES } from "./catalog";

afterEach(cleanup);

const RULES = TEST_RULES as unknown as PanelRules;

/** An icon's markup as the page serialises it. */
function markup(icon: string): string {
  const svg = iconSvg(icon);
  if (!svg) throw new Error(`missing icon ${icon}`);
  const holder = document.createElement("span");
  holder.innerHTML = svg;
  return holder.innerHTML;
}

function card(patch: Partial<OwnUnitView>, reason: string) {
  const unit = specimenUnit(TEST_CATALOG, "test_rifle", patch, [{ reason }, { reason }]);
  const { container } = render(
    <InfoPanel panel={ownPanel(TEST_CATALOG.units, unit, [unit], RULES)} zoom="compressed" />,
  );
  const shows = (selector: string, icon: string) =>
    [...container.querySelectorAll(selector)].filter((el) => el.innerHTML.includes(markup(icon)))
      .length;
  return { container, shows };
}

test("holding fire is the unit's row and name icon, not a mark on every weapon", () => {
  const { container, shows } = card({ engagement: "return_fire_only" }, "holding_fire");
  const mark = hudIcon("hold_fire");
  expect(container.querySelectorAll(".ro-weapon")).toHaveLength(2);
  expect(shows(".ro-weapon", mark)).toBe(0);
  expect(shows(".ro-state", mark)).toBe(1);
  expect(shows(".ro-name .ro-status-icon", mark)).toBe(1);

  const free = card({ engagement: "fire_at_will" }, "no_compatible_target");
  expect(free.shows(".ro-body", mark)).toBe(0);
});

test("entering a building is the unit's row; its weapons wait unmarked", () => {
  const { container, shows } = card(
    {
      hold: {
        host: { kind: "building", id: 3, center: [0, 0], half: [10, 10], seats: 12 },
        phase: "entering",
        progress: 0.5,
      },
    } as Partial<OwnUnitView>,
    "changing_position",
  );
  expect(container.querySelector('.ro-state[data-state="entering"]')).not.toBeNull();
  expect(shows(".ro-weapon", stateIcon("building"))).toBe(0);
});

test("suppression shows by the name, as pinning does", () => {
  for (const tier of ["suppressed", "pinned"] as const) {
    const { shows } = card({ suppression: tier }, "no_compatible_target");
    expect(shows(".ro-name .ro-status-icon", stateIcon(tier)), tier).toBe(1);
  }
});

test("returning to base is the unit's row and stands by its name on every card", () => {
  const mark = stateIcon("withdrawing");
  const { shows } = card({ withdrawing: true }, "no_compatible_target");
  expect(shows(".ro-state", mark)).toBe(1);
  expect(shows(".ro-name .ro-status-icon", mark)).toBe(1);
  expect(card({ withdrawing: false }, "no_compatible_target").shows(".ro-body", mark)).toBe(0);
});

test("a reserve running out is the unit's row and stands by its compact card's name", () => {
  const compact = (unit: OwnUnitView) => {
    const { container } = render(
      <InfoPanel panel={ownPanel(TEST_CATALOG.units, unit, [unit], RULES)} zoom="compressed" />,
    );
    const states = [...container.querySelectorAll<HTMLElement>(".ro-state")];
    const byName = [...container.querySelectorAll(".ro-name .ro-status-icon")];
    cleanup();
    return { states, byName };
  };
  const rows = TEST_CATALOG.units.type("test_tank").mounts[0].weapons;
  const full = rows.map((r) => RULES.weapons[r].ammo as number);
  const tank = (ammo: number[]) =>
    specimenUnit(TEST_CATALOG, "test_tank", {}, [{ ammo, loaded: null }]);
  const truckFull = TEST_CATALOG.units.type("test_supply").capabilities.supply!.stock;
  const truck = (stock: number) => specimenUnit(TEST_CATALOG, "test_supply", { stock });
  const cases: [string, OwnUnitView, string | null][] = [
    ["full tank", tank(full), null],
    ["low tank", tank(full.map((n) => Math.floor(n * LOW_RESERVE))), hudIcon("low_ammo")],
    // Full HE does not hide the empty AP rack.
    ["out tank", tank([0, full[1]]), hudIcon("no_ammo")],
    ["full truck", truck(truckFull), null],
    ["low truck", truck(Math.floor(truckFull * LOW_RESERVE)), stateIcon("stock")],
    ["out truck", truck(0), stateIcon("supply_blocked")],
  ];
  for (const [label, unit, icon] of cases) {
    const { states, byName } = compact(unit);
    const warnings = states.filter((el) => el.dataset.tone === "warn");
    if (!icon) {
      expect(byName, label).toHaveLength(0);
      expect(warnings, label).toHaveLength(0);
      continue;
    }
    const drawn = (els: Element[]) => els.filter((el) => el.innerHTML.includes(markup(icon)));
    expect(drawn(byName), `${label} by the name`).toHaveLength(1);
    expect(drawn(warnings), `${label} row`).toHaveLength(1);
  }
});

test("spent countermeasures are not a reserve running out", () => {
  // Full rounds, but its Trophy, flares and smoke all spent: the tank still
  // fights, so no warning stands by its name. Each spent charge says so on
  // its own row.
  const unit = specimenUnit(TEST_CATALOG, "test_tank", {
    protection: { charges: 0, cooldown: null },
    flares: { charges: 0, cooldown: null },
    smoke: 0,
  });
  const { container } = render(
    <InfoPanel panel={ownPanel(TEST_CATALOG.units, unit, [unit], RULES)} zoom="compressed" />,
  );
  expect(container.querySelectorAll(".ro-name .ro-status-icon")).toHaveLength(0);
});
