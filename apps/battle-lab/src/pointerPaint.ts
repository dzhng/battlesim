// Pointer-following ground paint: the held destination/facing preview and
// the Space range ruler share one feed, refreshed only when their geometry changes.
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import { EMPTY_MESH, concatMeshes, type Mesh } from "@packages/battle-renderer/src/mesh";
import {
  buildFacingPreview,
  circleExit,
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
import { dragFacing } from "@web/battle/input/useUnitControl";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { Vec3 } from "math";
import { orderView } from "./battleOverlay";
import { Feed } from "./feed";
import { groundUnderRay, type StaticWorld } from "./useStaticWorld";
import { villageOrderStyle, villageRulerStyle, villageStroke } from "./villageOverlay";

/** The ruler shown, and the circle the orders draw round its unit (with
 *  Space held), which the painted line leaves from. */
interface ShownRuler {
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
  // Space is held: the orders draw every unit's circle, shown in full.
  return {
    ruler: rangeRuler(unit, cursor, rules, UNITS),
    circle: unitCircle(orderView(own, true, 1), villageOrderStyle),
  };
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
    start_m: circleExit(ruler.from, ruler.to, circle),
    reach_m: reaches ? length : Math.max(...ticks),
    ticks,
  };
}

/** Pointer paint is independent of the observation overlay: moving the
 *  cursor must redraw even while the simulation is paused. */
export class PointerPaint {
  readonly feed = new Feed<Mesh>(EMPTY_MESH);
  private key = "";
  /** The ruler last shown, for the lab's probes. */
  shown: RangeRuler | null = null;

  preview: UnitCircle | null = null;

  update(
    shown: ShownRuler | null,
    preview: UnitCircle | null,
    z: SurfaceHeight,
    metresPerPx: number,
  ) {
    this.preview = preview;
    this.shown = shown?.ruler ?? null;
    const rulerKey = shown
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
    const key =
      rulerKey +
      ":" +
      (preview ? [...preview.c, preview.r, preview.facing ?? 0, metresPerPx].join() : "");
    if (key === this.key) return;
    this.key = key;
    this.feed.set(
      concatMeshes([
        shown
          ? buildRangeRuler(
              rulerLine(shown.ruler, shown.circle),
              z,
              villageRulerStyle,
              metresPerPx,
              villageStroke(metresPerPx),
            )
          : EMPTY_MESH,
        preview
          ? buildFacingPreview(preview, z, villageOrderStyle, {
              stroke: villageStroke(metresPerPx),
            })
          : EMPTY_MESH,
      ]),
    );
  }
}

/** One shared destination arrow commands the selected group. */
export function facingPreviewAt(
  start: WorldRay | null,
  cursor: WorldRay | null,
  world: StaticWorld,
  selected: readonly OwnUnitView[],
): UnitCircle | null {
  if (!start || selected.length === 0) return null;
  const at = groundUnderRay(world.view, start);
  if (!at) return null;
  const to = cursor && groundUnderRay(world.view, cursor);
  const fallback =
    dragFacing({
      ground: [selected[0].position[0], selected[0].position[1]],
      facingTo: [at[0], at[1]],
    }) ?? selected[0].yaw;
  const facing = dragFacing({ ground: [at[0], at[1]], facingTo: to && [to[0], to[1]] }) ?? fallback;
  const radii = selected.map((u) => unitCircle(orderView(u, true, 1), villageOrderStyle)?.r ?? 0);
  return { c: [at[0], at[1]], r: Math.max(...radii), facing };
}
