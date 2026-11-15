// @vitest-environment node
import { expect, test } from "vitest";
import { loopback } from "../../../packages/multiplayer/src/link";
import { KEYFRAME, recordSource } from "../../../packages/multiplayer/src/stream/index";
import { remoteConnection } from "../../src/battle/remote/connection";
import type { SimReply } from "../../src/battle/sim/protocol";

const flush = () => new Promise<void>((resolve) => setImmediate(resolve));

import { PageClock } from "./pageClock";

function payload(tick: number) {
  const bytes = new Uint8Array(28);
  new DataView(bytes.buffer).setUint32(0, tick, true);
  new DataView(bytes.buffer).setFloat32(24, tick, true);
  return bytes;
}

async function page() {
  const clock = new PageClock();
  const pair = loopback({ side: "blue", player: "blue-1" });
  const opened: (number | null)[] = [];
  const source = recordSource(
    1024 * 1024,
    () => ({ layout: "{}", tickHz: 20 }),
    () => {},
  );
  pair.port.onReader((after, deliver) => {
    opened.push(after);
    return source.attach(after, deliver);
  });
  const seen: { reply: SimReply; at: number }[] = [];
  const failures: string[] = [];
  const channel = remoteConnection(pair.link, clock)(
    (reply) => seen.push({ reply, at: clock.now() }),
    (failure) => failures.push(failure),
  );
  const publish = (tick: number, flags = 0) => source.append("publication", flags, payload(tick));
  channel.send({ type: "connect", side: "blue" });
  publish(0, KEYFRAME);
  await flush();
  channel.send({ type: "start" });
  await flush();
  return {
    clock,
    pair,
    opened,
    source,
    channel,
    seen,
    failures,
    publish,
    publications: () =>
      seen.flatMap(({ reply, at }) =>
        reply.type === "publication" ? [{ tick: reply.tick, at }] : [],
      ),
  };
}

test("bursts reach the page one tick apart", async () => {
  const p = await page();
  try {
    p.publish(1, KEYFRAME);
    p.publish(2);
    p.publish(3);
    await flush();
    expect(p.publications()).toEqual([]);
    await p.clock.advance(500);
    const released = p.publications();
    expect(released.map((r) => r.tick)).toEqual([1, 2, 3]);
    expect(released[0].at - 10000).toBeGreaterThanOrEqual(100);
    expect(released.slice(1).map((r, i) => r.at - released[i].at)).toEqual([50, 50]);
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("an underrun holds, then resumes after a fresh delay with contiguous ticks", async () => {
  const p = await page();
  try {
    p.publish(1, KEYFRAME);
    await flush();
    await p.clock.advance(500);
    const held = p.publications();
    expect(held.map((r) => r.tick)).toEqual([1]);
    await p.clock.advance(5000);
    expect(p.publications()).toEqual(held);
    const resumed = p.clock.now();
    p.publish(2);
    p.publish(3);
    await flush();
    await p.clock.advance(99);
    expect(p.publications()).toEqual(held);
    await p.clock.advance(401);
    const released = p.publications();
    expect(released.map((r) => r.tick)).toEqual([1, 2, 3]);
    expect(released[1].at - resumed).toBeGreaterThanOrEqual(100);
    expect(released[2].at - released[1].at).toBe(50);
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("a backlog below the reopen bound fast-forwards every tick in order", async () => {
  const p = await page();
  try {
    p.publish(1, KEYFRAME);
    await flush();
    await p.clock.advance(500);
    const burst = p.clock.now();
    for (let tick = 2; tick <= 21; tick++) p.publish(tick);
    await flush();
    await p.clock.advance(50);
    const caughtUp = p.publications().filter((r) => r.at >= burst);
    expect(caughtUp.length).toBeGreaterThan(1);
    expect(caughtUp.length).toBeLessThan(20);
    await p.clock.advance(500);
    const released = p.publications();
    expect(released.map((r) => r.tick)).toEqual(Array.from({ length: 21 }, (_, i) => i + 1));
    const rest = released.filter((r) => r.at > caughtUp[0].at);
    expect(rest.slice(1).map((r, i) => r.at - rest[i].at)).toEqual(rest.slice(1).map(() => 50));
    expect(p.opened).toEqual([null, null]);
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("an over-bound suspended tab reopens without after and next receives only the new snapshot", async () => {
  const p = await page();
  try {
    p.publish(1, KEYFRAME);
    await flush();
    await p.clock.advance(500);
    for (let tick = 2; tick <= 81; tick++) p.publish(tick);
    p.source.append(
      "status",
      0,
      new TextEncoder().encode(JSON.stringify({ status: "failed", slow: true })),
    );
    await flush();
    p.clock.suspend(60000);
    await p.clock.advance(0);
    expect(p.opened).toEqual([null, null, null]);
    expect(p.publications().map((r) => r.tick)).toEqual([1]);
    // The source owns skipping the deltas before the requested snapshot.
    p.publish(82);
    p.publish(83);
    p.publish(90, KEYFRAME);
    p.publish(91);
    await flush();
    await p.clock.advance(500);
    expect(p.publications().map((r) => r.tick)).toEqual([1, 90, 91]);
    expect(p.seen.some(({ reply }) => reply.type === "status" && reply.status === "failed")).toBe(
      false,
    );
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("a status neither overtakes its preceding publication nor waits behind the following one", async () => {
  const p = await page();
  try {
    p.publish(1, KEYFRAME);
    p.source.append(
      "status",
      0,
      new TextEncoder().encode(JSON.stringify({ status: "running", slow: true })),
    );
    p.publish(2);
    await flush();
    expect(p.seen.map(({ reply }) => reply.type)).toEqual(["ready"]);
    await p.clock.advance(500);
    expect(p.seen.map(({ reply }) => reply.type)).toEqual([
      "ready",
      "publication",
      "status",
      "publication",
    ]);
    expect(p.seen[2].reply).toEqual({ type: "status", status: "running", slow: true });
    expect(p.seen[2].at).toBe(p.seen[1].at);
    expect(p.seen[2].at).toBeLessThan(p.seen[3].at);
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("an order acknowledgement and a preview answer bypass waiting publications", async () => {
  const p = await page();
  try {
    p.pair.port.onUp(async (batch) => ({
      seq: batch.seq,
      results: [
        { id: batch.items[0].id, ack: { seq: 77, applied_tick: 2, error: null, refused: [] } },
      ],
    }));
    p.pair.port.onPreview(async () => ({ moves: [] }));
    p.publish(1, KEYFRAME);
    await flush();
    const sent = p.clock.now();
    p.channel.send({
      type: "command",
      id: 9,
      order: { kind: "ready" },
      queued: false,
      seat: { side: "blue", player: "blue-1" },
    });
    p.channel.send({
      type: "move_preview",
      id: 10,
      side: "blue",
      move: { units: [], goal: [0, 0] },
    });
    await flush();
    expect(p.publications()).toEqual([]);
    expect(p.seen.slice(1)).toEqual([
      {
        reply: { type: "ack", id: 9, ack: { seq: 77, applied_tick: 2, error: null, refused: [] } },
        at: sent,
      },
      { reply: { type: "move_preview", id: 10, destinations: [] }, at: sent },
    ]);
    await p.clock.advance(500);
    expect(p.publications().map((r) => r.tick)).toEqual([1]);
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("forty jittery arrivals leave evenly and write reviewer spacing evidence", async () => {
  const p = await page();
  try {
    const origin = p.clock.now();
    const arrivals: { tick: number; at: number }[] = [];
    for (let group = 0; group < 14; group++) {
      const at = group * 150 + [0, -20, 10, -10][group % 4];
      await p.clock.advance(origin + at - p.clock.now());
      for (let tick = group * 3 + 1; tick <= Math.min(40, group * 3 + 3); tick++) {
        arrivals.push({ tick, at });
        p.publish(tick, tick === 1 ? KEYFRAME : 0);
      }
      await flush();
    }
    await p.clock.advance(500);
    const released = p.publications();
    expect(released.map((r) => r.tick)).toEqual(Array.from({ length: 40 }, (_, i) => i + 1));
    expect(released.slice(1).map((r, i) => r.at - released[i].at)).toEqual(Array(39).fill(50));
    expect(p.failures).toEqual([]);
    const { mkdir, writeFile } = await import("node:fs/promises");
    const { join } = await import("node:path");
    const directory = join(import.meta.dirname, "../../../throwaway/multiplayer/slice10");
    await mkdir(directory, { recursive: true });
    await writeFile(
      join(directory, "spacing.txt"),
      [
        "20 Hz; milliseconds relative to first arrival; three-publication bursts with jitter",
        "tick arrival_ms release_ms",
        ...arrivals.map((a, i) => `${a.tick} ${a.at} ${released[i].at - origin}`),
        "",
      ].join("\n"),
    );
  } finally {
    p.channel.close();
  }
});

test("after receiving the end, the page sends no more orders or previews while the final observation drains", async () => {
  const p = await page();
  const sent: string[] = [];
  p.pair.port.onUp(async (batch) => {
    sent.push("order");
    return { seq: batch.seq, results: [] };
  });
  p.pair.port.onPreview(async () => {
    sent.push("preview");
    return { dropped: true };
  });
  try {
    p.publish(1, KEYFRAME);
    p.source.end("finished");
    await flush();
    // The stream is over on the server, but playout has not released its
    // final observation yet. Input may still arrive from the active frame.
    p.channel.send({
      type: "command",
      id: 1,
      seat: { side: "blue", player: "blue-1" },
      order: { kind: "ready" },
      queued: false,
    });
    p.channel.send({
      type: "move_preview",
      id: 2,
      side: "blue",
      move: { units: [], goal: [0, 0] },
    });
    await flush();
    expect(sent).toEqual([]);
    await p.clock.advance(500);
    expect(p.publications().map((frame) => frame.tick)).toEqual([1]);
    expect(p.seen.filter((entry) => entry.reply.type === "status").at(-1)?.reply).toMatchObject({
      type: "status",
      status: "finished",
    });
    expect(
      p.seen.filter((entry) => entry.reply.type === "request_refused").map((entry) => entry.reply),
    ).toEqual([
      { type: "request_refused", request: "command", id: 1, reason: "ended" },
      { type: "request_refused", request: "preview", id: 2, reason: "ended" },
    ]);
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});

test("a terminal backlog releases its final observation instead of reopening a deleted battle", async () => {
  const p = await page();
  try {
    const first = 1,
      last = 100;
    for (let tick = first; tick <= last; tick++) p.publish(tick, tick === first ? KEYFRAME : 0);
    p.source.end("finished");
    await flush();
    const readers = p.opened.length;
    await p.clock.advance(1000);
    expect(p.publications().map((frame) => frame.tick)).toEqual(
      Array.from({ length: last }, (_, index) => index + first),
    );
    expect(p.opened).toHaveLength(readers);
    expect(p.seen.filter((entry) => entry.reply.type === "status").at(-1)?.reply).toMatchObject({
      status: "finished",
    });
    expect(p.failures).toEqual([]);
  } finally {
    p.channel.close();
  }
});
