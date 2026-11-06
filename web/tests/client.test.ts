// @vitest-environment node
import { expect, test, vi } from "vitest";
import { readFileSync } from "node:fs";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { createSimClient, type Publication } from "../src/battle/sim/client";
import { ScenePresentation, loadPresentation } from "../src/battle/sim/presentation";
import type { SimReply, SimRequest } from "../src/battle/sim/protocol";
import { labScenario } from "./catalog";
import { loadMap } from "@web/maps/node";

await loadPresentation();

test("a direct authority's terminal error rejects ready before its transport closes", async () => {
  const client = createSimClient({ scenario: "{", seed: 1, side: "blue", transport: "direct" });
  const onStatus = vi.fn();
  client.onStatus(onStatus);
  try {
    client.pause();
    expect(client.paused).toBe(true);
    client.resume();
    expect(client.paused).toBe(false);
    await expect(client.ready).rejects.toThrow();
    expect(onStatus).toHaveBeenLastCalledWith("failed", false);
  } finally {
    client.dispose();
  }
}, 1000);

test("a rejected worker record returns credits and fails every pending request", async () => {
  const { requests, deliver } = workerSeam();
  const client = createSimClient({ scenario: "{}", seed: 1, side: "blue", transport: "worker" });
  const statuses: string[] = [];
  client.onStatus((status) => statuses.push(status));
  const command = client.command({ kind: "stop", units: [1] });
  const advance = client.advance(1);
  const replay = client.replay();
  const preview = client.previewMove({ units: [1], goal: [100, 100], facing: 0 });
  const outcomes = Promise.allSettled([client.ready, command, advance, replay, preview]);
  try {
    const buffer = new ArrayBuffer(4);
    expect(() =>
      deliver({ type: "publication", tick: 0, digest: "", length: 1, buffer, stepMs: 0 }),
    ).not.toThrow();
    expect(requests.filter((request) => request.type === "credit")).toEqual([
      { type: "credit", buffer },
    ]);
    expect(statuses).toEqual(["failed"]);
    const results = await outcomes;
    expect(results.every((result) => result.status === "rejected")).toBe(true);
    deliver({ type: "status", status: "running", slow: false });
    expect(statuses).toEqual(["failed"]);
    const late = new ArrayBuffer(4);
    deliver({ type: "publication", tick: 1, digest: "", length: 1, buffer: late, stepMs: 0 });
    expect(requests.filter((request) => request.type === "credit")).toEqual([
      { type: "credit", buffer },
      { type: "credit", buffer: late },
    ]);
    await expect(client.advance(1)).rejects.toThrow();
    await expect(client.command({ kind: "stop", units: [1] })).rejects.toThrow();
    await expect(client.replay()).rejects.toThrow();
  } finally {
    client.dispose();
    vi.unstubAllGlobals();
  }
}, 1000);

function workerSeam() {
  const requests: SimRequest[] = [];
  class FakeWorker {
    onmessage: ((event: MessageEvent<SimReply>) => void) | null = null;
    onerror = null;
    postMessage(request: SimRequest) {
      requests.push(request);
    }
    terminate = vi.fn();
    deliver(reply: SimReply) {
      this.onmessage?.({ data: reply } as MessageEvent<SimReply>);
    }
  }
  const worker = new FakeWorker();
  vi.stubGlobal("Worker", function () {
    return worker;
  });
  return {
    requests,
    close: worker.terminate,
    deliver: (reply: SimReply) => worker.deliver(reply),
  };
}

test("disposal rejects previews while leaving already pending operations silent", async () => {
  const { deliver, close } = workerSeam();
  const client = createSimClient({ scenario: "{}", seed: 1, side: "blue", transport: "worker" });
  const statuses: string[] = [];
  client.onStatus((status) => statuses.push(status));
  const settled = vi.fn();
  const pending = [
    client.ready,
    client.command({ kind: "stop", units: [1] }),
    client.advance(1),
    client.replay(),
  ];
  for (const promise of pending) void promise.then(settled, settled);
  const previews = Promise.allSettled([
    client.previewMove({ units: [1], goal: [100, 100], facing: 0 }),
    client.previewPurchase("test_tank", [100, 100]),
    client.previewBuilding({ units: [1], building: 9 }),
  ]);
  try {
    client.dispose();
    client.dispose();
    expect(statuses).toEqual(["disposed"]);
    expect(close).toHaveBeenCalledOnce();
    expect(await previews).toEqual(
      Array.from({ length: 3 }, () => ({
        status: "rejected",
        reason: new Error("simulation client disposed"),
      })),
    );
    deliver({ type: "ack", ack: { seq: 1, applied_tick: 1, error: null, refused: [] } });
    deliver({ type: "advanced", id: 1, tick: 0 });
    deliver({ type: "replay", json: "{}" });
    deliver({ type: "error", message: "late worker failure" });
    await Promise.resolve();
    expect(settled).not.toHaveBeenCalled();
    await expect(client.advance(1)).rejects.toThrow("simulation client disposed");
  } finally {
    client.dispose();
    vi.unstubAllGlobals();
  }
});

test.each([true, false])(
  "advance waits for both acknowledgement and publication release (ack first: %s)",
  async (ackFirst) => {
    const memory = initSync({
      module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
    }).memory;
    const battle = new Battle(
      labScenario(loadMap("geometry").definition, [
        { side: "blue", kind: "test_tank", position: [40, 150] },
      ]),
      9,
    );
    const { requests, deliver } = workerSeam();
    const client = createSimClient({ scenario: "{}", seed: 1, side: "blue", transport: "worker" });
    const held: Publication[] = [];
    client.onPublication((publication) => held.push(publication));
    try {
      deliver({ type: "ready", layout: battle.observation_layout(), tickHz: 10, tick: 0 });
      await client.ready;
      const advance = client.advance(1);
      const completed = vi.fn();
      void advance.then(completed);
      battle.step();
      const length = battle.publish("blue");
      const buffer = new Float32Array(memory.buffer, battle.publication_ptr(), length).slice()
        .buffer;
      deliver({ type: "publication", tick: 1, digest: battle.digest(), length, buffer, stepMs: 0 });
      const acknowledge = () => deliver({ type: "advanced", id: 1, tick: 1 });
      if (ackFirst) acknowledge();
      else held[0].release();
      await Promise.resolve();
      expect(completed).not.toHaveBeenCalled();
      if (ackFirst) held[0].release();
      else acknowledge();
      expect(await advance).toBe(1);
      held[0].release();
      expect(requests.filter((request) => request.type === "credit")).toEqual([
        { type: "credit", buffer },
      ]);
    } finally {
      client.dispose();
      battle.free();
      vi.unstubAllGlobals();
    }
  },
);

test("building preview returns the entrant and companion placements without issuing a command", async () => {
  const { requests, deliver } = workerSeam();
  const client = createSimClient({ scenario: "{}", seed: 1, side: "blue", transport: "worker" });
  try {
    const preview = client.previewBuilding({ units: [1, 2], building: 9, queued: true });
    const request = requests.find((request) => request.type === "building_preview");
    expect(request).toEqual({
      type: "building_preview",
      id: 1,
      side: "blue",
      building: { units: [1, 2], building: 9, queued: true },
    });
    const placement = {
      building: 9,
      entrant: { unit: 1, approach: [40, 50] as [number, number] },
      destinations: [
        { unit: 2, goal: [30, 50] as [number, number], placed: true, facing: 0, spots: [] },
      ],
    };
    deliver({ type: "building_preview", id: 1, placement });
    expect(await preview).toEqual(placement);
    expect(requests.some((request) => request.type === "command")).toBe(false);
  } finally {
    client.dispose();
    vi.unstubAllGlobals();
  }
});

test("an authority failure returns held records once and rejects requests after ready", async () => {
  const memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
  const battle = new Battle(
    labScenario(loadMap("geometry").definition, [
      { side: "blue", kind: "test_tank", position: [40, 150] },
    ]),
    9,
  );
  const { requests, deliver } = workerSeam();
  const client = createSimClient({ scenario: "{}", seed: 1, side: "blue", transport: "worker" });
  const held: Publication[] = [];
  client.onPublication((publication) => held.push(publication));
  try {
    deliver({ type: "ready", layout: battle.observation_layout(), tickHz: 10, tick: 0 });
    await client.ready;
    for (let tick = 1; tick <= 2; tick++) {
      battle.step();
      const length = battle.publish("blue");
      const buffer = new Float32Array(memory.buffer, battle.publication_ptr(), length).slice()
        .buffer;
      deliver({ type: "publication", tick, digest: battle.digest(), length, buffer, stepMs: 0 });
      expect(held.at(-1)!.copyPacked()).toEqual(Array.from(new Uint32Array(buffer)));
    }
    expect(held.map((publication) => publication.tick)).toEqual([1, 2]);
    expect(held[0].copyPacked()).toHaveLength(held[0].bytes / Uint32Array.BYTES_PER_ELEMENT);
    expect(requests.filter((request) => request.type === "credit")).toEqual([]);
    const command = client.command({ kind: "stop", units: [1] });
    const advance = client.advance(1);
    const replay = client.replay();
    const outcomes = Promise.allSettled([command, advance, replay]);
    deliver({ type: "error", message: "publication exceeded admission" });
    expect(await outcomes).toEqual([
      { status: "rejected", reason: new Error("publication exceeded admission") },
      { status: "rejected", reason: new Error("publication exceeded admission") },
      { status: "rejected", reason: new Error("publication exceeded admission") },
    ]);
    expect(requests.filter((request) => request.type === "credit")).toHaveLength(2);
    held.forEach((publication) => publication.release());
    expect(requests.filter((request) => request.type === "credit")).toHaveLength(2);
  } finally {
    client.dispose();
    battle.free();
    vi.unstubAllGlobals();
  }
});

test("raw capture words replay through a fresh Rust publication decoder", () => {
  const memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
  const battle = new Battle(
    labScenario(loadMap("geometry").definition, [
      { side: "blue", kind: "test_tank", position: [40, 150] },
    ]),
    9,
  );
  const decoder = new ScenePresentation(JSON.parse(battle.observation_layout()));
  decoder.invalidate("blue");
  for (let i = 0; i < 3; i++) {
    const tick = battle.step();
    const length = battle.publish("blue");
    const words = new Uint32Array(memory.buffer, battle.publication_ptr(), length).slice();
    const floats = new Float32Array(words.buffer);
    const observation = decoder.decode(floats);
    expect(observation?.tick).toBe(tick);
  }
});

test("an invalid building preview rejects only that query and commands still work", async () => {
  const { requests, deliver } = workerSeam();
  const client = createSimClient({ scenario: "{}", seed: 1, side: "blue", transport: "worker" });
  try {
    const preview = client.previewBuilding({ units: [1], building: 9 });
    const rejected = expect(preview).rejects.toThrow("not own unit");
    deliver({ type: "building_preview", id: 1, placement: null, error: "not own unit" });
    await rejected;
    const command = client.command({ kind: "stop", units: [2] });
    const sent = requests.find((request) => request.type === "command");
    expect(sent?.command.order).toEqual({ kind: "stop", units: [2] });
    deliver({ type: "ack", ack: { seq: 1, applied_tick: 2, error: null, refused: [] } });
    expect(await command).toMatchObject({ seq: 1, error: null });
  } finally {
    client.dispose();
    vi.unstubAllGlobals();
  }
});
