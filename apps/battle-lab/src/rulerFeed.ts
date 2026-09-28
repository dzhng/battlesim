// The range ruler's feed (Space held with a selection): each frame, the
// ground under the pointer, the selected unit nearest it (at its drawn
// position) and the ruler from it (`web/src/battle/present/rangeRuler.ts`),
// with the circle the orders draw round that unit, then the ground paint for
// it, rebuilt only when what it draws moved.
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import {
  circleReach,
  unitCircle,
  type SurfaceHeight,
  type UnitCircle,
} from "@packages/battle-renderer/src/orderOverlay";
import { buildRangeRuler, type RulerLine } from "@packages/battle-renderer/src/rangeRulerOverlay";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import {
  closestUnit,
  rangeRuler,
  type RangeRuler,
  type RulerRules,
} from "@web/battle/present/rangeRuler";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Vec3 } from "math";
import { orderView } from "./battleOverlay";
import { Feed } from "./feed";
import { groundUnderRay, type StaticWorld } from "./useStaticWorld";
import { villageOrderStyle, villageRulerStyle } from "./villageOverlay";

const NO_MARKS: Mesh = new Float32Array(0);

/** The ruler shown, and the circle the orders draw round its unit (with
 *  Space held), which the painted line leaves from. */
export interface ShownRuler {
  ruler: RangeRuler;
  circle: UnitCircle | null;
}

/** The ruler from the selected unit nearest the ground under `ray`, or null
 *  with no selection or no ground there. The cursor's point is the walkable
 *  surface where the ray lands. */
export function rulerAt(
  ray: WorldRay | null,
  world: StaticWorld,
  selected: readonly OwnUnitView[],
  drawnAt: ReadonlyMap<number, Readonly<Vec3>>,
  rules: RulerRules,
  z: SurfaceHeight,
): ShownRuler | null {
  if (!ray || selected.length === 0) return null;
  const hit = groundUnderRay(world.view, ray);
  if (!hit) return null;
  const cursor = [hit[0], hit[1], z(hit[0], hit[1])] as const;
  const drawn = selected.map((u) => {
    const p = drawnAt.get(u.id) ?? u.position;
    return { id: u.id, kind: u.kind, position: [p[0], p[1], p[2]] as const };
  });
  const unit = closestUnit(drawn, [cursor[0], cursor[1]]);
  const own = unit && selected.find((u) => u.id === unit.id);
  if (!unit || !own) return null;
  // Space is held: the orders draw every unit's circle (`all`).
  return {
    ruler: rangeRuler(unit, cursor, rules, UNITS),
    circle: unitCircle(orderView(own, true), villageOrderStyle, true),
  };
}

/** How far along the ground from `from` toward `to` the line leaves
 *  `circle` (clearing its arrowhead where it leaves along the facing): 0
 *  with no circle, or from outside one it doesn't cross. With the cursor
 *  inside the circle it is past the cursor: nothing to draw. */
function leaves(from: readonly number[], to: readonly number[], circle: UnitCircle | null) {
  if (!circle) return 0;
  const [dx, dy] = [to[0] - from[0], to[1] - from[1]];
  const length = Math.hypot(dx, dy);
  if (length < 1e-9) return 0;
  const [ux, uy] = [dx / length, dy / length];
  const r = circleReach(circle, Math.atan2(uy, ux));
  // |from + t·u − c|² = r², the larger root.
  const [fx, fy] = [from[0] - circle.c[0], from[1] - circle.c[1]];
  const b = fx * ux + fy * uy;
  const disc = b * b - (fx * fx + fy * fy - r * r);
  return disc > 0 ? Math.max(0, -b + Math.sqrt(disc)) : 0;
}

/** What the ruler paints: lit up to the farthest reach short of the cursor
 *  (all of it when a weapon reaches the cursor or the unit has none), a
 *  tick where each reach ends on the line. */
export function rulerLine(ruler: RangeRuler, circle: UnitCircle | null): RulerLine {
  const length = Math.hypot(ruler.to[0] - ruler.from[0], ruler.to[1] - ruler.from[1]);
  const ticks = ruler.marks.flatMap((m) => (m.along_m === null ? [] : [m.along_m]));
  const reaches = ruler.marks.length === 0 || ruler.marks.some((m) => m.inRange);
  return {
    from: [ruler.from[0], ruler.from[1]],
    to: [ruler.to[0], ruler.to[1]],
    start_m: leaves(ruler.from, ruler.to, circle),
    reach_m: reaches ? length : Math.max(...ticks),
    ticks,
  };
}

/** The ruler's paint for the viewport, rebuilt only when the ruler or the
 *  line scale changes (to a centimetre). */
export class RulerPaint {
  readonly feed = new Feed<Mesh>(NO_MARKS);
  private key = "";
  /** The ruler last shown, for the lab's probes. */
  shown: RangeRuler | null = null;

  update(shown: ShownRuler | null, z: SurfaceHeight, metresPerPx: number) {
    this.shown = shown?.ruler ?? null;
    const key = shown
      ? [
          shown.ruler.unit,
          ...shown.ruler.from,
          ...shown.ruler.to,
          ...(shown.circle ? [...shown.circle.c, shown.circle.r, shown.circle.facing ?? 0] : []),
          metresPerPx,
        ]
          .map((v) => v.toFixed(3))
          .join()
      : "";
    if (key === this.key) return;
    this.key = key;
    this.feed.set(
      shown
        ? buildRangeRuler(rulerLine(shown.ruler, shown.circle), z, villageRulerStyle, metresPerPx)
        : NO_MARKS,
    );
  }
}
