// @vitest-environment node
import { expect, test, vi } from "vitest";
import { prepareBattle } from "../src/battle/prepare/client";
import type { PrepareMessage, PreparedBattle } from "../src/battle/prepare/protocol";

test("the prepared worker remains available to play, and cancellation silences late answers", async () => {
  const workers: FakeWorker[] = [];
  class FakeWorker {
    onmessage: ((event: MessageEvent) => void) | null = null;
    onerror = null;
    terminate = vi.fn();
    postMessage = vi.fn();
    constructor() {
      workers.push(this);
    }
    deliver(data: unknown) {
      this.onmessage?.({ data } as MessageEvent);
    }
  }
  vi.stubGlobal("Worker", FakeWorker);
  try {
    const preparation = prepareBattle({} as PrepareMessage, () => {});
    const wire = { scenario: "{}", report: {} } as PreparedBattle;
    workers[0].deliver({ type: "prepared", battle: wire });
    const result = await preparation.battle;
    const receive = vi.fn();
    const channel = result.connect(receive, () => {});
    channel.send({ type: "pause" });
    workers[0].deliver({ type: "status", status: "paused", slow: false });
    expect(receive).toHaveBeenCalledWith({ type: "status", status: "paused", slow: false });
    expect(workers[0].postMessage).toHaveBeenLastCalledWith({ type: "pause" }, { transfer: [] });
    expect(workers).toHaveLength(1);
    expect(workers[0].terminate).not.toHaveBeenCalled();
    preparation.cancel();
    expect(workers[0].terminate).toHaveBeenCalledOnce();
    const abandoned = prepareBattle({} as PrepareMessage, () => {});
    const settled = vi.fn();
    abandoned.battle.then(settled, settled);
    abandoned.cancel();
    workers[1].deliver({ type: "prepared", battle: wire });
    await Promise.resolve();
    expect(settled).not.toHaveBeenCalled();
  } finally {
    vi.unstubAllGlobals();
  }
});

test("a worker refusal is visible in the browser console with its diagnostics", async () => {
  const workers: FakeWorker[] = [];
  class FakeWorker {
    onmessage: ((event: MessageEvent) => void) | null = null;
    onerror = null;
    terminate = vi.fn();
    postMessage = vi.fn();
    constructor() {
      workers.push(this);
    }
    deliver(data: unknown) {
      this.onmessage?.({ data } as MessageEvent);
    }
  }
  const warning = vi.spyOn(console, "warn").mockImplementation(() => {});
  vi.stubGlobal("Worker", FakeWorker);
  try {
    const preparation = prepareBattle({} as PrepareMessage, () => {});
    workers[0].deliver({
      type: "refused",
      stage: "encounter",
      diagnostics: [
        {
          code: "invalid_skirmish_sites",
          feature: "skirmish_sites",
          location: "$.sites.skirmish",
          message: "central objective candidates exhausted",
        },
      ],
    });
    await expect(preparation.battle).rejects.toThrow("central objective candidates exhausted");
    expect(warning).toHaveBeenCalledWith(
      "battle preparation refused",
      expect.objectContaining({
        stage: "encounter",
        diagnostics: expect.arrayContaining([
          expect.objectContaining({ code: "invalid_skirmish_sites" }),
        ]),
      }),
    );
  } finally {
    warning.mockRestore();
    vi.unstubAllGlobals();
  }
});
