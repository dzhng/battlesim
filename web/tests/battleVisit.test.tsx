import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import { BrowserRouter } from "react-router";
import { afterEach, expect, test, vi } from "vitest";
import { LabRouter } from "@apps/battle-lab/src/router";
import type { PreparedSession } from "../src/battle/prepare/client";
import type { PreparationMessage } from "../src/battle/prepare/protocol";
import { renderSettled } from "./support/router";

const pending = vi.hoisted(() => ({
  finish: null as null | ((battle: PreparedSession) => void),
  message: null as null | PreparationMessage,
  cancelled: false,
}));
// Control preparation's completion at its async boundary; routing, parsing,
// the request builder and the loading/cancel surface stay real.
vi.mock("@web/battle/prepare/client", async (actual) => ({
  ...(await actual<object>()),
  prepareBattle: (message: PreparationMessage) => {
    pending.message = message;
    return {
      battle: new Promise<PreparedSession>((resolve) => {
        pending.finish = resolve;
      }),
      cancel: () => {
        pending.cancelled = true;
      },
    };
  },
}));
// This suite stops at preparation rather than booting a GPU and combat authority.
vi.mock("@apps/battle-lab/src/BattleView", () => ({ BattleView: () => <p>Battle admitted</p> }));

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  pending.finish = null;
  pending.message = null;
  pending.cancelled = false;
});

test("Cancel discards preparation even when its resolved continuation is already queued", async () => {
  window.history.replaceState(
    null,
    "",
    "/battle?type=mixed&size=small&seed=42&blue=h&blue_faction=us&red=a&red_faction=eastern",
  );
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  const cancel = await screen.findByRole("link", { name: "Cancel" });
  // The loading cover shows before preparation starts (its documents load first).
  await vi.waitFor(() => expect(pending.message).not.toBeNull());
  const message = pending.message!;
  if (message.type !== "prepare") throw new Error("an exact battle prepares its request");
  const request = message.request;
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
  expect(window.location.search).toBe("?type=mixed&size=small&seed=42");
});
