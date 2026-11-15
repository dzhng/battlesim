// @vitest-environment node
import { timerClock } from "./timerClock";
import { afterEach, expect, test, vi } from "vitest";
import { loopback } from "../../../packages/multiplayer/src/link";
import { KEYFRAME, recordSource } from "../../../packages/multiplayer/src/stream/index";
import type { UpBatch, UpReply } from "../../../packages/multiplayer/src/wire";
import { remoteConnection } from "../../src/battle/remote/connection";
import type { SimReply, SimRequest } from "../../src/battle/sim/protocol";

const close: (() => void)[] = [];
afterEach(() => {
  for (const stop of close.splice(0)) stop();
  vi.useRealTimers();
});
const flush = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve();
};
const init: SimRequest = { type: "connect", side: "blue" };
const command = (id: number): SimRequest => ({
  type: "command",
  id,
  order: { kind: "ready" },
  queued: false,
  seat: { side: "red", player: "forged" },
});
const answer = (batch: UpBatch): UpReply => ({
  seq: batch.seq,
  results: batch.items.map((item) => ({
    id: item.id,
    ack: { seq: 100 + item.id, applied_tick: 7, error: null, refused: [] },
  })),
});

function connected() {
  const pair = loopback({ side: "blue", player: "blue-1" });
  let asked = 0;
  const source = recordSource(
    1024,
    () => ({ layout: "{}", tickHz: 30, status: { status: "running", slow: false } }),
    () => void asked++,
  );
  let readers = 0;
  pair.port.onReader((after, deliver) => {
    readers++;
    return source.attach(after, deliver);
  });
  const replies: SimReply[] = [];
  const failures: string[] = [];
  const channel = remoteConnection(pair.link, timerClock())(
    (reply) => replies.push(reply),
    (error) => failures.push(error),
  );
  close.push(channel.close);
  channel.send(init);
  return {
    ...pair,
    channel,
    replies,
    failures,
    source,
    readers: () => readers,
    keyframesAsked: () => asked,
  };
}

test("only one order batch is in flight; its late reply retries the same seq and later orders keep their order", async () => {
  vi.useFakeTimers();
  const c = connected();
  const batches: UpBatch[] = [];
  let first!: (reply: UpReply) => void;
  c.port.onUp((batch) => {
    batches.push(structuredClone(batch));
    if (batches.length === 1)
      return new Promise((resolve) => {
        first = resolve;
      });
    return Promise.resolve(answer(batch));
  });
  c.channel.send(command(1));
  await flush();
  c.channel.send(command(2));
  c.channel.send(command(3));
  await flush();
  expect(batches.map((b) => b.items.map((i) => i.id))).toEqual([[1]]);
  await vi.runOnlyPendingTimersAsync();
  await flush();
  expect(batches.map((b) => b.items.map((i) => i.id))).toEqual([[1], [1], [2, 3]]);
  expect(batches[1]).toEqual(batches[0]);
  expect(batches[2].seq).toBe(batches[0].seq + 1);
  first(answer(batches[0]));
  await flush();
  expect(c.replies.filter((r) => r.type === "ack")).toEqual(
    [1, 2, 3].map((id) => ({
      type: "ack",
      id,
      ack: { seq: 100 + id, applied_tick: 7, error: null, refused: [] },
    })),
  );
  expect(c.failures).toEqual([]);
});

test("previews keep only the newest waiting intent, settle dropped requests, and never retry", async () => {
  vi.useFakeTimers();
  const c = connected();
  const destinations: number[] = [];
  let finish!: (answer: { moves: [] }) => void;
  c.port.onPreview((query) => {
    if (query.kind !== "move") throw new Error("wrong query");
    destinations.push(query.move.goal[0]);
    if (destinations.length === 1)
      return new Promise((resolve) => {
        finish = resolve;
      });
    return Promise.resolve({ dropped: true });
  });
  const move = (id: number): SimRequest => ({
    type: "move_preview",
    id,
    side: "blue",
    move: { units: [], goal: [id, 0] },
  });
  c.channel.send(move(1));
  await flush();
  c.channel.send(move(2));
  c.channel.send(move(3));
  await flush();
  expect(destinations).toEqual([1]);
  expect(c.replies).toContainEqual({
    type: "request_refused",
    request: "preview",
    id: 2,
    reason: "superseded",
  });
  finish({ moves: [] });
  await flush();
  expect(destinations).toEqual([1, 3]);
  expect(c.replies).toContainEqual({ type: "move_preview", id: 1, destinations: [] });
  expect(c.replies).toContainEqual({
    type: "request_refused",
    request: "preview",
    id: 3,
    reason: "dropped",
  });
  await vi.runOnlyPendingTimersAsync();
  expect(destinations).toEqual([1, 3]);
  expect(c.failures).toEqual([]);
});

test("a batch refusal settles its own orders and the next batch can still succeed", async () => {
  const c = connected();
  let count = 0;
  c.port.onUp(async (batch) =>
    ++count === 1 ? { seq: batch.seq, refused: "rate" } : answer(batch),
  );
  c.channel.send(command(1));
  await flush();
  c.channel.send(command(2));
  await flush();
  expect(c.replies).toEqual([
    { type: "request_refused", request: "command", id: 1, reason: "rate" },
    { type: "ack", id: 2, ack: { seq: 102, applied_tick: 7, error: null, refused: [] } },
  ]);
  expect(c.failures).toEqual([]);
});

test("nothing of the battle is kept while the page loads; it starts on a snapshot taken then, resumes after a cut, and an end never reconnects", async () => {
  vi.useFakeTimers();
  const c = connected();
  const payload = (tick: number) => {
    const bytes = new Uint8Array(28);
    const view = new DataView(bytes.buffer);
    view.setUint32(0, tick, true);
    bytes.set(new TextEncoder().encode("0123456789abcdef"), 4);
    view.setFloat32(20, 1.25, true);
    view.setFloat32(24, tick * 2, true);
    return bytes;
  };
  const published = () =>
    c.replies.flatMap((r) =>
      r.type === "publication"
        ? [[r.side, r.tick, r.digest, r.stepMs, ...new Float32Array(r.buffer)]]
        : [],
    );
  c.source.append("publication", KEYFRAME, payload(40));
  await flush();
  expect(c.replies).toEqual([{ type: "ready", layout: "{}", tickHz: 30, tick: 40 }]);

  // The page is still loading, for as long as it likes. The battle goes on.
  for (let tick = 41; tick <= 400; tick++) c.source.append("publication", 0, payload(tick));
  await flush();
  expect(c.replies).toHaveLength(1);
  expect(c.failures).toEqual([]);

  // It starts: it asks for a snapshot, and takes nothing older than it.
  const asked = c.keyframesAsked();
  c.channel.send({ type: "start" });
  await flush();
  expect(c.keyframesAsked()).toBe(asked + 1);
  c.source.append("publication", 0, payload(401));
  c.source.append("publication", KEYFRAME, payload(402));
  c.source.append("publication", 0, payload(403));
  await flush();
  await vi.advanceTimersByTimeAsync(250);
  // A battle already running says so to a page that missed it starting.
  expect(c.replies[1]).toEqual({ type: "status", status: "running", slow: false });
  expect(published()).toEqual([
    ["blue", 402, "0123456789abcdef", 1.25, 804],
    ["blue", 403, "0123456789abcdef", 1.25, 806],
  ]);

  // A cut resumes where it stopped, with no new snapshot.
  c.cut();
  c.source.append("publication", 0, payload(404));
  await vi.advanceTimersByTimeAsync(200);
  expect(published().map((p) => p[1])).toEqual([402, 403, 404]);
  expect(c.keyframesAsked()).toBe(asked + 1);
  // Credits neither cause a request nor block the next publication.
  const last = c.replies.filter((r) => r.type === "publication").at(-1)!;
  c.channel.send({ type: "credit", buffer: last.buffer });
  c.source.append("publication", 0, payload(405));
  await flush();
  await vi.advanceTimersByTimeAsync(200);
  expect(published().map((p) => p[1])).toEqual([402, 403, 404, 405]);

  c.source.end("finished");
  await flush();
  await vi.advanceTimersByTimeAsync(100);
  expect(c.replies.at(-1)).toEqual({ type: "status", status: "finished", slow: false });
  await vi.advanceTimersByTimeAsync(200);
  // One connection to learn the battle, one to follow it, one after the cut.
  expect(c.readers()).toBe(3);
  expect(c.failures).toEqual([]);
});

test("a page reloaded onto its seat sends batches the server takes as later than its old ones", async () => {
  const numbers: number[] = [];
  for (const _page of [1, 2]) {
    const c = connected();
    c.port.onUp((batch) => {
      numbers.push(batch.seq);
      return Promise.resolve(answer(batch));
    });
    c.channel.send(command(1));
    await flush();
    c.channel.send(command(2));
    await flush();
    c.channel.close();
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
  expect(numbers).toHaveLength(4);
  expect(numbers).toEqual([...numbers].sort((a, b) => a - b));
  expect(new Set(numbers).size).toBe(4);
});

test("a finished battle settles orders and previews still waiting for a reply", async () => {
  vi.useFakeTimers();
  const c = connected();
  const payload = new Uint8Array(28);
  new DataView(payload.buffer).setUint32(0, 1, true);
  c.source.append("publication", 1, payload);
  await flush();
  c.channel.send({ type: "start" });
  c.port.onUp(() => new Promise(() => {}));
  c.port.onPreview(() => new Promise(() => {}));
  c.channel.send(command(1));
  c.channel.send({ type: "move_preview", id: 2, side: "blue", move: { units: [], goal: [0, 0] } });
  await flush();
  c.source.end("finished");
  await flush();
  await vi.advanceTimersByTimeAsync(100);
  expect(c.replies).toContainEqual({
    type: "request_refused",
    request: "command",
    id: 1,
    reason: "ended",
  });
  expect(c.replies).toContainEqual({
    type: "request_refused",
    request: "preview",
    id: 2,
    reason: "ended",
  });
  c.channel.send(command(3));
  c.channel.send({ type: "move_preview", id: 4, side: "blue", move: { units: [], goal: [0, 0] } });
  expect(c.replies).toContainEqual({
    type: "request_refused",
    request: "command",
    id: 3,
    reason: "ended",
  });
  expect(c.replies).toContainEqual({
    type: "request_refused",
    request: "preview",
    id: 4,
    reason: "ended",
  });
  expect(c.failures).toEqual([]);
});
