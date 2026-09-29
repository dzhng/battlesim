// @vitest-environment node
import { readFileSync } from "node:fs";
import { afterEach, beforeAll, expect, test } from "vitest";
import { initSync, village_scenario } from "@wasm/game_wasm.js";
import {
  createAuthority,
  type Authority,
  type AuthorityHost,
  type SimModule,
} from "../src/battle/sim/authority";
import { simModule } from "../src/battle/sim/module";
import { MAX_CATCHUP_TICKS, PUBLICATION_POOL } from "../src/battle/sim/timing";
import type { CommandEnvelope, SimReply, SimRequest } from "../src/battle/sim/protocol";
import { decodeObservation, type ObservationLayout } from "../src/battle/sim/observation";
import village from "@fixtures/village.json";
import geometry from "@fixtures/geometry-lab.json";
import { labScenario, VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";

let sim: SimModule;
const authorities: Authority[] = [];
afterEach(() => {
  for (const authority of authorities.splice(0)) authority.handle({ type: "dispose" });
});
beforeAll(() => {
  const out = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
  sim = simModule(out.memory);
});

const scenario = labScenario(geometry, [
  { side: "blue", kind: "tank", position: [40, 150] },
  { side: "red", kind: "rifle", position: [360, 150] },
]);
const TICK_MS = 1000 / village.tick_hz;

/** A host with a hand-driven clock that really detaches transferred buffers. */
function harness(load: () => Promise<SimModule> = async () => sim) {
  const replies: SimReply[] = [];
  const returned = new Set<SimReply>();
  let clock = 0;
  let closed = false;
  const host: AuthorityHost = {
    post(reply, transfer) {
      // Detach what was transferred, as a worker would; keep the moved copy.
      const moved = (transfer ?? []).map((t) =>
        structuredClone(t as ArrayBuffer, { transfer: [t as ArrayBuffer] }),
      );
      replies.push(reply.type === "publication" ? { ...reply, buffer: moved[0] } : reply);
    },
    now: () => clock,
    schedule: () => {},
    close: () => (closed = true),
    load,
  };
  const authority = createAuthority(host);
  authorities.push(authority);
  const publications = () => replies.filter((r) => r.type === "publication");
  return {
    authority,
    replies,
    publications,
    closed: () => closed,
    advanceClock(ms: number) {
      clock += ms;
      authority.pump();
    },
    /** Return every outstanding publication's buffer as a fresh credit. */
    releaseAll() {
      for (const p of publications()) {
        if (!returned.has(p)) {
          returned.add(p);
          authority.handle({ type: "credit", buffer: new ArrayBuffer(4096) });
        }
      }
    },
    async init(replay?: string) {
      authority.handle({ type: "init", scenario, seed: 9, side: "blue", replay });
      await new Promise((r) => setTimeout(r, 0));
    },
  };
}

/** Free every buffer, then advance exactly one tick. */
function stepOnce(h: ReturnType<typeof harness>, id: number) {
  h.releaseAll();
  h.authority.handle({ type: "advance", id, ticks: 1 });
}

const move = (seq: number, units: number[]): CommandEnvelope => ({
  side: "blue",
  seq,
  queued: false,
  order: { kind: "move", units, gesture: seq, goal: [120, 150], route: "shortest" },
});

test("nothing ticks before start, then ticks follow the clock", async () => {
  const h = harness();
  await h.init();
  expect(h.replies.some((r) => r.type === "ready")).toBe(true);
  h.advanceClock(1000);
  expect(h.publications()).toHaveLength(0);
  h.authority.handle({ type: "start" });
  expect(h.publications()).toHaveLength(1);
  h.releaseAll();
  h.advanceClock(TICK_MS);
  expect(h.publications().map((p) => p.type === "publication" && p.tick)).toEqual([1, 2]);
});

test("a consumer that withholds credit stalls the battle instead of dropping ticks", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "start" });
  for (let i = 0; i < 20; i++) h.advanceClock(TICK_MS);
  expect(h.publications()).toHaveLength(PUBLICATION_POOL);
  expect(h.replies.filter((r) => r.type === "status").at(-1)).toMatchObject({
    status: "waiting-consumer",
  });
  h.releaseAll();
  h.advanceClock(TICK_MS * 30);
  const ticks = h.publications().map((p) => (p.type === "publication" ? p.tick : 0));
  expect(ticks).toEqual(ticks.map((_, i) => i + 1)); // consecutive: nothing skipped
  expect(ticks.length).toBeLessThanOrEqual(PUBLICATION_POOL + PUBLICATION_POOL);
});

test("commands are acknowledged in order with the authority's verdict", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "command", command: move(1, [0]) });
  h.authority.handle({ type: "command", command: move(2, [1]) });
  h.authority.handle({ type: "command", command: move(4, [0]) });
  const acks = h.replies.flatMap((r) => (r.type === "ack" ? [r.ack] : []));
  expect(acks.map((a) => a.seq)).toEqual([1, 2, 4]);
  expect(acks[0].error).toBeNull();
  expect(acks[1].error?.reason).toBe("not_own_unit");
  expect(acks[2].error?.reason).toBe("out_of_sequence");
});

test("wall-clock debt is shed, never spiralled", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "start" });
  // Plenty of credit; the clock jumps ten seconds.
  for (let i = 0; i < 20; i++)
    h.authority.handle({ type: "credit", buffer: new ArrayBuffer(4096) });
  const before = h.publications().length;
  h.advanceClock(10_000);
  expect(h.publications().length - before).toBeLessThanOrEqual(MAX_CATCHUP_TICKS);
  expect(h.replies.some((r) => r.type === "status" && r.slow)).toBe(true);
});

test("a scripted advance steps exactly that many ticks while paused", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "start" });
  h.authority.handle({ type: "pause" });
  h.releaseAll();
  const start = h.publications().length;
  h.authority.handle({ type: "advance", id: 1, ticks: 6 });
  while (!h.replies.some((r) => r.type === "advanced")) {
    h.releaseAll();
    h.authority.pump();
  }
  expect(h.publications().length - start).toBe(6);
  expect(h.replies.find((r) => r.type === "advanced")).toMatchObject({ id: 1, tick: start + 6 });
  h.advanceClock(1000);
  expect(h.publications().length - start).toBe(6); // still paused
});

test("dispose stops all work and closes the host", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "start" });
  h.authority.handle({ type: "dispose" });
  const count = h.publications().length;
  h.releaseAll();
  h.advanceClock(1000);
  expect(h.publications().length).toBe(count);
  expect(h.closed()).toBe(true);
});

test("a failed load reports an error instead of loading forever", async () => {
  const h = harness(async () => {
    throw new Error("no wasm here");
  });
  await h.init();
  expect(h.replies).toContainEqual({ type: "error", message: "no wasm here" });
  expect(h.closed()).toBe(true);
});

test("a replay of the accepted commands reproduces every tick digest", async () => {
  const live = harness();
  await live.init();
  live.authority.handle({ type: "start" });
  live.authority.handle({ type: "pause" });
  live.authority.handle({ type: "command", command: move(1, [0]) });
  for (let i = 0; i < 40; i++) stepOnce(live, i);
  live.authority.handle({ type: "replay" });
  const json = (live.replies.find((r) => r.type === "replay") as { json: string }).json;
  const digests = live.publications().map((p) => (p.type === "publication" ? p.digest : ""));

  const again = harness();
  await again.init(json);
  again.authority.handle({ type: "start" });
  again.authority.handle({ type: "pause" });
  for (let i = 0; i < digests.length; i++) stepOnce(again, i);
  const replayed = again.publications().map((p) => (p.type === "publication" ? p.digest : ""));
  expect(replayed.slice(0, digests.length)).toEqual(digests);
  again.authority.handle({ type: "command", command: move(1, [0]) });
  const ack = again.replies.flatMap((r) => (r.type === "ack" ? [r.ack] : []))[0];
  expect(ack.error?.reason).toBe("replay_in_progress");
});

test("the ground streams as deltas, and a side switch reopens it with a full snapshot", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "start" });
  h.authority.handle({ type: "pause" });
  for (let i = 0; i < 5; i++) stepOnce(h, i);
  h.authority.handle({ type: "side", side: "red" });
  for (let i = 5; i < 8; i++) stepOnce(h, i);
  const layout = JSON.parse(
    (h.replies.find((r) => r.type === "ready") as { layout: string }).layout,
  ) as ObservationLayout;
  const patches = h
    .publications()
    .flatMap((p) =>
      p.type === "publication"
        ? [decodeObservation(layout, new Float32Array(p.buffer, 0, p.length)).groundPatch]
        : [],
    );
  const shape = patches.map((p) => [p.epoch, p.side, p.full]);
  expect(shape).toEqual([
    [1, "blue", true],
    ...Array.from({ length: 5 }, () => [1, "blue", false]),
    [2, "red", true],
    [2, "red", false],
    [2, "red", false],
  ]);
});

test("a scripted blue commands like a player: recorded, replayable, timed per step", async () => {
  const setup = village_scenario(JSON.stringify(VILLAGE_RULES), "ordinary");
  const run = async (init: SimRequest) => {
    const h = harness();
    h.authority.handle(init);
    await new Promise((r) => setTimeout(r, 0));
    h.authority.handle({ type: "start" });
    h.authority.handle({ type: "pause" });
    for (let i = 0; i < 30; i++) stepOnce(h, i);
    return h;
  };
  const published = (h: ReturnType<typeof harness>) =>
    h.publications().flatMap((p) => (p.type === "publication" ? [p] : []));
  const live = await run({
    type: "init",
    scenario: setup,
    seed: 3,
    side: "blue",
    script: "scout-suppress-flank",
  });
  expect(published(live).length).toBeGreaterThan(20);
  expect(published(live).every((p) => p.stepMs >= 0)).toBe(true);
  live.authority.handle({ type: "replay" });
  const json = (live.replies.find((r) => r.type === "replay") as { json: string }).json;
  const accepted = (JSON.parse(json) as { accepted: [number, { side: string }][] }).accepted;
  expect(accepted.some(([, c]) => c.side === "blue")).toBe(true);

  const again = await run({ type: "init", scenario: setup, seed: 3, side: "blue", replay: json });
  expect(published(again).map((p) => p.digest)).toEqual(published(live).map((p) => p.digest));
});
