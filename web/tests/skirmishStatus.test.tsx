import matchRules from "@fixtures/skirmish.json";
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
  const view = render(
    <SkirmishStatus target={matchRules.victory_score} match={match} viewer="blue" />,
  );
  expect(view.getByText("PREPARATION · 60 s")).toBeTruthy();
  view.rerender(
    <SkirmishStatus
      target={matchRules.victory_score}
      match={{ ...match, phase: "active", scores: [345.8, 120.2] }}
      viewer="blue"
    />,
  );
  expect(view.getByLabelText("Your victory score").textContent).toBe(
    `345 / ${matchRules.victory_score.toLocaleString("en-US")}`,
  );
  expect(view.getByLabelText("Enemy victory score").textContent).toBe(
    `120 / ${matchRules.victory_score.toLocaleString("en-US")}`,
  );
  view.rerender(
    <SkirmishStatus
      target={matchRules.victory_score}
      match={{ ...match, phase: "finished", result: "red" }}
      viewer="blue"
    />,
  );
  expect(view.getByText("DEFEAT")).toBeTruthy();
});

test("contested objective flags expose a flashing state", () => {
  const view = render(
    <SkirmishStatus
      target={matchRules.victory_score}
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
      viewer="blue"
    />,
  );
  expect(view.getByLabelText("Objective 1: contested").getAttribute("data-contested")).toBe("true");
});

test("a human playing Red reads Red's score and objectives as theirs", () => {
  const objective = (id: string, owner: "blue" | "red") => ({
    id,
    center: [0, 0] as [number, number],
    radiusM: 50,
    owner,
    contested: false,
    capturing: null,
    captureProgress: 0,
  });
  const view = render(
    <SkirmishStatus
      target={matchRules.victory_score}
      match={{
        ...match,
        phase: "active",
        scores: [345.8, 120.2],
        objectives: [objective("alpha", "red"), objective("bravo", "blue")],
      }}
      viewer="red"
    />,
  );
  expect(view.getByLabelText("Your victory score").textContent).toBe(
    `120 / ${matchRules.victory_score.toLocaleString("en-US")}`,
  );
  expect(view.getByLabelText("Enemy victory score").textContent).toBe(
    `345 / ${matchRules.victory_score.toLocaleString("en-US")}`,
  );
  expect(view.getByLabelText("Objective 1: yours")).toBeTruthy();
  expect(view.getByLabelText("Objective 2: enemy")).toBeTruthy();
});

test("the score strip uses the match's target rather than a fixed skirmish target", async () => {
  const frontline = (await import("@fixtures/frontline.json")).default;
  const view = render(
    <SkirmishStatus
      target={frontline.victory_score}
      match={{ ...match, phase: "active" }}
      viewer="blue"
    />,
  );
  for (const label of ["Your victory score", "Enemy victory score"])
    expect(view.getByLabelText(label).textContent).toBe(
      `0 / ${frontline.victory_score.toLocaleString("en-US")}`,
    );
});
