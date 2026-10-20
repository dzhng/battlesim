// @vitest-environment jsdom
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { ComponentProps } from "react";
import type { BattleView } from "@apps/battle-lab/src/BattleView";
import { useSimSession } from "@apps/battle-lab/src/useSimSession";
import Endurance from "@apps/battle-lab/src/routes/endurance";
import type { PrepareReply, PrepareWorkerRequest } from "@web/battle/prepare/protocol";
import { WithTestCatalog } from "./catalog";

// Replace the drawing edge, retaining the real session/adoption consumer.
vi.mock("@apps/battle-lab/src/BattleView", () => ({
  BattleView: (props: ComponentProps<typeof BattleView>) => {
    useSimSession({ ...props, player: props.player ?? undefined });
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
    requests: PrepareWorkerRequest[] = [];
    terminated = false;
    constructor() {
      workers.push(this);
    }
    postMessage(request: PrepareWorkerRequest) {
      this.requests.push(request);
    }
    terminate() {
      this.terminated = true;
    }
  }
  vi.stubGlobal("Worker", Worker);
  window.history.replaceState(null, "", "/labs/endurance?generated=1&seed=7");
  // The router scopes every lab to the test set.
  const page = render(
    <WithTestCatalog>
      <Endurance />
    </WithTestCatalog>,
  );
  await waitFor(() => expect(workers[0]?.requests[0].type).toBe("prepare"));
  const authority = workers[0];
  const scenario = "prepared endurance scenario";
  const request = authority.requests[0];
  if (request.type !== "prepare") throw new Error("expected preparation request");
  authority.onmessage?.({
    data: {
      type: "prepared",
      battle: {
        scenario,
        report: {
          request: request.request,
          identity: {
            kind: "generated",
            generation: {
              generator_version: request.request.map_source.request.generator_version,
              preset_revision: request.request.map_source.request.preset_revision,
              seed: request.request.map_source.request.seed,
              template_catalog_hash: request.request.map_source.request.template_catalog_hash,
              config_hash: "prepared-config",
              map_hash: "prepared-map",
            },
          },
          counts: { buildings: 0, parts: 0, props: 0, surfaces: 0, forests: 0 },
          start: { at: [5000, 5000], yaw: 0 },
          town: null,
          timings: { map: 0, encounter: 0 },
          wasmBytes: 0,
          worldBuildMs: 0,
          size: [10_000, 10_000],
          extents: {
            playable: [0, 0, 10_000, 10_000],
            physical: [0, 0, 10_000, 10_000],
            rendered: [0, 0, 10_000, 10_000],
          },
        },
      },
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
