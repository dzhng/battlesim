import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { useSimSession } from "@apps/battle-lab/src/useSimSession";
import type { SimRequest } from "@web/battle/sim/protocol";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

test("development Restart recreates the authority with the page's captured scenario", () => {
  const workers: Worker[] = [];
  class Worker {
    onmessage = null;
    onerror = null;
    requests: SimRequest[] = [];
    terminated = false;
    constructor() {
      workers.push(this);
    }
    postMessage(request: SimRequest) {
      this.requests.push(request);
    }
    terminate() {
      this.terminated = true;
    }
  }
  vi.stubGlobal("Worker", Worker);
  vi.stubEnv("DEV", true);
  const init = {
    type: "init",
    scenario: "captured scenario",
    seed: 7,
    side: "blue",
    replay: undefined,
    script: undefined,
  };
  const { result } = renderHook(() => useSimSession({ scenario: init.scenario, seed: init.seed }));
  expect(workers[0].requests[0]).toEqual(init);

  act(() => result.current.restart());

  expect(workers.map((worker) => worker.requests[0])).toEqual([init, init]);
  expect(workers[0].terminated).toBe(true);
});
