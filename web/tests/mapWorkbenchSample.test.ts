// @vitest-environment node
import { expect, test, vi } from "vitest";
import { collectSample } from "../../apps/map-workbench/src/sample";
import type { Draft, Report, WorkbenchAPI } from "../../apps/map-workbench/src/protocol";

test("a sample retains refusals and exact sources once, and names cancelled requests", async () => {
  const input: Draft = { revision: "source", documents: { presets: {}, defaults: {} } };
  const choices = ["1", "2", "3"].map((seed) => ({
    type: "mixed" as const,
    size: "small" as const,
    seed,
  }));
  const controller = new AbortController();
  const report = (seed: string): Report => ({
    fingerprint: `tested-${seed}`,
    choice: choices.find((choice) => choice.seed === seed)!,
    status: seed === "1" ? "refused" : "ok",
    diagnostics: [],
    artifactId: seed,
    svg: "large geometry which a sample does not retain",
  });
  const api = {
    generate: vi.fn(async ({ choice }) => report(choice.seed)),
    export: vi.fn(async () => ({
      inputs: { rules: "exact source bytes" },
      receipts: { rules: "receipt" },
    })),
  } as unknown as WorkbenchAPI;
  const result = await collectSample(api, input, choices, controller.signal, (progress) => {
    if (progress.rows.length === 2) controller.abort();
  });
  expect(result.rows.map((row) => [row.choice.seed, row.status])).toEqual([
    ["1", "refused"],
    ["2", "ok"],
  ]);
  expect(result.sources).toEqual([
    { inputs: { rules: "exact source bytes" }, receipts: { rules: "receipt" } },
  ]);
  expect(result.rows.map((row) => row.source)).toEqual([0, 0]);
  expect(result.rows[1].svg).toBeUndefined();
  expect(result.choices.slice(result.rows.length)).toEqual([choices[2]]);
  expect(result.cancelled).toBe(true);
  expect(result.complete).toBe(false);
});

test("cancellation during receipt capture explicitly marks a completed outcome as missing its sources", async () => {
  const input: Draft = { revision: "source", documents: { presets: {}, defaults: {} } };
  const choice = { type: "mixed" as const, size: "small" as const, seed: "1" };
  const controller = new AbortController();
  const api = {
    generate: async () => ({
      fingerprint: "tested",
      choice,
      status: "ok",
      diagnostics: [],
      artifactId: "map",
    }),
    export: async () => {
      controller.abort();
      throw new Error("Receipt capture cancelled");
    },
  } as unknown as WorkbenchAPI;
  const result = await collectSample(api, input, [choice], controller.signal, () => {});
  expect(result.rows[0].status).toBe("ok");
  expect(result.sources).toEqual([]);
  expect(result.receiptErrors).toEqual(["mixed small 1: Error: Receipt capture cancelled"]);
});

test("an execution failure keeps the requested outcome and explicitly lacks an input receipt", async () => {
  const input: Draft = { revision: "saved-source", documents: { presets: {}, defaults: {} } };
  const choice = { type: "mixed" as const, size: "small" as const, seed: "1" };
  const api = {
    generate: async () => {
      throw new Error("Native job exceeded its deadline");
    },
  } as unknown as WorkbenchAPI;
  const result = await collectSample(api, input, [choice], new AbortController().signal, () => {});
  expect(result.rows[0]).toMatchObject({
    choice,
    status: "refused",
    stage: "execution",
    fingerprint: "unavailable",
  });
  expect(result.rows[0].source).toBeUndefined();
  expect(result.sources).toEqual([]);
  expect(result.receiptErrors).toEqual([
    "mixed small 1: Execution failed before an exact-input receipt was available",
  ]);
  expect(result.complete).toBe(true);
});
