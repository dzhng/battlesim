import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { SkirmishStatus } from "@apps/battle-lab/src/battleStatus";
import type { SkirmishView } from "@web/battle/sim/observation";
afterEach(cleanup);
const match: SkirmishView = {
  phase: "preparation",
  ready: [false, false],
  preparationRemainingS: 59.2,
  credits: 1000,
  occupiedSlots: 0,
  maxUnits: 30,
  scores: [0, 0],
  result: null,
  pending: [],
  objectives: [],
};
test("the public match readout follows preparation, both scores and the terminal result", () => {
  const view = render(<SkirmishStatus match={match} />);
  expect(view.getByText("PREPARATION · 60 s")).toBeTruthy();
  view.rerender(<SkirmishStatus match={{ ...match, phase: "active", scores: [345.8, 120.2] }} />);
  expect(view.getByLabelText("Your victory score").textContent).toBe("345 / 1,000");
  expect(view.getByLabelText("Enemy victory score").textContent).toBe("120 / 1,000");
  view.rerender(<SkirmishStatus match={{ ...match, phase: "finished", result: "red" }} />);
  expect(view.getByText("DEFEAT")).toBeTruthy();
});
