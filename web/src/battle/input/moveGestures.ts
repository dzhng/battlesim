/** Right-click move gestures (contracts.md controls). A right-click on ground
 * issues an ordinary move carrying a fresh gesture token. A second right-click
 * within 350 ms and 6 CSS pixels upgrades that same token to the fastest route,
 * wherever its orders now are (applied, active or Shift-queued). It never adds
 * a waypoint and never touches other orders. */
import type { Order } from "../sim/protocol";

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

  /** The order a right-click on `goal` sends for `units`. */
  rightClick(click: RightClick, units: number[], goal: [number, number]): Order {
    const last = this.last;
    if (
      last &&
      click.time - last.time <= DOUBLE_CLICK_MS &&
      Math.hypot(click.x - last.x, click.y - last.y) <= DOUBLE_CLICK_PX
    ) {
      this.last = null;
      return { kind: "upgrade_move", gesture: last.gesture, route: "fastest" };
    }
    const gesture = this.next++;
    this.last = { ...click, gesture };
    return { kind: "move", units, gesture, goal, route: "shortest" };
  }
}
