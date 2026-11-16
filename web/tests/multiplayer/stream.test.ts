// @vitest-environment node
import { expect, test } from "vitest";
import {
  KEYFRAME,
  encodeRecord,
  jsonRecord,
  readJson,
  recordDecoder,
  recordRing,
  recordSource,
  resumingReader,
  type RecordSource,
} from "../../../packages/multiplayer/src/stream/index";
import { loopback } from "../../../packages/multiplayer/src/link";
import type { StreamOpen, StreamRecord } from "../../../packages/multiplayer/src/wire";

const bytes = (...values: number[]) => new Uint8Array(values);
const instant = { sleep: async () => {} };

/** A producer that answers a keyframe request with its next record. */
function producer(limitBytes = 1 << 16) {
  let keyframeDue = false;
  let asked = 0;
  const source: RecordSource = recordSource(
    limitBytes,
    () => ({ tickHz: 30 }),
    () => {
      keyframeDue = true;
      asked += 1;
    },
  );
  return {
    source,
    asked: () => asked,
    /** Publish one record carrying `value`; a keyframe when one was asked for. */
    publish(value: number) {
      const flags = keyframeDue ? KEYFRAME : 0;
      keyframeDue = false;
      return source.append("publication", flags, bytes(value));
    },
  };
}

/** Settle every promise the last step set going. */
const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

test("records decode the same however the bytes are chunked", () => {
  const records: StreamRecord[] = [
    jsonRecord("open", { resume: "keyframe" }),
    { id: 1, kind: "publication", flags: KEYFRAME, payload: bytes(1, 2, 3, 4, 5) },
    { id: 2, kind: "status", flags: 0, payload: bytes() },
    { id: 0xffffffff, kind: "end", flags: 0, payload: bytes(9) },
  ];
  const whole = new Uint8Array(records.flatMap((record) => [...encodeRecord(record)]));
  const decode = (chunks: Uint8Array[]) => {
    const decoder = recordDecoder();
    return chunks.flatMap((chunk) => decoder.push(chunk));
  };
  expect(decode([whole])).toEqual(records);
  expect(decode([...whole].map((byte) => bytes(byte)))).toEqual(records);
  for (let first = 0; first <= whole.length; first++)
    for (let second = first; second <= whole.length; second += 7)
      expect(
        decode([whole.slice(0, first), whole.slice(first, second), whole.slice(second)]),
        `cut at ${first} and ${second}`,
      ).toEqual(records);
});

test("the ring answers a place it still holds and reports a gap for one it dropped", () => {
  const ring = recordRing(3 * 16);
  for (let value = 1; value <= 5; value++) ring.append("publication", 0, bytes(value));
  expect(ring.since(1)).toBe("gap");
  expect(ring.since(2)).toEqual([3, 4, 5].map((id) => expect.objectContaining({ id })));
  expect(ring.since(5)).toEqual([]);
  // A place the ring never reached is no place in this stream.
  expect(ring.since(9)).toBe("gap");
});

test.each([1, 2, 3, 4, 5])(
  "a reader cut after record %i resumes with nothing lost and nothing repeated",
  async (cutAfter) => {
    const { source, publish } = producer();
    const { link, port, cut } = loopback({ side: "blue", player: "blue-1" });
    port.onReader((after, deliver) => source.attach(after, deliver));
    const seen: number[] = [];
    const reading = (async () => {
      for await (const record of resumingReader(link.records, instant)) {
        if (record.kind === "publication") seen.push(record.payload[0]);
        if (seen.length === 6) break;
      }
    })();
    await settle();
    for (let value = 1; value <= 6; value++) {
      publish(value);
      await settle();
      if (value === cutAfter) {
        cut();
        // Published while nobody is connected: the ring holds it.
        publish(++value);
        await settle();
      }
    }
    await reading;
    expect(seen).toEqual([1, 2, 3, 4, 5, 6]);
  },
);

test("a reader whose place has left the ring restarts at the next keyframe", async () => {
  const { source, publish, asked } = producer(2 * 16);
  const { link, port, cut } = loopback({ side: "red", player: "red-1" });
  port.onReader((after, deliver) => source.attach(after, deliver));
  const opens: string[] = [];
  const seen: { value: number; keyframe: boolean }[] = [];
  const reading = (async () => {
    for await (const record of resumingReader(link.records, instant)) {
      if (record.kind === "open") opens.push(readJson<StreamOpen>(record).resume);
      else seen.push({ value: record.payload[0], keyframe: (record.flags & KEYFRAME) !== 0 });
      if (seen.length === 4) break;
    }
  })();
  await settle();
  publish(1);
  publish(2);
  await settle();
  cut();
  for (let value = 3; value <= 8; value++) publish(value);
  await settle();
  publish(9);
  publish(10);
  await reading;
  expect(opens).toEqual(["keyframe", "keyframe"]);
  expect(asked()).toBe(2);
  expect(seen).toEqual([
    { value: 1, keyframe: true },
    { value: 2, keyframe: false },
    { value: 9, keyframe: true },
    { value: 10, keyframe: false },
  ]);
});

test("a reader that can take no more is dropped without holding the others up", () => {
  const { source, publish } = producer();
  const slow: number[] = [];
  const quick: number[] = [];
  source.attach(null, (record) => {
    if (record.kind === "publication") slow.push(record.payload[0]);
    return slow.length < 2;
  });
  source.attach(null, (record) => {
    if (record.kind === "publication") quick.push(record.payload[0]);
    return true;
  });
  for (let value = 1; value <= 5; value++) publish(value);
  expect(slow).toEqual([1, 2]);
  expect(quick).toEqual([1, 2, 3, 4, 5]);
});

test("the stream's last word ends the reader, and a late reader hears it too", async () => {
  const { source, publish } = producer();
  const { link, port } = loopback({ side: "blue", player: "blue-1" });
  port.onReader((after, deliver) => source.attach(after, deliver));
  const kinds = async () => {
    const seen: string[] = [];
    for await (const record of resumingReader(link.records, instant)) seen.push(record.kind);
    return seen;
  };
  const first = kinds();
  await settle();
  publish(1);
  source.end("finished");
  expect(await first).toEqual(["open", "publication", "end"]);
  expect(await kinds()).toEqual(["open", "end"]);
  expect(() => publish(2)).toThrow(/ended/);
});

test("the reader reconnects at once after progress and backs off while nothing arrives", async () => {
  const sleeps: number[] = [];
  let connection = 0;
  const reader = resumingReader(
    async function* () {
      connection += 1;
      if (connection === 1) yield { id: 1, kind: "publication", flags: 0, payload: bytes(1) };
      if (connection <= 4) return;
      if (connection === 5) throw new Error("connection refused");
      yield { id: 0, kind: "end", flags: 0, payload: bytes() };
    },
    { sleep: async (milliseconds) => void sleeps.push(milliseconds) },
  );
  const ids: number[] = [];
  for await (const record of reader) ids.push(record.id);
  expect(ids).toEqual([1, 0]);
  expect(sleeps[0]).toBe(0);
  for (let i = 2; i < sleeps.length; i++) expect(sleeps[i]).toBeGreaterThanOrEqual(sleeps[i - 1]);
  expect(sleeps.at(-1)).toBeGreaterThan(0);
});

test("a stream that skips a record without saying keyframe is broken, not resumed", async () => {
  const reader = resumingReader(async function* () {
    yield jsonRecord("open", { resume: "continued" });
    yield { id: 1, kind: "publication", flags: 0, payload: bytes(1) };
    yield { id: 3, kind: "publication", flags: 0, payload: bytes(3) };
  }, instant);
  const drain = async () => {
    for await (const _ of reader) void _;
  };
  await expect(drain()).rejects.toThrow(/skipped/);
});

test("the loopback carries orders and previews to the port and their replies back", async () => {
  const { link, port } = loopback({ side: "blue", player: "blue-1" });
  await expect(link.up({ seq: 1, items: [] })).rejects.toThrow(/orders/);
  port.onUp(async (batch) => ({ seq: batch.seq, results: [] }));
  port.onPreview(async () => ({ dropped: true }));
  await expect(link.up({ seq: 4, items: [] })).resolves.toEqual({ seq: 4, results: [] });
  await expect(
    link.preview({ kind: "purchase", variant: "tank", destination: [0, 0] }),
  ).resolves.toEqual({ dropped: true });
});
