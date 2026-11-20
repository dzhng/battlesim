// The coach's voice across its steps: each change of step queues its lines,
// and one speaker plays them in order. A reaction is heard to its end; a
// step's own line gives way to the next step's. The radio
// is a fake that records what it was asked to say; every take lasts 3 s.
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { CoachPanel } from "@web/battle/coach/CoachPanel";
import type { CoachDocument, CoachStep, CoachView } from "@web/battle/coach/coach";

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const killed: CoachStep = {
  id: "killed",
  card: "Take out the tank.",
  line: "killed",
  reacts: true,
  until: { beat: "tank_destroyed" },
};
const spend: CoachStep = { id: "spend", card: "Spend.", line: "credits", until: "continue" };
const look: CoachStep = { id: "look", card: "Look.", line: "look", until: { ui: ["camera_pan"] } };
const turn: CoachStep = {
  id: "turn",
  card: "Turn.",
  line: "turn",
  until: { ui: ["camera_rotate"] },
};
const DOC: CoachDocument = {
  speaker: { id: "lantern", name: "Lantern", post: "HQ", portrait: "" },
  chapters: [
    { id: "fight", title: "Fight", steps: [look, turn, killed] },
    { id: "buy", title: "Buy", steps: [spend] },
  ],
};
const at = (chapter: number, step: CoachStep): CoachView => ({
  chapter: DOC.chapters[chapter],
  step,
  index: DOC.chapters[chapter].steps.indexOf(step),
});

/** The panel over a run of the battle, and what its radio was asked to say. */
function coach() {
  const said: string[] = [];
  const speak = (_speaker: string, line: string) => {
    said.push(line);
    return Promise.resolve(3);
  };
  const props = (view: CoachView | null, told: string[] = [], run: unknown = 1) => (
    <CoachPanel
      doc={DOC}
      view={view}
      told={new Set(told)}
      run={run}
      onContinue={() => {}}
      speak={speak}
    />
  );
  const panel = render(props(at(0, killed)));
  const show = (view: CoachView | null, told: string[] = [], run: unknown = 1) =>
    act(async () => panel.rerender(props(view, told, run)));
  const wait = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));
  return { said, show, wait };
}

test("a reaction that closes its chapter is heard to its end before the next chapter's line", async () => {
  const { said, show, wait } = coach();
  // The tank dies: the chapter's steps are all done, and the next chapter
  // opens a tick later, each in a render of its own.
  await show(null, ["tank_destroyed"]);
  await show(at(1, spend), ["tank_destroyed"]);
  expect(said).toEqual(["killed"]);
  await wait(3000);
  expect(said).toEqual(["killed", "credits"]);
});

test("a step's own line gives way at once to the next step's", async () => {
  const { said, show } = coach();
  await show(at(0, look));
  await show(at(0, turn));
  expect(said).toEqual(["look", "turn"]);
});

test("a new run of the battle silences what the last one queued", async () => {
  const { said, show, wait } = coach();
  await show(at(0, look));
  await show(null, ["tank_destroyed"]);
  // Restarted: the reaction still waiting is dropped, and the new run's
  // first step speaks at once.
  await show(at(0, look), [], 2);
  expect(said).toEqual(["look", "look"]);
  await wait(10_000);
  expect(said).toEqual(["look", "look"]);
});
