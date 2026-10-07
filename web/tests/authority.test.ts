// @vitest-environment node
import { readFileSync } from "node:fs";
import { afterEach, beforeAll, expect, test } from "vitest";
import { initSync } from "@wasm/game_wasm.js";
import {
  createAuthority,
  type Authority,
  type AuthorityHost,
  type SimModule,
} from "../src/battle/sim/authority";
import { simModule } from "../src/battle/sim/module";
import { MAX_CATCHUP_TICKS, PUBLICATION_POOL } from "../src/battle/sim/timing";
import type { CommandEnvelope, SimReply, SimRequest } from "../src/battle/sim/protocol";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import game from "@fixtures/game.json";
import { loadMap } from "@web/maps/node";
import { labScenario, TEST_RULES } from "./catalog";

const geometry = loadMap("geometry").definition;

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
  { side: "blue", kind: "test_tank", position: [40, 150] },
  { side: "red", kind: "test_rifle", position: [360, 150] },
]);
const TICK_MS = 1000 / game.tick_hz;

/** A host with a hand-driven clock that really detaches transferred buffers. */
function harness(load: () => Promise<SimModule> = async () => sim) {
  const replies: SimReply[] = [];
  const returned = new Set<SimReply>();
  let clock = 0;
  let closed = false;
  const host: AuthorityHost = {
    post(reply, transfer) {
      replies.push(transfer?.length ? structuredClone(reply, { transfer }) : reply);
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
    async init(replay?: string, setup = scenario) {
      authority.handle({
        type: "init",
        scenario: setup,
        seed: 9,
        side: "blue",
        replay,
      });
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
  order: {
    kind: "move",
    units,
    gesture: seq,
    goal: [120, 150],
    route: "shortest",
  },
});

test("a paused formation preview returns only own destinations without issuing an order", async () => {
  const h = harness();
  await h.init();
  h.authority.handle({ type: "start" });
  h.authority.handle({ type: "pause" });
  const count = h.publications().length;
  const moveRequest = {
    units: [0],
    goal: [120, 150] as [number, number],
    facing: Math.PI / 2,
  };
  h.authority.handle({
    type: "move_preview",
    id: 1,
    side: "blue",
    move: moveRequest,
  });
  h.authority.handle({
    type: "move_preview",
    id: 2,
    side: "blue",
    move: { ...moveRequest, units: [1] },
  });
  expect(h.replies.filter((r) => r.type === "move_preview")).toEqual([
    {
      type: "move_preview",
      id: 1,
      destinations: [
        {
          unit: 0,
          placed: true,
          goal: [120, 150],
          facing: expect.closeTo(Math.PI / 2, 12),
        },
      ],
    },
    { type: "move_preview", id: 2, destinations: [] },
  ]);
  expect(h.publications().length).toBe(count);
  h.authority.handle({ type: "command", command: move(1, [0]) });
  expect(h.replies.filter((r) => r.type === "ack")).toEqual([
    {
      type: "ack",
      ack: {
        seq: 1,
        applied_tick: 2,
        error: null,
        placement: {
          gesture: 1,
          destinations: [{ unit: 0, placed: true, goal: [120, 150], facing: 0 }],
        },
      },
    },
  ]);
});

test("building previews preserve battle outcomes and an invalid query leaves the authority usable", async () => {
  const setup = labScenario(loadMap("garrison").definition, [
    { side: "blue", kind: "test_recon", position: [325, 250] },
    { side: "blue", kind: "test_tank", position: [290, 250] },
  ]);
  const queried = harness();
  const untouched = harness();
  await queried.init(undefined, setup);
  await untouched.init(undefined, setup);
  for (const h of [queried, untouched]) {
    h.authority.handle({ type: "start" });
    h.authority.handle({ type: "pause" });
  }
  const publications = queried.publications().length;
  queried.authority.handle({
    type: "building_preview",
    id: 1,
    side: "blue",
    building: { units: [0, 1], building: 0 },
  });
  expect(queried.replies.find((r) => r.type === "building_preview" && r.id === 1)).toMatchObject({
    placement: {
      building: 0,
      entrant: { unit: 0 },
      destinations: [{ unit: 1, placed: true }],
    },
  });
  queried.authority.handle({
    type: "building_preview",
    id: 2,
    side: "red",
    building: { units: [0], building: 0 },
  });
  expect(queried.replies.find((r) => r.type === "building_preview" && r.id === 2)).toMatchObject({
    placement: null,
    error: expect.any(String),
  });
  expect(queried.publications()).toHaveLength(publications);
  expect(queried.closed()).toBe(false);
  const command: CommandEnvelope = {
    side: "blue",
    seq: 1,
    queued: false,
    order: { kind: "occupy_building", units: [0, 1], building: 0, gesture: 1 },
  };
  for (const h of [queried, untouched]) {
    h.authority.handle({ type: "command", command });
    for (let i = 0; i < 20; i++) stepOnce(h, i);
    h.authority.handle({ type: "replay" });
  }
  expect(queried.replies.filter((r) => r.type === "ack")).toEqual(
    untouched.replies.filter((r) => r.type === "ack"),
  );
  expect(queried.publications().map((p) => p.digest)).toEqual(
    untouched.publications().map((p) => p.digest),
  );
  expect(queried.replies.find((r) => r.type === "replay")).toEqual(
    untouched.replies.find((r) => r.type === "replay"),
  );
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
  expect(h.replies.find((r) => r.type === "advanced")).toMatchObject({
    id: 1,
    tick: start + 6,
  });
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

test("a different simulation build is refused before replay publication", async () => {
  const live = harness();
  await live.init();
  live.authority.handle({ type: "replay" });
  const json = (live.replies.find((r) => r.type === "replay") as { json: string }).json;
  const record = JSON.parse(json);
  record.engine_build = "0".repeat(64);
  const replay = harness();
  await replay.init(JSON.stringify(record));
  expect(replay.replies).toContainEqual({
    type: "error",
    message: "replay was recorded by a different simulation build",
  });
  expect(replay.replies.some((r) => r.type === "ready")).toBe(false);
  expect(replay.publications()).toHaveLength(0);
  expect(replay.closed()).toBe(true);
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
  const decoder = new ObservationDecoder(layout);
  const patches = h
    .publications()
    .flatMap((p) =>
      p.type === "publication"
        ? [decoder.decode(new Float32Array(p.buffer, 0, p.length))!.groundPatch]
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

test("worker publication copying and transfer preserve every raw NaN carrier bit", async () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  const bits = Uint32Array.from([0x7f800001, 0x7fc12345, 0xff800001, 0x80000000, 0xffffffff]);
  new Uint32Array(memory.buffer, 0, bits.length).set(bits);
  let tick = 0;
  const battle = {
    observation_layout: () => "{}",
    accept: () => "{}",
    preview_purchase: () => '{"Err":{"reason":"not_skirmish"}}',
    preview_move: () => "[]",
    preview_building: () => "null",
    step: () => ++tick,
    tick: () => tick,
    finished: () => false,
    digest: () => "carrier",
    replay_json: () => "{}",
    publish: () => bits.length,
    resync_observation: () => {},
    publication_ptr: () => 0,
    free: () => {},
  };
  const h = harness(async () => ({
    memory,
    createBattle: () => battle,
    replayBattle: () => battle,
  }));
  await h.init();
  h.authority.handle({ type: "start" });
  const publication = h.publications()[0];
  expect(new Uint32Array(publication.buffer, 0, publication.length)).toEqual(bits);
});

test("terminal matches complete fast-forward at their final tick and stop publishing", async () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let tick = 0;
  const battle = {
    observation_layout: () => "{}",
    accept: () => "{}",
    preview_purchase: () => '{"Err":{"reason":"not_skirmish"}}',
    preview_move: () => "[]",
    preview_building: () => "null",
    step: () => (tick < 2 ? ++tick : tick),
    tick: () => tick,
    finished: () => tick === 2,
    digest: () => "final",
    replay_json: () => "{}",
    publish: () => 1,
    resync_observation: () => {},
    publication_ptr: () => 0,
    free: () => {},
  };
  const h = harness(async () => ({
    memory,
    createBattle: () => battle,
    replayBattle: () => battle,
  }));
  await h.init();
  h.authority.handle({ type: "start" });
  h.authority.handle({ type: "advance", id: 7, ticks: 100 });
  expect(h.replies.find((r) => r.type === "advanced")).toEqual({
    type: "advanced",
    id: 7,
    tick: 2,
  });
  expect(h.publications().map((p) => p.tick)).toEqual([1, 2]);
  expect(h.replies.filter((r) => r.type === "status").at(-1)).toMatchObject({
    status: "finished",
  });
  h.releaseAll();
  h.advanceClock(10000);
  h.authority.handle({ type: "advance", id: 8, ticks: 10 });
  expect(h.replies.find((r) => r.type === "advanced" && r.id === 8)).toEqual({
    type: "advanced",
    id: 8,
    tick: 2,
  });
  expect(h.publications().map((p) => p.tick)).toEqual([1, 2]);
});

test("purchase preview resolves through the authority without spending or advancing", async () => {
  const { default: record } = await import("@fixtures/parity/skirmish-purchases.json");
  const h = harness();
  await h.init(undefined, JSON.stringify(record.scenario));
  h.authority.handle({
    type: "purchase_preview",
    id: 1,
    side: "blue",
    variant: "test_tank",
    destination: [400, 250],
  });
  h.authority.handle({
    type: "purchase_preview",
    id: 2,
    side: "red",
    variant: "test_tank",
    destination: [400, 250],
  });
  expect(h.replies.filter((r) => r.type === "purchase_preview")).toEqual([
    { type: "purchase_preview", id: 1, placement: { Ok: expect.any(Number) } },
    { type: "purchase_preview", id: 2, placement: { Err: { reason: "wrong_faction" } } },
  ]);
  expect(h.publications()).toHaveLength(0);
  h.authority.handle({ type: "start" });
  const layout = JSON.parse(
    (h.replies.find((r) => r.type === "ready") as { layout: string }).layout,
  ) as ObservationLayout;
  const p = h.publications()[0];
  const view = new ObservationDecoder(layout).decode(new Float32Array(p.buffer, 0, p.length))!;
  expect(view.skirmish).toMatchObject({ credits: 1000, occupiedSlots: 0, pending: [] });
  expect(view.own).toEqual([]);
});
