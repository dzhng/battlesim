// The range ruler's feed (Space held with a selection): each frame, the
// ground under the pointer, the selected unit nearest it (at its drawn
// position) and the ruler from it (`web/src/battle/present/rangeRuler.ts`),
// then the ground paint for it, rebuilt only when what it draws moved.
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import type { Mesh } from "@packages/battle-renderer/src/mesh";
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
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
import { Feed } from "./feed";
import { groundUnderRay, type StaticWorld } from "./useStaticWorld";
import { villageRulerStyle } from "./villageOverlay";

const NO_MARKS: Mesh = new Float32Array(0);

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
): RangeRuler | null {
  if (!ray || selected.length === 0) return null;
  const hit = groundUnderRay(world.view, ray);
  if (!hit) return null;
  const cursor = [hit[0], hit[1], z(hit[0], hit[1])] as const;
  const drawn = selected.map((u) => {
    const p = drawnAt.get(u.id) ?? u.position;
    return { id: u.id, kind: u.kind, position: [p[0], p[1], p[2]] as const };
  });
  const unit = closestUnit(drawn, [cursor[0], cursor[1]]);
  return unit && rangeRuler(unit, cursor, rules, UNITS);
}

/** What the ruler paints: lit up to the farthest reach short of the cursor
 *  (all of it when a weapon reaches the cursor or the unit has none), a
 *  tick where each reach ends on the line. */
export function rulerLine(ruler: RangeRuler): RulerLine {
  const length = Math.hypot(ruler.to[0] - ruler.from[0], ruler.to[1] - ruler.from[1]);
  const ticks = ruler.marks.flatMap((m) => (m.along_m === null ? [] : [m.along_m]));
  const reaches = ruler.marks.length === 0 || ruler.marks.some((m) => m.inRange);
  return {
    from: [ruler.from[0], ruler.from[1]],
    to: [ruler.to[0], ruler.to[1]],
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

  update(ruler: RangeRuler | null, z: SurfaceHeight, metresPerPx: number) {
    this.shown = ruler;
    const key = ruler
      ? [ruler.unit, ...ruler.from, ...ruler.to, metresPerPx].map((v) => v.toFixed(3)).join()
      : "";
    if (key === this.key) return;
    this.key = key;
    this.feed.set(
      ruler ? buildRangeRuler(rulerLine(ruler), z, villageRulerStyle, metresPerPx) : NO_MARKS,
    );
  }
}
