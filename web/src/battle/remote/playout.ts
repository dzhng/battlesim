// Evens out a server battle's records before the page plays them. A reliable
// stream never loses a record: it stalls and then delivers a burst, and a
// page that timed motion by arrival would lurch. Playout holds publications
// a little behind arrival and releases them a tick apart. It never drops one,
// because the page's reader needs every delta, and it never invents one.
//
// The delay is what an order costs on top of the network, so playout keeps no
// more of it than it was asked for. A stall longer than the buffer leaves the
// page behind by the stall as well: once a second of ticks has all waited
// longer than the delay, the extra is played off a little fast. A link that
// delivers in bursts is left alone, because the first tick of each burst
// still waits only the delay. A long backlog is fast-forwarded.
import type { SimReply } from "../sim/protocol";

export interface Playout {
  /** A reply arrived from the stream at `now` (milliseconds, any monotonic clock). */
  push(reply: SimReply, now: number): void;
  /** The replies to hand on at `now`, in stream order. */
  due(now: number): SimReply[];
  /** When the next publication is due, for a caller to wake at; null while
   *  none is waiting. */
  nextAt(): number | null;
  /** How much battle time is waiting. A caller that finds this past what it
   *  will fast-forward through reopens its stream for a snapshot and calls
   *  `reset`, instead of asking for what is `due`. */
  queuedMs(): number;
  /** The stream was reopened: forget what is waiting and start afresh. */
  reset(): void;
}

/** How much of a tick the playhead takes while it plays off extra delay: a
 *  fifth faster, quick enough to be back within a second and too little to
 *  read as a jump. */
const CATCH_UP = 0.8;

export function playout(tickHz: number, delayMs: number): Playout {
  if (!Number.isFinite(tickHz) || tickHz <= 0) throw new RangeError("tickHz must be positive");
  if (!Number.isFinite(delayMs) || delayMs < 0)
    throw new RangeError("delayMs must be non-negative");
  const tickMs = 1000 / tickHz;
  /** Publications held when arrival is steady. */
  const steady = Math.ceil(delayMs / tickMs);
  /** Past this many, the page has fallen behind and fast-forwards to `steady`. */
  const behind = 2 * steady + 1;

  /** Each reply with when it arrived. */
  const queue: { reply: SimReply; at: number }[] = [];
  let publications = 0;
  /** How long each of the last second's publications waited. */
  const waits: number[] = [];
  const second = Math.ceil(tickHz);
  /** Every one of them waited over a tick longer than the delay, and none
   *  since has waited only the delay: the playhead is playing the extra off. */
  let hurrying = false;
  const settle = () => {
    waits.length = 0;
    hurrying = false;
  };
  /** When the next publication is due; null while none is waiting, so the
   *  next to arrive is held for the full delay again. */
  let nextAt: number | null = null;

  return {
    push(reply, now) {
      queue.push({ reply, at: now });
      if (reply.type !== "publication") return;
      publications += 1;
      nextAt ??= now + delayMs;
    },

    due(now) {
      const out: SimReply[] = [];
      let fastForward = publications > behind;
      while (queue.length > 0) {
        if (queue[0].reply.type === "publication") {
          if (fastForward) {
            // Back to back, then on the even clock again from here.
            nextAt = now + tickMs;
            fastForward = publications - 1 > steady;
            settle();
          } else if (nextAt !== null && now >= nextAt) {
            // Arriving late only shortens a wait, so the shortest of a second
            // is the delay the page is carrying, clear of jitter and bursts.
            const waited = nextAt - queue[0].at;
            if (waits.push(waited) > second) waits.shift();
            if (waited <= delayMs) hurrying = false;
            else if (waits.length === second && Math.min(...waits) > delayMs + tickMs)
              hurrying = true;
            nextAt += hurrying ? tickMs * CATCH_UP : tickMs;
          } else break;
          publications -= 1;
        }
        out.push(queue.shift()!.reply);
      }
      if (publications === 0) {
        nextAt = null;
        settle();
      }
      return out;
    },

    nextAt: () => nextAt,

    queuedMs: () => publications * tickMs,

    reset() {
      queue.length = 0;
      publications = 0;
      nextAt = null;
      settle();
    },
  };
}
