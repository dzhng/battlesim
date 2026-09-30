// @vitest-environment node
import { expect, test, vi } from "vitest";
import { readFileSync } from "node:fs";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { createSimClient, type Publication } from "../src/battle/sim/client";
import type { SimReply, SimRequest } from "../src/battle/sim/protocol";

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
  const outcomes = Promise.allSettled([client.ready, command, advance, replay]);
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
    terminate() {}
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
    deliver: (reply: SimReply) => worker.deliver(reply),
  };
}

test("an authority failure returns held records once and rejects requests after ready", async () => {
  const memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
  const oracle = JSON.parse(
    readFileSync(
      new URL("../../specs/city-maps/assets/fog-delivery/oracle.json", import.meta.url),
      "utf8",
    ),
  );
  const battle = new Battle(JSON.stringify(oracle.scenario), oracle.seed);
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
    }
    expect(held.map((publication) => publication.tick)).toEqual([1, 2]);
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
}, 1000);
