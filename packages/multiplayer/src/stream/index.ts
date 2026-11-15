// The record stream: one framing, one bounded backlog and one resume rule for
// every stream the game server sends. `specs/multiplayer/wire.md` is the
// contract; nothing here knows about HTTP or about battles.
import type { RecordKind, StreamOpen, StreamRecord } from "../wire";

const HEADER_BYTES = 12;
/** Flag bit 0: a record a reader can start from, such as a full snapshot. */
export const KEYFRAME = 1;
const KINDS: readonly RecordKind[] = ["open", "publication", "status", "state", "end"];

const framedBytes = (payloadBytes: number) => HEADER_BYTES + ((payloadBytes + 3) & ~3);

export function encodeRecord(record: StreamRecord): Uint8Array {
  const kind = KINDS.indexOf(record.kind);
  if (kind < 0) throw new Error(`unknown record kind: ${record.kind}`);
  if (!Number.isInteger(record.id) || record.id < 0 || record.id > 0xffffffff)
    throw new RangeError("record id must be a u32");
  if (!Number.isInteger(record.flags) || record.flags < 0 || record.flags > 0xff)
    throw new RangeError("record flags must be a u8");
  const bytes = new Uint8Array(framedBytes(record.payload.byteLength));
  const view = new DataView(bytes.buffer);
  view.setUint32(0, record.payload.byteLength, true);
  view.setUint32(4, record.id, true);
  view.setUint8(8, kind);
  view.setUint8(9, record.flags);
  bytes.set(record.payload, HEADER_BYTES);
  return bytes;
}

/** Turns a response's bytes back into records, however they were chunked. */
export function recordDecoder(): { push(chunk: Uint8Array): StreamRecord[] } {
  let pending = new Uint8Array(0);
  return {
    push(chunk) {
      if (chunk.byteLength) {
        const joined = new Uint8Array(pending.byteLength + chunk.byteLength);
        joined.set(pending);
        joined.set(chunk, pending.byteLength);
        pending = joined;
      }
      const records: StreamRecord[] = [];
      while (pending.byteLength >= HEADER_BYTES) {
        const view = new DataView(pending.buffer, pending.byteOffset, pending.byteLength);
        const payloadBytes = view.getUint32(0, true);
        const total = framedBytes(payloadBytes);
        if (pending.byteLength < total) break;
        const kind = KINDS[view.getUint8(8)];
        if (!kind) throw new Error(`unknown record kind byte: ${view.getUint8(8)}`);
        records.push({
          id: view.getUint32(4, true),
          kind,
          flags: view.getUint8(9),
          payload: pending.slice(HEADER_BYTES, HEADER_BYTES + payloadBytes),
        });
        pending = pending.slice(total);
      }
      return records;
    },
  };
}

/** A record whose payload is JSON. `open` and `end` are unnumbered: id 0. */
export const jsonRecord = (kind: RecordKind, value: unknown, id = 0, flags = 0): StreamRecord => ({
  id,
  kind,
  flags,
  payload: new TextEncoder().encode(JSON.stringify(value)),
});

export const readJson = <T>(record: StreamRecord): T =>
  JSON.parse(new TextDecoder().decode(record.payload)) as T;

/** The newest numbered records, up to a byte budget. Ids count up from 1. */
export function recordRing(limitBytes: number): {
  append(kind: RecordKind, flags: number, payload: Uint8Array): StreamRecord;
  /** The records after `after`, or "gap" when some of them are gone. */
  since(after: number): StreamRecord[] | "gap";
} {
  if (!Number.isFinite(limitBytes) || limitBytes <= 0)
    throw new RangeError("ring limit must be positive");
  const records: StreamRecord[] = [];
  let bytes = 0;
  let nextId = 1;
  return {
    append(kind, flags, payload) {
      // A copy: the caller's buffer is often reused for the next record.
      const record = { id: nextId++, kind, flags, payload: payload.slice() };
      records.push(record);
      bytes += framedBytes(payload.byteLength);
      while (records.length > 1 && bytes > limitBytes)
        bytes -= framedBytes(records.shift()!.payload.byteLength);
      return record;
    },
    since(after) {
      const oldest = records[0]?.id ?? nextId;
      if (after < oldest - 1 || after >= nextId) return "gap";
      return records.filter((record) => record.id > after);
    },
  };
}

export interface RecordSource {
  /** Number a record, keep it for resuming readers and hand it to the live ones. */
  append(kind: RecordKind, flags: number, payload: Uint8Array): number;
  /** A reader opened the stream after `after` (null: from nothing). `deliver`
   *  returns false when that reader can take no more, and it is then dropped.
   *  The result detaches the reader. */
  attach(after: number | null, deliver: (record: StreamRecord) => boolean): () => void;
  /** Say a last word to every reader, now and later, and take no more records. */
  end(reason: string): void;
}

/** One stream's producer end. A reader whose place is still in the ring
 *  continues from it. Any other reader is told `keyframe`, the producer is
 *  asked for one, and that reader hears nothing until a record flagged
 *  [`KEYFRAME`] arrives. The source never waits for a reader. */
export function recordSource(
  limitBytes: number,
  describe: () => Omit<StreamOpen, "resume">,
  requestKeyframe: () => void,
): RecordSource {
  const ring = recordRing(limitBytes);
  type Reader = { deliver: (record: StreamRecord) => boolean; awaitingKeyframe: boolean };
  const readers = new Set<Reader>();
  let last: StreamRecord | null = null;
  return {
    append(kind, flags, payload) {
      if (last) throw new Error("the stream has ended");
      const record = ring.append(kind, flags, payload);
      for (const reader of readers) {
        if (reader.awaitingKeyframe && !(flags & KEYFRAME)) continue;
        reader.awaitingKeyframe = false;
        if (!reader.deliver(record)) readers.delete(reader);
      }
      return record.id;
    },
    attach(after, deliver) {
      const backlog = after === null ? "gap" : ring.since(after);
      const resume = backlog === "gap" && !last ? "keyframe" : "continued";
      if (!deliver(jsonRecord("open", { resume, ...describe() }))) return () => {};
      if (last) {
        deliver(last);
        return () => {};
      }
      if (backlog !== "gap") for (const record of backlog) if (!deliver(record)) return () => {};
      const reader = { deliver, awaitingKeyframe: backlog === "gap" };
      readers.add(reader);
      if (reader.awaitingKeyframe) requestKeyframe();
      return () => void readers.delete(reader);
    },
    end(reason) {
      if (last) return;
      last = jsonRecord("end", { reason });
      for (const reader of readers) reader.deliver(last);
      readers.clear();
    },
  };
}

export interface Clock {
  sleep(milliseconds: number): Promise<void>;
}

/** The server broke the stream's own rule; reconnecting cannot repair that. */
export class StreamBroken extends Error {}

/** How long to wait before reconnecting, by how many connections in a row
 *  delivered nothing. A connection that delivered something is followed at once. */
const RECONNECT_MS = [0, 250, 1000, 4000];

/** One endless stream out of many connections. `open(after)` is one
 *  connection's records; when it ends or fails the reader opens another after
 *  the last record it handed on, so its consumer sees every record once and
 *  in order, until an `end` record. `open` and `end` are unnumbered and pass
 *  through: a `keyframe` open says the numbering starts afresh with what
 *  follows. */
export async function* resumingReader(
  open: (after: number | null) => AsyncIterable<StreamRecord>,
  clock: Clock,
): AsyncGenerator<StreamRecord, void, void> {
  let after: number | null = null;
  let idle = 0;
  for (;;) {
    let progressed = false;
    let afresh = false;
    try {
      for await (const record of open(after)) {
        if (record.kind === "open") {
          afresh = readJson<StreamOpen>(record).resume === "keyframe";
          yield record;
          continue;
        }
        if (record.kind === "end") {
          yield record;
          return;
        }
        if (afresh) afresh = false;
        else if (after !== null) {
          if (record.id <= after) continue;
          if (record.id !== after + 1)
            throw new StreamBroken(`the stream skipped from record ${after} to ${record.id}`);
        }
        after = record.id;
        progressed = true;
        yield record;
      }
    } catch (error) {
      if (error instanceof StreamBroken) throw error;
      // A dropped connection: reconnect after the last record handed on.
    }
    idle = progressed ? 0 : idle + 1;
    await clock.sleep(RECONNECT_MS[Math.min(idle, RECONNECT_MS.length - 1)]);
  }
}
