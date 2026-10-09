// @vitest-environment node
import { expect, test, vi } from "vitest";
import { readFileSync } from "node:fs";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { createSimClient, type Publication } from "../src/battle/sim/client";
import type { SimReply, SimRequest } from "../src/battle/sim/protocol";
import { labScenario } from "./catalog";
import { loadMap } from "@web/maps/node";

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
    deliver({ type: "ack", ack: { seq: 1, applied_tick: 2, error: null } });
    expect(await command).toMatchObject({ seq: 1, error: null });
  } finally {
    client.dispose();
    vi.unstubAllGlobals();
  }
});
