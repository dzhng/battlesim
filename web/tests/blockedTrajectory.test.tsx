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

/** A launcher holding fire for a blocked trajectory (no clear launch path)
 *  keeps its weapon row in the full card and the far card's compact line,
 *  with its rounds, and wears the blocked-shot mark. */
test("a weapon held for a blocked trajectory keeps its row and its blocked-shot mark", () => {
  const unit = specimenUnit(TEST_CATALOG, "test_at", {}, [
    {},
    { ammo: [3], reason: "blocked_trajectory" },
  ]);
  const panel = ownPanel(TEST_CATALOG.units, unit, [unit], shippedRules as unknown as PanelRules);
  // The blocked-shot icon as the page parses it.
  const icon = document.createElement("span");
  icon.innerHTML = iconSvg(hudIcon("blocked_shot"))!;
  for (const zoom of ["default", "far"] as const) {
    const view = render(<InfoPanel panel={panel} zoom={zoom} />);
    const rows = view.container.querySelectorAll('.ro-weapon[data-reason="blocked_trajectory"]');
    expect(rows.length, zoom).toBe(1);
    const badge = rows[0].querySelector(":scope > .ro-badge");
    expect(badge?.innerHTML, zoom).toBe(icon.innerHTML);
    expect(rows[0].getAttribute("data-ammo"), zoom).toContain("3");
    cleanup();
  }
});
