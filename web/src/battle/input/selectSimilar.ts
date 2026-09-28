/** Select similar (the user's call): double-click an own unit to select
 *  every own unit of its type; a second double-click, or Ctrl +
 *  double-click, widens to every own unit sharing its role (the type's first
 *  role, the one its symbol shows). A double-click is two left clicks on the
 *  same unit within the right-click gesture's window and slop; the second
 *  double-click must start within that window too, and before the selection
 *  changes any other way. */
import type { OwnUnitView } from "../sim/observation";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { DOUBLE_CLICK_MS, DOUBLE_CLICK_PX } from "./moveGestures";

export type SimilarBy = "type" | "role";

export interface LeftClick {
  /** The own unit clicked, or null for anything else. */
  unit: number | null;
  /** Its type id. */
  kind: string | null;
  ctrl: boolean;
  x: number;
  y: number;
  /** Event time in milliseconds. */
  time: number;
}

export class SelectClicks {
  private last: LeftClick | null = null;
  /** The type the last double-click selected by, how widely, when, and the
   *  selection it made (null until the caller reports it). */
  private widened: { kind: string; by: SimilarBy; at: number; selection: string | null } | null =
    null;

  /** What a left click asks for: null for an ordinary click, or the
   *  similar units to select by type or role. */
  click(c: LeftClick): SimilarBy | null {
    const last = this.last;
    const double =
      c.unit !== null &&
      last?.unit === c.unit &&
      c.time - last.time <= DOUBLE_CLICK_MS &&
      Math.hypot(c.x - last.x, c.y - last.y) <= DOUBLE_CLICK_PX;
    if (!double) {
      // Clicking a unit of another type (or the ground), or past the
      // window, starts over.
      const w = this.widened;
      if (c.kind !== w?.kind || c.time - w.at > DOUBLE_CLICK_MS) this.widened = null;
      this.last = c;
      return null;
    }
    // A double-click again on the type just selected widens to its role.
    const by: SimilarBy = c.ctrl || this.widened?.kind === c.kind ? "role" : "type";
    this.widened = { kind: c.kind!, by, at: c.time, selection: null };
    // The next click starts a new double-click, never a third click.
    this.last = null;
    return by;
  }

  /** Report the selection whenever it changes: the first change after a
   *  double-click is its own, any other forgets the widening. */
  selectionChanged(selection: readonly number[]) {
    const w = this.widened;
    if (!w) return;
    const key = selection.join(",");
    if (w.selection === null) w.selection = key;
    else if (w.selection !== key) this.widened = null;
  }

  /** A selection made any other way (a box, a reset) starts over. */
  reset() {
    this.last = null;
    this.widened = null;
  }
}

/** Own units like `kind`: of the same type, or sharing its first role. */
export function similarUnits(
  own: readonly Pick<OwnUnitView, "id" | "kind">[],
  kind: string,
  by: SimilarBy,
  units: UnitCatalog,
): number[] {
  if (by === "type") return own.filter((u) => u.kind === kind).map((u) => u.id);
  const role = units.type(kind).roles[0];
  return own.filter((u) => units.type(u.kind).roles.includes(role)).map((u) => u.id);
}
