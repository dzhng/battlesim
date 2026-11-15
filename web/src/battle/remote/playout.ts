// Evens out a server battle's records before the page plays them. A reliable
// stream never loses a record: it stalls and then delivers a burst, and a
// page that timed motion by arrival would lurch. Playout holds publications
// a little behind arrival and releases them a tick apart. It never drops one,
// because the page's reader needs every delta, and it never invents one.
import type { SimReply } from "../sim/protocol";

export interface Playout {
  /** A reply arrived from the stream at `now` (milliseconds, any monotonic clock). */
  push(reply: SimReply, now: number): void;
  /** The replies to hand on at `now`, in stream order. */
  due(now: number): SimReply[];
  /** How much battle time is waiting. A caller that finds this past what it
   *  will fast-forward through reopens its stream for a snapshot and calls
   *  `reset`, instead of asking for what is `due`. */
  queuedMs(): number;
  /** The stream was reopened: forget what is waiting and start afresh. */
  reset(): void;
}

export function playout(tickHz: number, delayMs: number): Playout {
  if (!Number.isFinite(tickHz) || tickHz <= 0) throw new RangeError("tickHz must be positive");
  if (!Number.isFinite(delayMs) || delayMs < 0)
    throw new RangeError("delayMs must be non-negative");
  const tickMs = 1000 / tickHz;
  /** Publications held when arrival is steady. */
  const steady = Math.ceil(delayMs / tickMs);
  /** Past this many, the page has fallen behind and fast-forwards to `steady`. */
  const behind = 2 * steady + 1;

  const queue: SimReply[] = [];
  let publications = 0;
  /** When the next publication is due; null while none is waiting, so the
   *  next to arrive is held for the full delay again. */
  let nextAt: number | null = null;

  return {
    push(reply, now) {
      queue.push(reply);
      if (reply.type !== "publication") return;
      publications += 1;
      nextAt ??= now + delayMs;
    },

    due(now) {
      const out: SimReply[] = [];
      let fastForward = publications > behind;
      while (queue.length > 0) {
        if (queue[0].type === "publication") {
          if (fastForward) {
            // Back to back, then on the even clock again from here.
            nextAt = now + tickMs;
            fastForward = publications - 1 > steady;
          } else if (nextAt !== null && now >= nextAt) {
            nextAt += tickMs;
          } else break;
          publications -= 1;
        }
        out.push(queue.shift()!);
      }
      if (publications === 0) nextAt = null;
      return out;
    },

    queuedMs: () => publications * tickMs,

    reset() {
      queue.length = 0;
      publications = 0;
      nextAt = null;
    },
  };
}
