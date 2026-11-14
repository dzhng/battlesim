import { shippedRules } from "@web/shippedRules";
import { cleanup, render } from "@testing-library/react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { afterEach, expect, test } from "vitest";
import { iconSvg } from "../src/battle/present/icons";
import { InfoPanel } from "../src/battle/present/infoPanel";
import { ownPanel, type PanelRules } from "../src/battle/present/panelRows";
import { TEST_CATALOG } from "./catalog";

afterEach(cleanup);

const panelFor = (engagement: string) => {
  const unit = specimenUnit(TEST_CATALOG, "test_rifle", { engagement });
  return ownPanel(TEST_CATALOG.units, unit, [unit], shippedRules as unknown as PanelRules);
};

/** The hold-fire mark as the page parses it. */
const holdFireMark = () => {
  const icon = document.createElement("span");
  icon.innerHTML = iconSvg(hudIcon("hold_fire"))!;
  return icon.innerHTML;
};

/** Hold fire is the unit's order, not one weapon's: a unit told to hold
 *  fire says so in its own state row on the full card and by its name on the
 *  compact card, in the same mark as the command that set it. A unit firing
 *  at will says nothing. */
test("a unit holding fire says so on its full and compact cards; one firing at will doesn't", () => {
  const holding = panelFor("return_fire_only");
  const full = render(<InfoPanel panel={holding} zoom="default" />);
  const row = full.container.querySelector('[data-state="hold_fire"]');
  expect(row?.textContent).toMatch(/HOLD/);
  cleanup();

  const compact = render(<InfoPanel panel={holding} zoom="compressed" />);
  const status = compact.getByLabelText("unit status");
  expect([...status.children].map((c) => c.innerHTML)).toContain(holdFireMark());
  cleanup();

  const free = render(<InfoPanel panel={panelFor("fire_at_will")} zoom="compressed" />);
  expect(free.container.querySelector('[data-state="hold_fire"]')).toBeNull();
  expect(free.queryByLabelText("unit status")).toBeNull();
});
