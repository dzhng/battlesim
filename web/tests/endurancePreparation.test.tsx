// @vitest-environment jsdom
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { ComponentProps } from "react";
import type { BattleView } from "@apps/battle-lab/src/BattleView";
import { useSimSession } from "@apps/battle-lab/src/useSimSession";
import Endurance from "@apps/battle-lab/src/routes/endurance";
import type { PrepareReply } from "@web/battle/prepare/protocol";
import { WithTestCatalog } from "./catalog";

// Replace the drawing edge, retaining the real session/adoption consumer.
vi.mock("@apps/battle-lab/src/BattleView", () => ({
  BattleView: (props: ComponentProps<typeof BattleView>) => {
    useSimSession(props);
    return null;
  },
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

test("generated endurance adopts its prepared authority instead of starting a second worker", async () => {
  const workers: Worker[] = [];
  class Worker {
    onmessage: ((event: { data: PrepareReply }) => void) | null = null;
    onerror = null;
    requests: { type: string; scenario?: string; seed?: number }[] = [];
    terminated = false;
    constructor() {
      workers.push(this);
    }
    postMessage(request: { type: string }) {
      this.requests.push(request);
    }
    terminate() {
      this.terminated = true;
    }
  }
  vi.stubGlobal("Worker", Worker);
  window.history.replaceState(null, "", "/lab/endurance?generated=1&seed=7");
  // The router scopes every lab to the test set.
  const page = render(
    <WithTestCatalog>
      <Endurance />
    </WithTestCatalog>,
  );
  await waitFor(() => expect(workers[0]?.requests[0].type).toBe("prepare"));
  const authority = workers[0];
  const scenario = "prepared endurance scenario";
  authority.onmessage?.({
    data: {
      type: "prepared",
      battle: { scenario, report: { size: [10_000, 10_000] } } as Extract<
        PrepareReply,
        { type: "prepared" }
      >["battle"],
    },
  });
  await waitFor(() => {
    expect(workers).toHaveLength(1);
    expect(authority.requests).toContainEqual({
      type: "init",
      scenario,
      seed: 7,
      side: "blue",
      replay: undefined,
      script: undefined,
    });
  });
  page.unmount();
  expect(authority.terminated).toBe(true);
});
