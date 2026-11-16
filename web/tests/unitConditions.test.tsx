// A condition on the whole unit (its fire order, suppression, entering a
// building) is said once, as the unit's own row and as an icon by its name
// that every card keeps, compact too; never repeated on each weapon's row.
import { shippedRules } from "@web/shippedRules";
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { InfoPanel } from "../src/battle/present/infoPanel";
import { iconSvg } from "../src/battle/present/icons";
import { ownPanel, type PanelRules } from "../src/battle/present/panelRows";
import type { OwnUnitView } from "../src/battle/sim/observationView";
import { TEST_CATALOG } from "./catalog";

afterEach(cleanup);

const RULES = shippedRules as unknown as PanelRules;

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
