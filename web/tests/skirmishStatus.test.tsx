import matchRules from "@fixtures/skirmish.json";
import { cleanup, render, type RenderResult } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { SkirmishStatus } from "@apps/battle-lab/src/battleStatus";
import type { SkirmishView } from "@web/battle/sim/observationView";
afterEach(cleanup);
/** A side's score meter reads its floored score out of the match's target. */
function expectScore(view: RenderResult, label: string, score: number, target: number) {
  const meter = view.getByRole("meter", { name: label });
  expect(Number(meter.getAttribute("aria-valuenow"))).toBe(score);
  expect(Number(meter.getAttribute("aria-valuemax"))).toBe(target);
}
const match: SkirmishView = {
  phase: "preparation",
  players: [
    {
      index: 0,
      id: "blue-1",
      controller: "human",
      ready: false,
      credits: 1000,
      incomePerMinute: 200,
      unitCount: 0,
      cap: 30,
      pending: [],
      offered: null,
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
  expectScore(view, "Your victory score", 345, matchRules.victory_score);
  expectScore(view, "Enemy victory score", 120, matchRules.victory_score);
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
  expectScore(view, "Your victory score", 120, matchRules.victory_score);
  expectScore(view, "Enemy victory score", 345, matchRules.victory_score);
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
    expectScore(view, label, 0, frontline.victory_score);
});

test("each side's meter fills toward the win, lighting cells for what it has scored", () => {
  const target = matchRules.victory_score;
  const view = render(
    <SkirmishStatus
      target={target}
      match={{ ...match, phase: "active", scores: [target / 2, target] }}
      viewer="blue"
    />,
  );
  const lit = (label: string) => {
    const cells = [...view.getByLabelText(label).children];
    return cells.filter((c) => c.getAttribute("data-lit") === "full").length / cells.length;
  };
  expect(lit("Your victory score")).toBe(0.5);
  expect(lit("Enemy victory score")).toBe(1);
});

test("the cell a side is filling blinks only while that side holds a flag the enemy isn't on", () => {
  const target = matchRules.victory_score;
  const flag = (owner: "blue" | "red", contested: boolean) => ({
    id: `${owner}-${contested}`,
    center: [0, 0] as [number, number],
    radiusM: 50,
    owner,
    contested,
    capturing: null,
    captureProgress: 0,
  });
  const view = render(
    <SkirmishStatus
      target={target}
      match={{
        ...match,
        phase: "active",
        scores: [target * 0.31, target * 0.42],
        objectives: [flag("blue", false), flag("red", true)],
      }}
      viewer="blue"
    />,
  );
  const filling = (label: string) =>
    [...view.getByLabelText(label).children].filter((c) => c.getAttribute("data-filling")).length;
  // Blue holds a quiet flag and is scoring; Red's only flag has Blue on it.
  expect(filling("Your victory score")).toBe(1);
  expect(filling("Enemy victory score")).toBe(0);
});
