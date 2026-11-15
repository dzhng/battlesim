# 05 Record stream and seat link

**Stage:** Foundation. **Needs:** nothing. **Carries:** decisions 9, 10, P4.

## Contract

A resumable, ordered stream of records, independent of HTTP and of battles,
and the seam a transport plugs into. The framing and the resume rule are in
[wire.md](../wire.md).

## Seam

New package `packages/multiplayer/src/`, relative imports only, no DOM and no
Node:

```ts
// wire.ts: the types in wire.md
// stream/
function encodeRecord(r: StreamRecord): Uint8Array;
function recordDecoder(): { push(chunk: Uint8Array): StreamRecord[] };
function recordRing(limitBytes: number): {
  append(kind: RecordKind, flags: number, payload: Uint8Array): number;   // returns the id
  since(after: number | null): StreamRecord[] | "gap";
};
function resumingReader(
  open: (after: number | null) => AsyncIterable<Uint8Array>, clock: Clock,
): AsyncIterable<StreamRecord>;                // reconnects, never repeats or skips an id

// link.ts: what a transport implements
interface SeatLink {            // the page's end
  records(after: number | null): AsyncIterable<StreamRecord>;
  up(batch: UpBatch): Promise<UpReply>;
  preview(query: PreviewQuery): Promise<PreviewAnswer>;
  close(): void;
}
interface SeatPort {            // the server's end
  side: "blue" | "red"; player: string;
  deliver(record: StreamRecord): boolean;      // false: the reader is behind
  onUp(handle: (batch: UpBatch) => Promise<UpReply>): void;
  onPreview(handle: (query: PreviewQuery) => Promise<PreviewAnswer>): void;
  onGone(handle: () => void): void;
}
function loopback(): { link: SeatLink; port: SeatPort };
```

`loopback` joins a link to a port in memory. Tests use it, and it is the
model a WebSocket transport would follow later.

## Run or see

`bun run --cwd web test multiplayer/stream`.

## Verification

No network and no real timers:

- Every way of splitting a byte sequence into chunks decodes to the same
  records.
- A reader cut after any record resumes with no record lost and none
  repeated.
- An id the ring has dropped yields `"gap"`.
- A reader that reconnects after a gap sees `open` with `keyframe`.
- A loopback round trip carries a batch up and its reply back.

## Must stay green

`serverGraph.test.ts` from slice 04, now also covering this package.

## Delegated

The ring's size (start near ten seconds of one side's records, about 600 KB);
the reader's reconnect delay.
