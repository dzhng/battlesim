// Pointer-following ground paint: the held destination/facing preview and
// the Space range ruler share one feed, refreshed only when their geometry changes.
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import { EMPTY_MESH, concatMeshes, type Mesh } from "@packages/battle-renderer/src/mesh";
import {
  buildDestinationPreview,
  circleExit,
  unitCircle,
  destinationCircle,
  type SurfaceHeight,
  type UnitCircle,
  type DestinationMarker,
} from "@packages/battle-renderer/src/orderOverlay";
import { buildRangeRuler, type RulerLine } from "@packages/battle-renderer/src/rangeRulerOverlay";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import {
  closestUnit,
  rangeRuler,
  type RangeRuler,
  type RulerRules,
} from "@web/battle/present/rangeRuler";
import { inReverseZone } from "@web/battle/input/reverseZone";
import { dragFacing } from "@web/battle/input/useUnitControl";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { SimClient } from "@web/battle/sim/client";
import type { MovePreviewRequest, MoveDestination } from "@web/battle/sim/protocol";
import type { Vec3 } from "math";
import { orderView } from "./battleOverlay";
import { Feed } from "./feed";
import { groundUnderRay, type StaticWorld } from "./useStaticWorld";
import { gameOrderStyle, gameRulerStyle, gameStroke } from "./gameOverlay";

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
    circle: unitCircle(orderView(own, true, 1), gameOrderStyle),
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

  preview: readonly (DestinationMarker & { unit: number })[] = [];
  private previewClient: Pick<SimClient, "previewMove"> | null = null;
  private resolved = "";
  private gesture = "";
  private generation = 0;
  private busy = false;
  private destinations: MoveDestination[] = [];

  /** At most one placement query is in flight; intermediate pointer updates
   *  are coalesced, and a reply cannot restore a cancelled gesture. */
  resolveMove(
    move: MovePreviewRequest | null,
    selected: readonly OwnUnitView[],
    client: Pick<SimClient, "previewMove"> | null,
    tick: number,
  ) {
    const gesture = move && client ? JSON.stringify(move) : "";
    if (gesture !== this.gesture || client !== this.previewClient) {
      this.generation += 1;
      this.gesture = gesture;
      this.previewClient = client;
      this.destinations = [];
      this.resolved = "";
    }
    const key = gesture && JSON.stringify([move, tick]);
    const generation = this.generation;
    if (move && client && key && !this.busy && key !== this.resolved) {
      this.busy = true;
      void client
        .previewMove(move)
        .then(
          (marks) => {
            if (this.generation === generation && this.previewClient === client) {
              this.destinations = marks;
              this.resolved = key;
            }
          },
          () => {
            if (this.generation === generation && this.previewClient === client) {
              this.destinations = [];
              this.resolved = key;
            }
          },
        )
        .finally(() => {
          this.busy = false;
        });
    }
    return this.markers(this.destinations, selected);
  }

  markers(
    destinations: readonly MoveDestination[],
    own: readonly OwnUnitView[],
    opacity: ReadonlyMap<number, number> | null = null,
  ) {
    if (destinations.length === 0) return [];
    const units = new Map(own.map((u) => [u.id, u]));
    return destinations.flatMap((mark) => {
      const u = units.get(mark.unit);
      const alpha = opacity?.get(mark.unit) ?? (opacity ? 0 : 1);
      if (!mark.placed || !u || alpha <= 0) return [];
      const view = orderView(u, true, 1);
      return [
        {
          unit: u.id,
          placed: mark.placed,
          opacity: alpha,
          ...destinationCircle(
            {
              ...view,
              finalFacing: mark.facing,
              area: u.area ? { ...u.area, anchor: mark.goal } : null,
            },
            mark.goal,
            gameOrderStyle,
          ),
        },
      ];
    });
  }

  update(
    shown: ShownRuler | null,
    preview: readonly (DestinationMarker & { unit: number })[],
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
      preview
        .map((p) => [p.unit, ...p.c, p.r, p.facing ?? 0, p.placed, p.opacity, metresPerPx].join())
        .join(";");
    if (key === this.key) return;
    this.key = key;
    this.feed.set(
      concatMeshes([
        shown
          ? buildRangeRuler(
              rulerLine(shown.ruler, shown.circle),
              z,
              gameRulerStyle,
              metresPerPx,
              gameStroke(metresPerPx),
            )
          : EMPTY_MESH,
        preview.length
          ? buildDestinationPreview(preview, z, gameOrderStyle, {
              stroke: gameStroke(metresPerPx),
            })
          : EMPTY_MESH,
      ]),
    );
  }
}

/** The gesture's shared goal and facing; the authority resolves each unit. */
export function movePreviewAt(
  start: WorldRay | null,
  cursor: WorldRay | null,
  world: StaticWorld,
  selected: readonly OwnUnitView[],
  queued: boolean,
): MovePreviewRequest | null {
  if (!start || selected.length === 0) return null;
  const at = groundUnderRay(world.view, start);
  if (!at) return null;
  const to = cursor && groundUnderRay(world.view, cursor);
  const facing = dragFacing({ ground: [at[0], at[1]], facingTo: to && [to[0], to[1]] });
  return {
    units: selected.map((u) => u.id),
    queued,
    route: "shortest",
    goal: [at[0], at[1]],
    ...(facing === undefined ? {} : { facing }),
    direction: inReverseZone(selected, [at[0], at[1]]) ? "reverse" : "forward",
  };
}
