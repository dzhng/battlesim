import type { Seat, SeatPort } from "../../../../packages/multiplayer/src/link.ts";
import type { UpReply } from "../../../../packages/multiplayer/src/wire.ts";
import type { BattleHost } from "../battleHost/index.ts";

/** Five evenly spaced previews a second keep cursor work out of the order path. */
const PREVIEW_INTERVAL_MS = 200;
/** Convert the simulation's per-second allowance to the transport's monotonic clock. */
const ORDER_WINDOW_MS = 1000;

export class HttpRefusal extends Error {
  constructor(
    readonly status: number,
    readonly reason: string,
  ) {
    super(reason);
  }
}
export type Handlers = {
  reader: Parameters<SeatPort["onReader"]>[0];
  up: Parameters<SeatPort["onUp"]>[0];
  preview: Parameters<SeatPort["onPreview"]>[0];
};

/** One cache and allowance per seat occupant, shared by all their sockets.
 * `seatOf` is whoever knows which browser sits where: it is asked on every
 * request, so a seat may change hands while the battle runs. A credential
 * is never copied into a reply. */
export function battleRoute(
  host: Pick<BattleHost, "attach">,
  seatOf: (browserId: string) => Seat | undefined,
  ordersPerSecond: number,
  now: () => number = () => performance.now(),
) {
  const ports = new Map<string, { who: string; handlers: Handlers }>();
  return {
    /** Absence preserves a returning occupant; a different occupant retires their transport. */
    seating(entries: [string, Seat][]) {
      for (const [who, seat] of entries) {
        const key = `${seat.side}/${seat.player}`;
        const previous = ports.get(key);
        if (previous && previous.who !== who) ports.delete(key);
      }
    },
    seat(browserId: string): Handlers | undefined {
      const seat = seatOf(browserId);
      if (!seat) return;
      const key = `${seat.side}/${seat.player}`;
      const existing = ports.get(key);
      if (existing?.who === browserId) return existing.handlers;
      const handlers = limited(seatHandlers(host, seat), ordersPerSecond, now);
      ports.set(key, { who: browserId, handlers });
      return handlers;
    },
  };
}
export type BattleRoute = ReturnType<typeof battleRoute>;

function limited(port: Handlers, ordersPerSecond: number, now: () => number): Handlers {
  let last: { seq: number; reply: Promise<UpReply> } | null = null;
  let pending = false;
  let tokens = ordersPerSecond;
  let replenished = now();
  let nextPreview = -Infinity;
  return {
    reader: port.reader,
    up(batch) {
      if (batch.items.length > ordersPerSecond)
        return Promise.resolve({ seq: batch.seq, refused: "invalid" });
      if (last?.seq === batch.seq) return last.reply;
      if ((last && batch.seq < last.seq) || pending) throw new HttpRefusal(409, "sequence");
      const at = now();
      tokens = Math.min(
        ordersPerSecond,
        tokens + ((at - replenished) * ordersPerSecond) / ORDER_WINDOW_MS,
      );
      replenished = at;
      pending = true;
      const reply: Promise<UpReply> =
        batch.items.length > tokens
          ? Promise.resolve({ seq: batch.seq, refused: "rate" })
          : ((tokens -= batch.items.length), Promise.resolve().then(() => port.up(batch)));
      last = { seq: batch.seq, reply };
      void reply
        .finally(() => {
          pending = false;
        })
        .catch(() => {});
      return reply;
    },
    preview(query) {
      const at = now();
      if (at < nextPreview) return Promise.resolve({ dropped: true });
      nextPreview = at + PREVIEW_INTERVAL_MS;
      return port.preview(query);
    },
  };
}

/** A raw seat port; browser allowances wrap it in battleRoute. */
export function seatHandlers(host: Pick<BattleHost, "attach">, seat: Seat): Handlers {
  let reader!: Handlers["reader"];
  let up!: Handlers["up"];
  let preview!: Handlers["preview"];
  host.attach({
    seat,
    onReader: (h) => {
      reader = h;
    },
    onUp: (h) => {
      up = h;
    },
    onPreview: (h) => {
      preview = h;
    },
  });
  return { reader, up, preview };
}
