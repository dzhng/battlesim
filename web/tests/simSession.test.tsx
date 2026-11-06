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

test("ticks published within one frame render once, the latest, while each is still decoded", async () => {
  const { readFileSync } = await import("node:fs");
  const { initSync, Battle } = await import("@wasm/game_wasm.js");
  const { labScenario } = await import("./catalog");
  const { loadMap } = await import("@web/maps/node");
  type Reply = import("@web/battle/sim/protocol").SimReply;
  const memory = initSync({
    module: readFileSync(`${process.cwd()}/src/wasm/game_wasm_bg.wasm`),
  }).memory;
  const battle = new Battle(
    labScenario(loadMap("geometry").definition, [
      { side: "blue", kind: "test_tank", position: [40, 150] },
    ]),
    9,
  );
  let worker: {
    onmessage: ((event: MessageEvent<Reply>) => void) | null;
    postMessage(): void;
    terminate(): void;
  } | null = null;
  vi.stubGlobal(
    "Worker",
    class {
      constructor() {
        return (worker = {
          onmessage: null,
          postMessage() {},
          terminate() {},
        });
      }
    },
  );
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => frames.push(callback));
  vi.stubGlobal("cancelAnimationFrame", () => {});
  const deliver = (reply: Reply) => worker!.onmessage?.({ data: reply } as MessageEvent<Reply>);
  const decoded: number[] = [];
  let renders = 0;
  const { result } = renderHook(() => {
    renders++;
    return useSimSession({
      scenario: "{}",
      seed: 1,
      onDecoded: (observation) => decoded.push(observation.tick),
    });
  });
  try {
    await act(async () => {
      deliver({ type: "ready", layout: battle.observation_layout(), tickHz: 30, tick: 0 });
    });
    const before = renders;
    act(() => {
      for (let tick = 1; tick <= 3; tick++) {
        battle.step();
        const length = battle.publish("blue");
        const buffer = new Float32Array(memory.buffer, battle.publication_ptr(), length).slice()
          .buffer;
        deliver({ type: "publication", tick, digest: battle.digest(), length, buffer, stepMs: 0 });
      }
    });
    expect(decoded).toEqual([1, 2, 3]);
    expect(result.current.observation).toBeNull();
    act(() => frames.splice(0).forEach((frame) => frame(0)));
    expect(result.current.observation?.tick).toBe(3);
    expect(renders - before).toBe(1);
  } finally {
    battle.free();
  }
});

test("a held warm-up continuation cannot act after Restart replaces its session", async () => {
  const { readFileSync } = await import("node:fs");
  const { initSync, Battle } = await import("@wasm/game_wasm.js");
  const { labScenario } = await import("./catalog");
  const { loadMap } = await import("@web/maps/node");
  type Reply = import("@web/battle/sim/protocol").SimReply;
  const memory = initSync({
    module: readFileSync(`${process.cwd()}/src/wasm/game_wasm_bg.wasm`),
  }).memory;
  const battle = new Battle(
    labScenario(loadMap("geometry").definition, [
      { side: "blue", kind: "test_tank", position: [40, 150] },
    ]),
    9,
  );
  const workers: Array<{
    onmessage: ((event: MessageEvent<Reply>) => void) | null;
    requests: SimRequest[];
    terminate(): void;
  }> = [];
  vi.stubGlobal(
    "Worker",
    class {
      onmessage: ((event: MessageEvent<Reply>) => void) | null = null;
      requests: SimRequest[] = [];
      constructor() {
        workers.push(this);
      }
      postMessage(request: SimRequest) {
        this.requests.push(request);
      }
      terminate() {}
    },
  );
  let releaseHold!: () => void;
  const onWarm = vi.fn();
  const scripted = {
    warmTo: 1,
    hold: () => new Promise<void>((resolve) => (releaseHold = resolve)),
    onWarm,
    onTick: () => {},
  };
  const { result } = renderHook(() => useSimSession({ scenario: "{}", seed: 1, scripted }));
  try {
    const deliver = (worker: (typeof workers)[number], reply: Reply) =>
      worker.onmessage?.({ data: reply } as MessageEvent<Reply>);
    await act(async () => {
      deliver(workers[0], {
        type: "ready",
        layout: battle.observation_layout(),
        tickHz: 30,
        tick: 0,
      });
    });
    act(() => result.current.onViewportReady());
    expect(workers[0].requests).toContainEqual({ type: "start" });
    deliver(workers[0], { type: "advanced", id: 1, tick: 1 });
    battle.step();
    const length = battle.publish("blue");
    const buffer = new Float32Array(memory.buffer, battle.publication_ptr(), length).slice().buffer;
    deliver(workers[0], {
      type: "publication",
      tick: 1,
      digest: battle.digest(),
      length,
      buffer,
      stepMs: 0,
    });
    await Promise.resolve();
    expect(releaseHold).toBeDefined();
    act(() => result.current.restart());
    await act(async () => {
      deliver(workers[1], {
        type: "ready",
        layout: battle.observation_layout(),
        tickHz: 30,
        tick: 0,
      });
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(workers[1].requests.some((request) => request.type === "start")).toBe(true);
    await act(async () => {
      releaseHold();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(onWarm).not.toHaveBeenCalled();
  } finally {
    battle.free();
  }
});
