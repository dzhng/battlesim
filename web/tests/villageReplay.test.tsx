// @vitest-environment jsdom
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { VillageReplay } from "@apps/battle-lab/src/routes/village";
import { restartBattle } from "@web/mechanicsLifecycle";
import { readSavedReplay } from "@apps/battle-lab/src/replayFile";

// Exercise replay import while the scenario is still preparing; drawing is irrelevant.
vi.mock("@apps/battle-lab/src/useBuiltScenario", () => ({ useBuiltScenario: () => null }));
vi.mock("@apps/battle-lab/src/BattleView", () => ({ BattleView: () => null }));
vi.mock("@web/mechanicsLifecycle", () => ({
  restartBattle: vi.fn((restart: () => void) => restart()),
}));
afterEach(() => {
  cleanup();
  localStorage.clear();
  vi.restoreAllMocks();
  vi.clearAllMocks();
});

test("importing a village replay remembers it before restarting its viewer", async () => {
  localStorage.clear();
  const replay = { variant: "ordinary", replay: "{}" };
  const view = render(<VillageReplay />);
  fireEvent.change(view.container.querySelector("input[type=file]")!, {
    target: { files: [{ text: async () => JSON.stringify(replay) }] },
  });
  await waitFor(() => expect(readSavedReplay()).toEqual(replay));
});

test("a replay import reports blocked storage without reloading into an older replay", async () => {
  localStorage.clear();
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("Storage unavailable");
  });
  const view = render(<VillageReplay />);
  fireEvent.change(view.container.querySelector("input[type=file]")!, {
    target: {
      files: [{ text: async () => JSON.stringify({ variant: "ordinary", replay: "{}" }) }],
    },
  });
  await waitFor(() => expect(view.getByText("Storage unavailable")).toBeTruthy());
  expect(restartBattle).not.toHaveBeenCalled();
});
