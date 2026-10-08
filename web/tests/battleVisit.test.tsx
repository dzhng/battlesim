import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router";
import { preparedBattleHref } from "@apps/battle-lab/src/battleLinks";
import { afterEach, expect, test, vi } from "vitest";
import { LabRouter } from "@apps/battle-lab/src/router";
import type { PreparedSession } from "../src/battle/prepare/client";
import type { AdmissionInputs } from "../src/battle/prepare/admission";
import { renderSettled } from "./support/router";

const pending = vi.hoisted(() => ({
  finish: null as null | ((battle: PreparedSession) => void),
  input: null as null | AdmissionInputs,
  cancelled: false,
}));
// Control admission completion at its async boundary; routing, parsing,
// the scenario builder and the loading/cancel surface stay real.
vi.mock("@web/battle/prepare/admission", () => ({
  admitBattle: (input: AdmissionInputs) => {
    pending.input = input;
    return {
      battle: new Promise<PreparedSession>((resolve) => {
        pending.finish = resolve;
      }),
      cancel: () => {
        pending.cancelled = true;
      },
      attempts: [],
    };
  },
}));
// This suite stops at admission rather than booting a GPU and combat authority.
vi.mock("@apps/battle-lab/src/BattleView", () => ({ BattleView: () => <p>Battle admitted</p> }));

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  pending.finish = null;
  pending.input = null;
  pending.cancelled = false;
});

test("Cancel discards admission even when its resolved continuation is already queued", async () => {
  window.history.replaceState(null, "", "/battle?play=1&type=mixed&size=small&faction=us");
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  const cancel = await screen.findByRole("link", { name: "Cancel" });
  const request = pending.input!.candidate("42");
  await act(async () => {
    pending.finish!({
      scenario: "{}",
      report: { request } as PreparedSession["report"],
      connect: () => ({ send() {}, close() {} }),
    });
    fireEvent.click(cancel);
  });
  expect(pending.cancelled).toBe(true);
  // Back on the menu, on the page that asks for the cancelled battle.
  expect(screen.getByRole("heading", { name: "Skirmish" })).toBeDefined();
  expect(window.location.pathname).toBe("/");
  expect(window.location.search).toBe("?type=mixed&size=small");
});

test("successful ordinary admission publishes the exact address without returning to preparation", async () => {
  window.history.replaceState(
    { usr: { note: "keep" } },
    "",
    "/battle?play=1&type=mixed&size=small&faction=us",
  );
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  await screen.findByRole("link", { name: "Cancel" });
  const request = pending.input!.candidate("9007199254740993");
  const bounds = [0, 0, 100, 100] as const;
  await act(async () => {
    pending.finish!({
      scenario: "{}",
      report: {
        request,
        start: { at: [0, 0], yaw: 0 },
        size: [100, 100],
        extents: { playable: bounds, physical: bounds, rendered: bounds },
      } as PreparedSession["report"],
      connect: () => ({ send() {}, close() {} }),
    });
  });
  expect(screen.getByText("Battle admitted")).toBeDefined();
  expect(screen.queryByTestId("loading")).toBeNull();
  expect(window.location.pathname + window.location.search).toBe(preparedBattleHref(request));
  expect(window.history.state.usr.note).toBe("keep");
  expect(pending.cancelled).toBe(false);
});
