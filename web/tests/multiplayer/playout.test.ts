// @vitest-environment node
import { expect, test } from "vitest";
import type { SimReply } from "../../src/battle/sim/protocol";
import { playout } from "../../src/battle/remote/playout";

const publication = (tick: number): SimReply => ({
  type: "publication",
  side: "blue",
  tick,
  digest: `digest-${tick}`,
  length: 0,
  buffer: new ArrayBuffer(0),
  stepMs: 0,
});
const status: SimReply = { type: "status", status: "running", slow: false };

const TICK_HZ = 10;
const TICK_MS = 1000 / TICK_HZ;
const DELAY_MS = 2 * TICK_MS;

const ticks = (replies: SimReply[]) =>
  replies.flatMap((reply) => (reply.type === "publication" ? [reply.tick] : []));

/** Play `feed` frame by frame and say when each tick was handed on. */
function play(feed: ReturnType<typeof playout>, from: number, to: number, frameMs = 10) {
  const released: [tick: number, at: number][] = [];
  for (let now = from; now <= to; now += frameMs)
    for (const tick of ticks(feed.due(now))) released.push([tick, now]);
  return released;
}

test("steady arrival plays a tick apart, a fixed delay behind", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  const released: [number, number][] = [];
  for (let tick = 1; tick <= 6; tick++) {
    const arrived = tick * TICK_MS;
    feed.push(publication(tick), arrived);
    released.push(...play(feed, arrived, arrived + TICK_MS - 10));
  }
  released.push(...play(feed, 7 * TICK_MS, 10 * TICK_MS));
  expect(released).toEqual([1, 2, 3, 4, 5, 6].map((tick) => [tick, tick * TICK_MS + DELAY_MS]));
});

test("a small burst leaves a tick apart, not all at once", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  for (let tick = 1; tick <= 3; tick++) feed.push(publication(tick), 0);
  const released = play(feed, 0, 10 * TICK_MS);
  expect(released.map(([tick]) => tick)).toEqual([1, 2, 3]);
  expect(released[1][1] - released[0][1]).toBe(TICK_MS);
  expect(released[2][1] - released[1][1]).toBe(TICK_MS);
});

test("what is not a publication keeps its place in the stream", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  feed.push(status, 0);
  feed.push(publication(1), 0);
  feed.push(status, 0);
  feed.push(publication(2), 0);
  // The first status waits for nothing; the second waits for the tick before it.
  expect(feed.due(0)).toEqual([status]);
  expect(feed.due(DELAY_MS).map((reply) => reply.type)).toEqual(["publication", "status"]);
});

test("an empty buffer holds, and the next arrival is held for the delay again", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  feed.push(publication(1), 0);
  expect(ticks(feed.due(DELAY_MS))).toEqual([1]);
  // The stream stalls: nothing is due, and nothing is invented.
  expect(feed.due(DELAY_MS + 5 * TICK_MS)).toEqual([]);
  const resumed = 20 * TICK_MS;
  feed.push(publication(2), resumed);
  expect(feed.due(resumed + DELAY_MS - 1)).toEqual([]);
  expect(ticks(feed.due(resumed + DELAY_MS))).toEqual([2]);
});

test("a page that fell behind fast-forwards through every tick, then plays evenly again", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  feed.push(publication(1), 0);
  expect(ticks(feed.due(DELAY_MS))).toEqual([1]);
  // A stall, then everything it held back arrives at once.
  const burst = 30 * TICK_MS;
  for (let tick = 2; tick <= 13; tick++) feed.push(publication(tick), burst);
  const waiting = feed.queuedMs();
  const caughtUp = ticks(feed.due(burst));
  // None is dropped and their order holds; most leave at once.
  expect(caughtUp[0]).toBe(2);
  expect(caughtUp).toEqual([...caughtUp].sort((a, b) => a - b));
  expect(caughtUp.length).toBeGreaterThan(1);
  expect(feed.queuedMs()).toBeLessThan(waiting);
  const rest = play(feed, burst + 10, burst + 20 * TICK_MS);
  expect([...caughtUp, ...rest.map(([tick]) => tick)]).toEqual([
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
  ]);
  for (let i = 1; i < rest.length; i++) expect(rest[i][1] - rest[i - 1][1]).toBe(TICK_MS);
  // Caught up: what arrives next is no further behind than a fresh stream.
  const next = burst + 21 * TICK_MS;
  feed.push(publication(14), next);
  expect(play(feed, next, next + 10 * TICK_MS)).toEqual([[14, next + DELAY_MS]]);
});

test("the battle time waiting grows with each publication held and falls as they play", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  expect(feed.queuedMs()).toBe(0);
  feed.push(status, 0);
  expect(feed.queuedMs()).toBe(0);
  feed.push(publication(1), 0);
  feed.push(publication(2), 0);
  expect(feed.queuedMs()).toBe(2 * TICK_MS);
  feed.due(DELAY_MS);
  expect(feed.queuedMs()).toBe(TICK_MS);
});

test("a reopened stream starts afresh from its first record", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  for (let tick = 1; tick <= 40; tick++) feed.push(publication(tick), 0);
  feed.reset();
  expect(feed.queuedMs()).toBe(0);
  const reopened = 5 * TICK_MS;
  feed.push(publication(90), reopened);
  expect(feed.due(reopened + DELAY_MS - 1)).toEqual([]);
  expect(ticks(feed.due(reopened + DELAY_MS))).toEqual([90]);
});

test("the delay a stall adds is played off, a little fast, instead of kept for the rest of the battle", () => {
  const feed = playout(TICK_HZ, DELAY_MS);
  const arrivals = new Map<number, number>();
  const released: [tick: number, at: number][] = [];
  // A stall longer than the buffer: ticks 4 to 6 arrive together, late.
  const STALLED_UNTIL = 6.5 * TICK_MS;
  const arrival = (tick: number) => (tick >= 4 && tick <= 6 ? STALLED_UNTIL : tick * TICK_MS);
  let next = 1;
  for (let now = 0; now <= 60 * TICK_MS; now += 10) {
    for (; next <= 55 && arrival(next) <= now; next++) {
      arrivals.set(next, now);
      feed.push(publication(next), now);
    }
    for (const tick of ticks(feed.due(now))) released.push([tick, now]);
  }
  expect(released.map(([tick]) => tick)).toEqual(Array.from({ length: 55 }, (_, i) => i + 1));
  const waited = (index: number) => released[index][1] - arrivals.get(released[index][0])!;
  // Right after the stall the page is behind by the stall as well as the buffer.
  expect(waited(7)).toBeGreaterThan(DELAY_MS + TICK_MS);
  // It never jumps: no tick leaves closer than half a tick behind the one before.
  for (let i = 1; i < released.length; i++)
    expect(released[i][1] - released[i - 1][1]).toBeGreaterThanOrEqual(TICK_MS / 2);
  // In the end a tick waits the buffer again, and plays a tick apart.
  for (let i = 45; i < 55; i++) {
    expect(waited(i)).toBeLessThanOrEqual(DELAY_MS);
    expect(waited(i)).toBeGreaterThan(DELAY_MS - TICK_MS);
    expect(released[i][1] - released[i - 1][1]).toBe(TICK_MS);
  }
});
