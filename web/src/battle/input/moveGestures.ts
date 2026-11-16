/** Right-click gestures (contracts.md controls). A right-click that sends
 * units walking (a move on ground; boarding, collecting or entering a building
 * on a host) carries a fresh gesture token. A second right-click within 350 ms
 * and 6 CSS pixels upgrades that same token to the fastest route, wherever its
 * orders now are (applied, active or Shift-queued). It never adds a waypoint
 * and never touches other orders. A right-drag carries a facing (Q9). */
import type { Order } from "../sim/protocol";

type Walk = Extract<Order, { kind: "move" | "board" | "collect" | "occupy_building" }>;
/** A walking order a right-click sends, before its token is minted. */
type WalkClick = Walk extends unknown ? Omit<Walk, "gesture"> : never;

export const DOUBLE_CLICK_MS = 350;
export const DOUBLE_CLICK_PX = 6;

export interface RightClick {
  x: number;
  y: number;
  /** Event time in milliseconds. */
  time: number;
}

export class MoveGestures {
  private next = 1;
  private last: (RightClick & { gesture: number }) | null = null;

  /** A fresh token for any gesture-carrying order (attack-move, fast move),
   *  from the same sequence as right-click moves so tokens never collide. */
  token(): number {
    return this.next++;
  }

  /** The order a right-click sends: `walk` under a fresh token, or the
   *  upgrade of the first click's token when it is the second click. */
  rightClick(click: RightClick, walk: WalkClick): Order {
    const last = this.last;
    // A right-drag (a facing) is its own gesture, never the second click.
    if (
      !("facing" in walk && walk.facing !== undefined) &&
      last &&
      click.time - last.time <= DOUBLE_CLICK_MS &&
      Math.hypot(click.x - last.x, click.y - last.y) <= DOUBLE_CLICK_PX
    ) {
      this.last = null;
      return { kind: "upgrade_move", gesture: last.gesture, route: "fastest" };
    }
    const gesture = this.next++;
    this.last = { ...click, gesture };
    return { ...walk, gesture } as Walk;
  }
}
