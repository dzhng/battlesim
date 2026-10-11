import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { MatchResultOverlay } from "@apps/battle-lab/src/MatchResultOverlay";
import { WithTestCatalog } from "./catalog";
import type { SkirmishView } from "@web/battle/sim/observationView";

afterEach(cleanup);

const match: SkirmishView = {
  phase: "finished",
  ready: [true, true],
  preparationRemainingS: 0,
  credits: 420,
  occupiedSlots: 2,
  maxUnits: 30,
  scores: [700, 300],
  result: "blue",
  objectives: [],
  pending: [],
  report: {
    totals: [
      { spawned: 2, spent: 600, killed: 3, refunded: 120 },
      { spawned: 3, spent: 900, killed: 1, refunded: 0 },
    ],
    events: [
      { kind: "spawn", tick: 30, side: "blue", unitId: 0, unitKind: "test_tank", amount: 300 },
      {
        kind: "kill",
        tick: 90,
        side: "red",
        unitId: 1,
        unitKind: "test_rifle",
        victimCost: 110,
        killer: { side: "blue", unitId: 0, unitKind: "test_tank", cost: 300 },
      },
    ],
  },
};

test("finished matches show a dominant report with visual kill causality", () => {
  const view = render(<MatchResultOverlay match={match} />, { wrapper: WithTestCatalog });
  expect(view.getByRole("dialog", { name: "Match result" })).toBeTruthy();
  expect(view.getByText("VICTORY")).toBeTruthy();
  expect(view.getAllByText("RED PLAYER").length).toBeGreaterThan(0);
  expect(view.getByLabelText("Unit action timeline")).toBeTruthy();
  expect(view.getByText("300 CREDITS")).toBeTruthy();
  expect(view.getByText("110 CREDITS")).toBeTruthy();
});

test("finished report keeps the heading focused on the result", () => {
  const view = render(<MatchResultOverlay match={match} />, { wrapper: WithTestCatalog });
  expect(view.queryByText("OPERATION REPORT · BATTLE CONCLUDED")).toBeNull();
  expect(view.queryByText("FINAL SCORE · TERRITORY CONTROL")).toBeNull();
  expect(view.queryByText("FORCE ACTIONS · UNIT LEVEL")).toBeNull();
});
