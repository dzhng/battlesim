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
  append(kind: RecordKind, flags: number, payload: Uint8Array): StreamRecord;
  since(after: number): StreamRecord[] | "gap";
};
// One stream's producer end: the ring, the live readers and the resume rule.
function recordSource(
  limitBytes: number,
  describe: () => Omit<StreamOpen, "resume">,   // what `open` says besides the mode
  requestKeyframe: () => void,                   // a reader needs one
): {
  append(kind: RecordKind, flags: number, payload: Uint8Array): number;
  attach(after: number | null, deliver: (record: StreamRecord) => boolean): () => void;
  end(reason: string): void;
};
// One endless stream out of many connections.
function resumingReader(
  open: (after: number | null) => AsyncIterable<StreamRecord>, clock: Clock,
): AsyncGenerator<StreamRecord>;

// link.ts: what a transport implements
interface Seat { side: "blue" | "red"; player: string }
interface SeatLink {            // the page's end
  records(after: number | null): AsyncIterable<StreamRecord>;   // one connection
  up(batch: UpBatch): Promise<UpReply>;
  preview(query: PreviewQuery): Promise<PreviewAnswer>;
  close(): void;
}
interface SeatPort {            // the server's end
  seat: Seat;                   // the transport's word, never a request's
  onReader(handle: (after: number | null,
                    deliver: (record: StreamRecord) => boolean) => () => void): void;
  onUp(handle: (batch: UpBatch) => Promise<UpReply>): void;
  onPreview(handle: (query: PreviewQuery) => Promise<PreviewAnswer>): void;
}
function loopback(seat: Seat): { link: SeatLink; port: SeatPort; cut(): void };
```

`loopback` joins a link to a port in memory, and `cut` drops its open
connections as a network would. Tests use it, and it is the model a
WebSocket transport would follow later.

The resume rule has one owner, `recordSource`. A transport carries `after`
to the port and records back; it keeps no history of its own. A reader's
stream opening and closing is what `onReader` and its returned function
report, so nothing else needs to be told that a reader has gone.

## Run or see

`bun run --cwd web test multiplayer/stream`.

## Verification

No network and no real timers:

- Every way of splitting a byte sequence into chunks decodes to the same
  records.
- A reader cut after any record resumes with no record lost and none
  repeated.
- An id the ring has dropped yields `"gap"`.
- A reader that reconnects after a gap sees `open` with `keyframe`, and then
  nothing until the next keyframe.
- A reader that can take no more is dropped without holding the others up.
- An `end` record stops the reader, and a reader that opens later hears it.
- The reader reconnects at once after progress and backs off while nothing
  arrives.
- A loopback round trip carries a batch up and its reply back.

## Must stay green

`serverGraph.test.ts` from slice 04, now also covering this package.

## Delegated

The ring's size (start near ten seconds of one side's records, about 600 KB);
the reader's reconnect delay.
