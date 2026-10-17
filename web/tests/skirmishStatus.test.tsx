import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { SkirmishStatus } from "@apps/battle-lab/src/battleStatus";
import type { SkirmishView } from "@web/battle/sim/observationView";
afterEach(cleanup);
const match: SkirmishView = {
  phase: "preparation",
  players: [
    {
      index: 0,
      id: "blue-1",
      controller: "human",
      ready: false,
      credits: 1000,
      unitCount: 0,
      cap: 30,
      pending: [],
    },
  ],
  preparationRemainingS: 59.2,
  scores: [0, 0],
  result: null,
  objectives: [],
  report: null,
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

test("contested objective flags expose a flashing state", () => {
  const view = render(
    <SkirmishStatus
      match={{
        ...match,
        objectives: [
          {
            id: "alpha",
            center: [0, 0],
            radiusM: 50,
            owner: "blue",
            contested: true,
            capturing: null,
            captureProgress: 0,
          },
        ],
      }}
    />,
  );
  expect(view.getByLabelText("Objective 1: contested").getAttribute("data-contested")).toBe("true");
});
