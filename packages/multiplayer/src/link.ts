// A seat's transport boundary. The battle runtime speaks SeatPort, and the
// page speaks SeatLink. HTTP joins them; loopback joins them in tests.
import type { PreviewAnswer, PreviewQuery, StreamRecord, UpBatch, UpReply } from "./wire";

export interface BattleSeat {
  side: "blue" | "red";
  player: string;
}

/** The page's end of one seat. */
export interface SeatLink {
  /** One connection's records, after `after` (null: from nothing). It ends
   *  when the connection does; `resumingReader` opens the next. Aborting
   *  discards unread records and detaches just this reader. */
  records(after: number | null, signal?: AbortSignal): AsyncIterable<StreamRecord>;
  up(batch: UpBatch): Promise<UpReply>;
  preview(query: PreviewQuery): Promise<PreviewAnswer>;
  close(): void;
}

/** The server's end of one seat. The seat is the transport's word, taken from
 *  who connected, never from what a request says. */
export interface SeatPort {
  seat: BattleSeat;
  /** A reader opened the seat's stream. `deliver` returns false when the
   *  reader can take no more. The handler's result is called when that
   *  reader goes away. */
  onReader(
    handle: (after: number | null, deliver: (record: StreamRecord) => boolean) => () => void,
  ): void;
  onUp(handle: (batch: UpBatch) => Promise<UpReply>): void;
  onPreview(handle: (query: PreviewQuery) => Promise<PreviewAnswer>): void;
}

/** A link and a port joined in memory. `cut` drops every open connection the
 *  way a network would, leaving the link free to open another. */
export function loopback(seat: BattleSeat): { link: SeatLink; port: SeatPort; cut(): void } {
  let reader: Parameters<SeatPort["onReader"]>[0] | undefined;
  let up: ((batch: UpBatch) => Promise<UpReply>) | undefined;
  let preview: ((query: PreviewQuery) => Promise<PreviewAnswer>) | undefined;
  const connections = new Set<() => void>();
  let closed = false;

  async function* connection(
    after: number | null,
    signal?: AbortSignal,
  ): AsyncGenerator<StreamRecord, void, void> {
    if (closed || signal?.aborted || !reader) return;
    const queue: StreamRecord[] = [];
    let over = false;
    let wake: (() => void) | undefined;
    const drop = () => {
      over = true;
      wake?.();
    };
    connections.add(drop);
    signal?.addEventListener("abort", drop, { once: true });
    const gone = reader(after, (record) => {
      if (over) return false;
      queue.push(record);
      wake?.();
      return true;
    });
    try {
      for (;;) {
        while (queue.length && !signal?.aborted) yield queue.shift()!;
        if (over) return;
        await new Promise<void>((resolve) => (wake = resolve));
        wake = undefined;
      }
    } finally {
      over = true;
      connections.delete(drop);
      signal?.removeEventListener("abort", drop);
      gone();
    }
  }

  const cut = () => {
    for (const drop of connections) drop();
  };
  const unanswered = (what: string) => Promise.reject(new Error(`no handler for ${what}`));
  return {
    cut,
    link: {
      records: connection,
      up: (batch) => (up ? up(batch) : unanswered("orders")),
      preview: (query) => (preview ? preview(query) : unanswered("previews")),
      close() {
        closed = true;
        cut();
      },
    },
    port: {
      seat,
      onReader: (handle) => void (reader = handle),
      onUp: (handle) => void (up = handle),
      onPreview: (handle) => void (preview = handle),
    },
  };
}
