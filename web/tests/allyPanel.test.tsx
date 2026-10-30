// An ally's unit in the battle's callouts: its normal info panel, read-only,
// with a line naming the player that owns it.
import { shippedRules } from "@web/shippedRules";
import { cleanup, render } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, expect, test } from "vitest";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { TEST_CATALOG, WithTestCatalog } from "./catalog";
import type { PanelRules } from "../src/battle/present/panelRows";
import { ReadoutLayer, type ReadoutLayerHandle } from "../src/battle/present/readouts";
import { allyOwners } from "../src/battle/input/controlled";
import type { ObservationView } from "../src/battle/sim/observationView";

afterEach(cleanup);

test("an ally's panel names its owner and has no order controls; your own has neither", () => {
  const mine = specimenUnit(TEST_CATALOG, "test_tank", { id: 1, owner: 0 });
  const ally = specimenUnit(TEST_CATALOG, "test_tank", { id: 2, owner: 1 });
  const observation = {
    own: [mine, ally],
    skirmish: {
      players: [
        { index: 0, id: "blue-1", controller: "human" },
        { index: 1, id: "blue-2", controller: "active_ai" },
      ],
    },
  } as unknown as ObservationView;
  const view = render(
    <ReadoutLayer
      own={observation.own}
      allies={allyOwners(observation, "blue-1")}
      rules={shippedRules as unknown as PanelRules}
      selected={[2]}
      handle={createRef<ReadoutLayerHandle>()}
    />,
    { wrapper: WithTestCatalog },
  );
  const card = (id: number) => view.container.querySelector<HTMLElement>(`[data-unit="${id}"]`)!;
  expect(card(2).dataset.owner).toBe("ally");
  expect(card(2).textContent).toContain("BLUE 2 · AI");
  // Read-only: the panel is the same panel as your unit's, with no control in it.
  expect(card(2).textContent).toContain("MAIN GUN");
  expect(card(2).querySelector("button, [role=button], input")).toBeNull();
  expect(card(1).dataset.owner).toBe("own");
  expect(card(1).textContent).not.toContain("BLUE");
});
