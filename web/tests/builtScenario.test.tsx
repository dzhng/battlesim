import { expect, test, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useBuiltScenario } from "@apps/battle-lab/src/useBuiltScenario";
vi.mock("@web/battle/sim/module", () => ({ loadWasm: async () => ({}) }));

test("leaving a loading scenario cancels its preparation work", async () => {
  let started = false;
  let cancelled = false;
  const { unmount } = renderHook(() =>
    useBuiltScenario(
      { seed: 1 },
      async (_wasm, _options, signal) =>
        new Promise<string>(() => {
          started = true;
          signal?.addEventListener("abort", () => {
            cancelled = true;
          });
        }),
    ),
  );
  await waitFor(() => expect(started).toBe(true));
  unmount();
  expect(cancelled).toBe(true);
});
