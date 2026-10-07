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
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import {
  closestUnit,
  rangeRuler,
  type RangeRuler,
  type RulerRules,
} from "@web/battle/present/rangeRuler";
import type { PointerIntent } from "@web/battle/input/pointerIntent";
import type { CursorAction } from "@web/battle/present/gameCursor";
import type { OwnUnitView } from "@web/battle/sim/observation";
import type { SimClient } from "@web/battle/sim/client";
import type {
  MovePreviewRequest,
  MoveDestination,
  BuildingPreviewRequest,
  BuildingPlacement,
  Order,
  CommandAck,
  SideName,
} from "@web/battle/sim/protocol";
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

export type PointerPreview =
  | { kind: "move"; request: MovePreviewRequest }
  | { kind: "building"; request: BuildingPreviewRequest };

/** Publications refresh a certificate; changed knowledge or eligibility revokes it. */
export function previewContextIdentity(
  side: SideName,
  knownKey: string,
  eligibilityIdentity: string,
  pendingClaims: string,
  clearedCount: number,
  clearingEpoch: number,
): string {
  return JSON.stringify([
    side,
    knownKey,
    eligibilityIdentity,
    pendingClaims,
    clearedCount,
    clearingEpoch,
  ]);
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
  units: UnitCatalog,
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
    ruler: rangeRuler(unit, cursor, rules, units),
    circle: unitCircle(orderView(units, own, true, 1), gameOrderStyle),
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
  /** The session's unit catalog: a hull's size sizes its marks. */
  private readonly units: UnitCatalog;
  private key = "";
  /** The ruler last shown, for the lab's probes. */
  shown: RangeRuler | null = null;

  preview: readonly (DestinationMarker & { unit: number })[] = [];
  private previewClient: Pick<SimClient, "previewMove" | "previewBuilding"> | null = null;
  private resolved = "";
  private gesture = "";
  private generation = 0;
  private busy = false;
  private destinations: MoveDestination[] = [];
  building: BuildingPlacement | null = null;
  state: "idle" | "pending" | "ready" | "blocked" = "idle";

  constructor(units: UnitCatalog) {
    this.units = units;
  }

  /** At most one placement query is in flight; intermediate pointer updates
   *  are coalesced, and a reply cannot restore a cancelled gesture. */
  resolvePreview(
    intent: PointerPreview | null,
    selected: readonly OwnUnitView[],
    client: Pick<SimClient, "previewMove" | "previewBuilding"> | null,
    revision: number | string,
    identity = "",
  ) {
    const gesture = intent && client ? JSON.stringify([intent, identity]) : "";
    if (gesture !== this.gesture || client !== this.previewClient) {
      this.generation += 1;
      this.gesture = gesture;
      this.previewClient = client;
      this.destinations = [];
      this.building = null;
      this.state = intent && client ? "pending" : "idle";
      this.resolved = "";
    }
    const key = gesture && JSON.stringify([intent, revision]);
    const generation = this.generation;
    if (intent && client && key && !this.busy && key !== this.resolved) {
      this.busy = true;
      const query =
        intent.kind === "move"
          ? client
              .previewMove(intent.request)
              .then((destinations) => ({ destinations, building: null }))
          : client
              .previewBuilding(intent.request)
              .then((building) => ({ destinations: building.destinations, building }));
      void query
        .then(
          (result) => {
            if (this.generation === generation && this.previewClient === client) {
              this.destinations = result.destinations;
              this.building = result.building;
              this.state =
                result.building?.entrant || result.destinations.some((mark) => mark.placed)
                  ? "ready"
                  : !result.building || result.building.unproven
                    ? "pending"
                    : "blocked";
              this.resolved = key;
            }
          },
          () => {
            if (this.generation === generation && this.previewClient === client) {
              this.destinations = [];
              this.building = null;
              this.state = "pending"; // A failed query cannot prove an action unavailable.
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
    const byId = new Map(own.map((u) => [u.id, u]));
    return destinations.flatMap((mark) => {
      const u = byId.get(mark.unit);
      const alpha = opacity?.get(mark.unit) ?? (opacity ? 0 : 1);
      if (!mark.placed || !u || alpha <= 0) return [];
      const view = orderView(this.units, u, true, 1);
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

/** Movement and building queries carry the same intent that dispatch commits. */
export function previewForIntent(intent: PointerIntent): PointerPreview | null {
  switch (intent.kind) {
    case "move": {
      return {
        kind: "move",
        request: {
          units: intent.units,
          goal: intent.goal,
          route: intent.route,
          direction: intent.direction,
          facing: intent.facing,
          queued: intent.queued,
        },
      };
    }
    case "attack_move":
      return {
        kind: "move",
        request: { units: intent.units, goal: intent.goal, queued: intent.queued },
      };
    case "occupy_building": {
      return {
        kind: "building",
        request: {
          units: intent.units,
          building: intent.building,
          facing: intent.facing,
          queued: intent.queued,
        },
      };
    }
    default:
      return null;
  }
}

/** Pending availability stays plain; an attack may validly pursue its target. */
export function cursorForIntent(
  intent: PointerIntent,
  paint: Pick<PointerPaint, "state" | "building">,
): CursorAction {
  if (intent.kind === "none") return "default";
  if (intent.kind === "blocked") return "blocked";
  if (intent.kind === "attack") return intent.target.kind === "ground" ? "attack_ground" : "attack";
  if (paint.state === "blocked") return "blocked";
  if (paint.state !== "ready") return "default";
  if (intent.kind === "occupy_building") return paint.building?.entrant ? "garrison" : "default";
  if (intent.kind === "attack_move") return "attack_move";
  return intent.route === "fastest"
    ? "fast_move"
    : intent.direction === "reverse"
      ? "reverse_move"
      : "default";
}

/** Release acknowledgement corrects only the same pointer action. */
export function intentForOrder(order: Order, queued: boolean): PointerIntent | null {
  switch (order.kind) {
    case "attack":
      return { ...order, queued };
    case "move":
    case "attack_move":
    case "occupy_building": {
      const { gesture: _, ...intent } = order;
      return { ...intent, queued };
    }
    default:
      return null;
  }
}

export function samePointerIntent(a: PointerIntent, b: PointerIntent): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "attack" && b.kind === "attack")
    return (
      JSON.stringify([a.units, a.target, a.queued]) ===
      JSON.stringify([b.units, b.target, b.queued])
    );
  const request = previewForIntent(a);
  return request !== null && JSON.stringify(request) === JSON.stringify(previewForIntent(b));
}

/** Admission owns release feedback until its publication arrives. */
export function cursorForAcknowledgement(intent: PointerIntent, ack: CommandAck): CursorAction {
  const building = ack.building ?? null;
  if (ack.error) return building?.unproven ? "default" : "blocked";
  const destinations = ack.placement?.destinations ?? building?.destinations ?? [];
  const state =
    building?.entrant || destinations.some((d) => d.placed)
      ? "ready"
      : building && !building.unproven
        ? "blocked"
        : "pending";
  return cursorForIntent(intent, { state, building });
}

/** A released drag retains its facing; a fresh hover has no facing gesture. */
export function cursorForRelease(
  hover: PointerIntent,
  released: PointerIntent,
  ack: CommandAck,
): CursorAction | null {
  const target =
    (hover.kind === "move" || hover.kind === "occupy_building") &&
    hover.kind === released.kind &&
    hover.facing === undefined
      ? { ...released, facing: undefined }
      : released;
  return samePointerIntent(hover, target) ? cursorForAcknowledgement(hover, ack) : null;
}
