// Pointer-following ground paint: the held destination/facing preview, the
// Space range ruler, the attack-ground cursor's footprint and reach rings and
// the outline of the building a garrison order would enter share one feed,
// refreshed only when their geometry changes.
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import {
  EMPTY_MESH,
  concatMeshes,
  groundRing,
  MeshBuilder,
  type Mesh,
} from "@packages/battle-renderer/src/mesh";
import {
  buildDestinationPreview,
  buildHoverMarker,
  type SurfaceHeight,
  type DestinationMarker,
} from "@packages/battle-renderer/src/orderOverlay";
import {
  circleExit,
  unitCircle,
  destinationCircle,
  type UnitCircle,
} from "@packages/battle-renderer/src/unitCircle";
import { buildRangeRuler, type RulerLine } from "@packages/battle-renderer/src/rangeRulerOverlay";
import type { BuildingIndex } from "@packages/battle-renderer/src/models/buildingReferences";
import { airborne, fixedWing, type UnitCatalog } from "@packages/scene-assets/src/units";
import { GroundStrikes } from "@wasm/game_wasm";
import type { PurchaseGhost } from "@web/battle/input/purchasePlacement";
import {
  closestUnit,
  rangeRuler,
  type RangeRuler,
  type RulerRules,
} from "@web/battle/present/rangeRuler";
import type { PointerIntent } from "@web/battle/input/pointerIntent";
import type { CursorAction } from "@web/battle/present/gameCursor";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observationView";
import type {
  MovePreviewRequest,
  MoveDestination,
  BuildingPlacement,
  BuildingPreviewRequest,
  Order,
  CommandAck,
} from "@web/battle/sim/protocol";
import type { Vec3 } from "math";
import { garrisonTargetOutline, orderView } from "./battleOverlay";
import type { OrderPreviewSnapshot } from "./orderPreview";
import { Feed } from "@web/feed";
import { groundUnderRay, type StaticWorld } from "./useStaticWorld";
import { gameAttackGroundSpread, gameOrderStyle, gameRulerStyle, gameStroke } from "./gameOverlay";

/** The attack-ground cursor's marks: the circle its strikes there can
 *  damage (`radius_m`, null when none of the selection reaches the aim), and
 *  round the selected unit nearest the aim the nearest and farthest it
 *  strikes from where it stands (`reach`, null for a selection with none: a
 *  jet strikes from its runs). */
export interface AttackGroundMarks {
  center: readonly [number, number];
  radius_m: number | null;
  reach: { from: readonly [number, number]; near_m: number; far_m: number } | null;
}

/** The simulation's ground strikes (`GroundStrikes`) for a scenario's
 *  resolved rules, admitted once per rules; the previous rules' are freed. */
let strikes: { rules: string; owner: GroundStrikes } | null = null;
function groundStrikes(rules: string): GroundStrikes {
  if (strikes?.rules !== rules) {
    strikes?.owner.free();
    strikes = { rules, owner: new GroundStrikes(rules) };
  }
  return strikes.owner;
}

/** The attack-ground cursor's marks under `rules` (a scenario's resolved
 *  rules), as the simulation's footprint and reach say. The circle is the
 *  widest area the selected units' strikes there can damage, never so small
 *  it vanishes close in, and absent when none of them can strike the aim
 *  from where it stands. Null when none strikes an area. */
export function attackGroundMarks(
  center: readonly [number, number],
  selected: readonly OwnUnitView[],
  drawnAt: ReadonlyMap<number, Readonly<Vec3>>,
  rules: string,
): AttackGroundMarks | null {
  const strikes = groundStrikes(rules);
  const strikers = selected.map((unit) => {
    const at = drawnAt.get(unit.id) ?? unit.position;
    const distance = Math.hypot(center[0] - at[0], center[1] - at[1]);
    const reach = strikes.reach_m(unit.kind);
    return {
      from: [at[0], at[1]] as const,
      distance,
      radius: strikes.radius_m(unit.kind, distance),
      reach: reach ? { near_m: reach[0], far_m: reach[1] } : null,
    };
  });
  const radii = strikers.flatMap((s) => (s.radius === undefined ? [] : [s.radius]));
  // One subject, one mark: the reach of the striker nearest the aim.
  const nearest = strikers
    .filter((s) => s.reach)
    .reduce<(typeof strikers)[number] | null>(
      (best, s) => (best && best.distance <= s.distance ? best : s),
      null,
    );
  const reach = nearest?.reach ? { from: nearest.from, ...nearest.reach } : null;
  if (!radii.length && !reach) return null;
  return { center, radius_m: radii.length ? Math.max(6, ...radii) : null, reach };
}

/** The ruler shown, and the circle the orders draw round its unit (with
 *  Space held), which the painted line leaves from. */
interface ShownRuler {
  ruler: RangeRuler;
  circle: UnitCircle | null;
}

/** The building under the pointer that the order there would garrison: its
 *  prop, and its outline at a line width (`garrisonTargetOutline`). */
export interface GarrisonTarget {
  building: number;
  outline: (line: number) => Mesh | null;
}

export type PointerPreview =
  | { kind: "move"; request: MovePreviewRequest }
  | { kind: "building"; request: BuildingPreviewRequest };

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

/** The ring a purchase being placed shows (the pointer is over its ghost):
 *  the circle its unit will show selected once it stands there, or null for
 *  a squad until a preview names its area. */
export function placementCircle(ghost: PurchaseGhost, units: UnitCatalog): UnitCircle | null {
  const type = units.type(ghost.variant);
  const hull = units.hull(ghost.variant);
  if (!hull && ghost.areaRadius === null) return null;
  const [x, y] = ghost.destination;
  return unitCircle(
    {
      position: [x, y, 0],
      goal: null,
      state: "idle",
      members: ghost.spots.map(([sx, sy]): [number, number, number] => [sx, sy, 0]),
      area:
        ghost.areaRadius === null ? null : { anchor: ghost.destination, radius: ghost.areaRadius },
      hullHalfLength: hull?.half_extents_m[0] ?? 0,
      building: null,
      yaw: ghost.facing,
      selected: true,
      fixedWing: fixedWing(type),
      airborne: airborne(type),
    },
    gameOrderStyle,
  );
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

/** Where each squad a building order sends stands to step in, as a move's
 *  destinations: one ring per entry, pointing the way its squad walks there. */
function entryDestinations(
  building: BuildingPlacement | null | undefined,
  own: readonly OwnUnitView[],
): MoveDestination[] {
  return (building?.entries ?? []).flatMap(({ unit, approach }) => {
    const u = own.find((o) => o.id === unit);
    if (!u) return [];
    const facing = Math.atan2(approach[1] - u.position[1], approach[0] - u.position[0]);
    return [{ unit, goal: approach, facing, placed: true, spots: [] }];
  });
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
  /** The building prop last outlined as a garrison target, for the probes. */
  garrison: number | null = null;

  preview: readonly (DestinationMarker & { unit: number })[] = [];

  constructor(units: UnitCatalog) {
    this.units = units;
  }

  /** One mark per move destination and per building entry. */
  markers(
    order: { destinations: readonly MoveDestination[]; building?: BuildingPlacement | null },
    own: readonly OwnUnitView[],
    opacity: ReadonlyMap<number, number> | null = null,
  ) {
    const destinations = [...order.destinations, ...entryDestinations(order.building, own)];
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
    attackGround: AttackGroundMarks | null = null,
    renderScale = 1,
    placing: PurchaseGhost | null = null,
    garrison: GarrisonTarget | null = null,
  ) {
    this.preview = preview;
    // A purchase being placed is under the pointer: its ghost shows its ring.
    const hover = placing && placementCircle(placing, this.units);
    this.shown = shown?.ruler ?? null;
    this.garrison = garrison?.building ?? null;
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
        .map((p) =>
          [p.unit, ...p.c, p.r, p.facing ?? 0, p.placed, p.opacity, !!p.unload, metresPerPx].join(),
        )
        .join(";");
    const marksKey = attackGround
      ? `:${attackGround.center.join(",")}:${attackGround.radius_m?.toFixed(3)}:${
          attackGround.reach
            ? [...attackGround.reach.from, attackGround.reach.near_m, attackGround.reach.far_m]
                .map((v) => v.toFixed(3))
                .join(",")
            : ""
        }:${metresPerPx}`
      : ":";
    const hoverKey = hover
      ? `:${[...hover.c, hover.r, hover.facing ?? 0, metresPerPx].join()}`
      : ":";
    const nextKey =
      key + marksKey + hoverKey + `:${garrison?.building ?? ""}:${metresPerPx}:${renderScale}`;
    if (nextKey === this.key) return;
    this.key = nextKey;
    const marks = attackGround
      ? (() => {
          const mesh = new MeshBuilder();
          const width = gameStroke(metresPerPx, renderScale)(1.6);
          if (attackGround.radius_m !== null)
            groundRing(
              mesh,
              attackGround.center,
              attackGround.radius_m,
              width,
              gameAttackGroundSpread,
              { z, segments: 48 },
            );
          // The reach rings are where the ruler's reaches end (its tick),
          // dashed as a limit, never a solid edge like sight's.
          const reach = attackGround.reach;
          for (const radius of reach ? [reach.near_m, reach.far_m] : [])
            if (reach && radius > 0)
              groundRing(mesh, reach.from, radius, width, gameRulerStyle.tick, {
                z,
                dashed: true,
                segments: Math.min(512, Math.max(48, Math.ceil(radius / 5))),
              });
          return mesh.build();
        })()
      : EMPTY_MESH;
    const stroke = gameStroke(metresPerPx, renderScale);
    this.feed.set(
      concatMeshes([
        marks,
        garrison?.outline(stroke(gameOrderStyle.line_px)) ?? EMPTY_MESH,
        shown
          ? buildRangeRuler(
              rulerLine(shown.ruler, shown.circle),
              z,
              gameRulerStyle,
              metresPerPx,
              gameStroke(metresPerPx, renderScale),
            )
          : EMPTY_MESH,
        preview.length
          ? buildDestinationPreview(preview, z, gameOrderStyle, {
              stroke: gameStroke(metresPerPx, renderScale),
            })
          : EMPTY_MESH,
        hover
          ? buildHoverMarker(hover, z, gameOrderStyle, {
              stroke: gameStroke(metresPerPx, renderScale),
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
    // Unloading at a point is placed as the move that takes it there; into a
    // host, the simulation picks where to set down, and admission shows it.
    case "unload":
      return intent.goal && !intent.into
        ? {
            kind: "move",
            request: {
              units: intent.units,
              goal: intent.goal,
              route: intent.route,
              queued: intent.queued,
            },
          }
        : null;
    case "occupy_building": {
      return {
        kind: "building",
        request: {
          units: intent.units,
          building: intent.building,
          queued: intent.queued,
        },
      };
    }
    default:
      return null;
  }
}

/** A move still being placed stays plain; an attack may validly pursue its
 *  target. */
export function cursorForIntent(
  intent: PointerIntent,
  paint: Pick<OrderPreviewSnapshot, "state" | "building" | "carried">,
): CursorAction {
  if (intent.kind === "none" || intent.kind === "cancel") return "default";
  // A query in flight shows what the last one settled on.
  const state = paint.state === "pending" ? (paint.carried ?? "pending") : paint.state;
  if (intent.kind === "blocked") return "blocked";
  if (intent.kind === "attack") return intent.target.kind === "ground" ? "attack_ground" : "attack";
  // Boarding is the simulation's to admit; its refusal names the squads left out.
  if (intent.kind === "board" || intent.kind === "collect") return "board";
  // So is unloading into a host, which no hover preview places.
  if (intent.kind === "unload" && intent.into)
    return intent.into.kind === "building" ? "unload_garrison" : "unload_board";
  if (state === "blocked") return "blocked";
  // Entering a building shows at once, as boarding does (the intent names
  // only squads); only an answer that gives no squad an entry takes the
  // badge away.
  if (intent.kind === "occupy_building")
    return paint.building?.entries.length || state !== "ready" ? "garrison" : "default";
  if (state !== "ready") return "default";
  if (intent.kind === "attack_move") return "attack_move";
  if (intent.kind === "unload") return "unload";
  return intent.route === "fastest"
    ? "fast_move"
    : intent.direction === "reverse"
      ? "reverse_move"
      : "default";
}

/** The cursor for the pointer's `intent`: plain over an own unit
 *  (`overOwnUnit`) unless the click would board it; while a `release` of the
 *  same action awaits its publication, what admission said; else what the
 *  `query`'s preview answered, when it asked about this intent. */
export function pointerCursor(
  intent: PointerIntent,
  overOwnUnit: boolean,
  release: { released: PointerIntent; ack: CommandAck } | null,
  query: { intent: PointerIntent; preview: OrderPreviewSnapshot },
): CursorAction {
  const intoUnit =
    intent.kind === "board" ||
    intent.kind === "collect" ||
    (intent.kind === "unload" && intent.into?.kind === "carrier");
  if (overOwnUnit && !intoUnit) return "default";
  const acknowledged = release && cursorForRelease(intent, release.released, release.ack);
  if (acknowledged) return acknowledged;
  return cursorForIntent(
    intent,
    samePointerIntent(intent, query.intent) ? query.preview : { state: "pending", building: null },
  );
}

/** The building the garrison badge (`action`) offers for `intent`, outlined
 *  as a held one is; null under any other cursor. */
export function garrisonTarget(
  action: CursorAction | null,
  intent: PointerIntent,
  shown: ObservationView | null,
  buildings: Pick<BuildingIndex, "partBuilding" | "parts"> | null,
  z: SurfaceHeight,
): GarrisonTarget | null {
  const building =
    action === "garrison" && intent.kind === "occupy_building"
      ? intent.building
      : action === "unload_garrison" && intent.kind === "unload" && intent.into?.kind === "building"
        ? intent.into.building
        : null;
  if (building === null || !shown || !buildings) return null;
  return {
    building,
    outline: (line) => garrisonTargetOutline(shown, buildings, building, line, z),
  };
}

/** Release acknowledgement corrects only the same pointer action. */
export function intentForOrder(order: Order, queued: boolean): PointerIntent | null {
  switch (order.kind) {
    case "attack":
      return { ...order, queued };
    case "move":
    case "attack_move":
    case "occupy_building":
    case "unload": {
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
  if (a.kind === "unload" && b.kind === "unload" && (a.into || b.into))
    return (
      JSON.stringify([a.units, a.into, a.queued]) === JSON.stringify([b.units, b.into, b.queued])
    );
  const request = previewForIntent(a);
  return request !== null && JSON.stringify(request) === JSON.stringify(previewForIntent(b));
}

/** Admission owns release feedback until its publication arrives. */
export function cursorForAcknowledgement(intent: PointerIntent, ack: CommandAck): CursorAction {
  const building = ack.building ?? null;
  if (ack.error) return "blocked";
  const destinations = ack.placement?.destinations ?? [];
  const state =
    building?.entries.length || destinations.some((d) => d.placed)
      ? "ready"
      : building
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
    hover.kind === "move" && released.kind === "move" && hover.facing === undefined
      ? { ...released, facing: undefined }
      : released;
  return samePointerIntent(hover, target) ? cursorForAcknowledgement(hover, ack) : null;
}
