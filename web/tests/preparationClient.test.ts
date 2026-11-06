// @vitest-environment node
import { expect, test, vi } from "vitest";
import { prepareBattle, prepareUploadedScenarioBundle } from "../src/battle/prepare/client";
import type {
  PrepareMessage,
  PreparedBattle,
  PreparedScenarioBundle,
} from "../src/battle/prepare/protocol";

test.each(["ordinary", "bundle"] as const)(
  "the %s worker remains available to play, and cancellation silences late answers",
  async (path) => {
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
      const prepare = () =>
        path === "ordinary"
          ? prepareBattle({} as PrepareMessage, () => {})
          : prepareUploadedScenarioBundle("{}", () => {}, "player");
      const reply =
        path === "ordinary"
          ? { type: "prepared", battle: { scenario: "{}", report: {} } as PreparedBattle }
          : {
              type: "bundle-prepared",
              battle: { canonical: "{}", scenario: "{}", report: {} } as PreparedScenarioBundle,
            };
      const preparation = prepare();
      workers[0].deliver(reply);
      const result = await preparation.battle;
      const receive = vi.fn();
      const session = "session" in result ? result.session : result;
      const channel = session.connect(receive, () => {});
      channel.send({ type: "pause" });
      workers[0].deliver({ type: "status", status: "paused", slow: false });
      expect(receive).toHaveBeenCalledWith({ type: "status", status: "paused", slow: false });
      expect(workers[0].postMessage).toHaveBeenLastCalledWith({ type: "pause" }, { transfer: [] });
      expect(workers).toHaveLength(1);
      expect(workers[0].terminate).not.toHaveBeenCalled();
      preparation.cancel();
      expect(workers[0].terminate).toHaveBeenCalledOnce();
      receive.mockClear();
      workers[0].deliver({ type: "status", status: "running", slow: false });
      expect(receive).not.toHaveBeenCalled();
      const abandoned = prepare();
      const settled = vi.fn();
      abandoned.battle.then(settled, settled);
      queueMicrotask(() => workers[1].deliver(reply));
      abandoned.cancel();
      await Promise.resolve();
      await Promise.resolve();
      expect(settled).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllGlobals();
    }
  },
);

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
