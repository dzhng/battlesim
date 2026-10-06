/** Which own units' order marks show, and how strongly: the one owner.
 *  Holding Space shows every own unit's; an order just issued (queued or
 *  not) shows its units' in full for `hold_s`, then fades them out over
 *  `fade_s`, a confirmation; a unit whose route turns out blocked flashes
 *  the same way when that is found (a route takes a while to plan, so the
 *  order's own flash may be over); nothing else does. A selection alone shows
 *  only the selection's own markers, which are not order marks. Either
 *  way the marks are the one Space view (`buildOrderOverlay`), drawn at the
 *  opacity given here. Time is the presentation clock, so a paused battle
 *  holds its flash and a held clock gives a still capture. */
import type { Order } from "../sim/protocol";

/** `presentation.overlay.orders.flash`: how long an order's marks show after
 *  it is issued, then how long they take to fade out, in seconds. */
export interface OrderFlash {
  hold_s: number;
  fade_s: number;
}

export function validateOrderFlash(flash: OrderFlash): OrderFlash {
  if (!(flash?.hold_s >= 0 && flash?.fade_s >= 0))
    throw new Error("presentation.overlay.orders.flash: hold_s ≥ 0 and fade_s ≥ 0 (seconds)");
  return flash;
}

/** Opacities are rounded to this many steps, so a fade rebuilds the overlay
 *  at most this often. */
const STEPS = 16;

/** Each shown unit's order-mark opacity in (0, 1]; a unit absent is hidden. */
export type RevealedOrders = ReadonlyMap<number, number>;

/** No unit's order marks shown. */
export const NOTHING_REVEALED: RevealedOrders = new Map();

/** The units whose order marks an order flashes: those it sends somewhere or
 *  sets on something. A fire-policy or deployment toggle changes no order
 *  mark; its answer is the unit's panel. A fast-route upgrade names no
 *  units: it follows the move it upgrades, whose flash is still running. */
function flashedUnits(order: Order): readonly number[] {
  switch (order.kind) {
    case "set_engagement":
    case "set_deployment":
    case "ready":
    case "confirm_purchase":
    case "cancel_pending":
    case "upgrade_move":
      return [];
    default:
      return order.units;
  }
}

export class OrderReveal {
  /** Each flashed unit's latest order, at its presentation time. */
  private readonly issued = new Map<number, number>();
  /** The units whose route was blocked when last looked at. */
  private blocked = new Set<number>();

  constructor(private readonly flash: OrderFlash) {}

  /** `order` was issued at presentation time `at`. */
  noteOrder(order: Order, at: number) {
    for (const id of flashedUnits(order)) this.issued.set(id, at);
  }

  /** A new battle: no flash carries over. */
  clear() {
    this.issued.clear();
    this.blocked.clear();
  }

  /** The order marks shown at presentation time `now` among `own`: all in
   *  full with Space held (`showOrders`), otherwise each flashed unit's
   *  flash. A unit seen here with its route newly blocked starts one. */
  at(
    now: number,
    showOrders: boolean,
    own: readonly { id: number; state?: string }[],
  ): RevealedOrders {
    const blocked = new Set<number>();
    for (const { id, state } of own) {
      if (state !== "route_blocked") continue;
      blocked.add(id);
      if (!this.blocked.has(id)) this.issued.set(id, now);
    }
    this.blocked = blocked;
    if (!showOrders && this.issued.size === 0) return NOTHING_REVEALED;
    const shown = new Map<number, number>();
    const { hold_s, fade_s } = this.flash;
    for (const { id } of own) {
      const issued = this.issued.get(id);
      const since = issued === undefined ? Infinity : now - issued;
      const flash =
        since <= hold_s ? 1 : fade_s > 0 ? Math.max(0, 1 - (since - hold_s) / fade_s) : 0;
      const opacity = showOrders ? 1 : Math.round(flash * STEPS) / STEPS;
      if (opacity > 0) shown.set(id, opacity);
    }
    for (const [id, issued] of this.issued)
      if (now - issued > hold_s + fade_s) this.issued.delete(id);
    return shown;
  }
}

/** Whether two reveals show the same units at the same opacities. */
export function sameReveal(a: RevealedOrders, b: RevealedOrders): boolean {
  if (a.size !== b.size) return false;
  for (const [id, opacity] of a) if (b.get(id) !== opacity) return false;
  return true;
}
