// Order presentation (D2, D2+): remaining routes draped on the walkable
// surface, the final marker with its facing (Total War style), queued
// waypoints, blocked/waiting marks, each soldier's resolved spot and the
// selection's marker under each selected unit. Holding Space (`all`) adds,
// for every own unit, a marker under each soldier's and vehicle's current
// position and a cover pip on each soldier's marker: the cover he has now at
// his current marker, the cover his spot gives at the final one. Built only
// from the observing side's own-unit view.
//
// Slice 27e drew it as a holo-tactical projection in one colour: thin lines
// of a fixed width on screen (`OrderStyle.line_px` at the camera's target,
// never under `min_line_m` on the ground), no filled discs; the glow is the
// overlay pass's halo. A unit's marker (under it, at its destination, each
// soldier's spot) is "the unit plus its facing": a circle with a small filled
// arrowhead on its rim (the user's pick). A squad's area ring is a big one:
// the same arrowhead on its rim at the squad's final facing. The circle a
// squad stands in is plain, and the route runs between the two circles'
// edges. A moving
// vehicle's travel shows as two chevrons behind it, against its facing on a
// reverse move, with a pulse marching along them. Colour carries meaning only
// where a player needs it: the cover tiers, a blocked route, and the
// selection.
import { vec2, type Vec2 } from "math";
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { WorldMeshes } from "./scene";

export type CoverTierName = "light" | "medium" | "heavy";

/** `presentation.overlay.orders`: every colour and width the orders draw. */
export interface OrderStyle {
  /** A route's width on screen at the camera's target. */
  line_px: number;
  /** The width of a marker's outline. */
  mark_px: number;
  /** Never narrower than this on the ground. */
  min_line_m: number;
  /** Height over the walkable surface. */
  lift_m: number;
  /** Every route and marker, whatever the order's kind. */
  color: Rgba;
  /** Queued waypoints and Space's current markers are the colour at these
   *  alphas. */
  queued_alpha: number;
  current_alpha: number;
  /** A route the unit can't take: the one warning tint. */
  blocked: Rgba;
  /** The marker under a selected unit. */
  selected: Rgba;
  /** Cover pips by tier (D2+). */
  cover: Record<CoverTierName, Rgba>;
  /** A cover pip's radius. */
  cover_pip_m: number;
  /** The pulse along a moving vehicle's travel chevrons, on the frame's
   *  presentation clock: cycles a second, and how far it dims them between
   *  beats (0 steady, 1 to nothing). */
  march: { cycles_per_s: number; amplitude: number };
  /** A squad's circles are drawn at this fraction of their radius. */
  area_draw_scale: number;
}

export function validateOrderStyle(style: OrderStyle): OrderStyle {
  const rgba = (c: unknown) =>
    Array.isArray(c) && c.length === 4 && c.every((v) => typeof v === "number" && v >= 0);
  const unit = (v: number) => v > 0 && v <= 1;
  const ok =
    style.line_px > 0 &&
    style.mark_px > 0 &&
    style.min_line_m > 0 &&
    style.lift_m >= 0 &&
    style.cover_pip_m > 0 &&
    unit(style.queued_alpha) &&
    unit(style.current_alpha) &&
    style.march?.cycles_per_s >= 0 &&
    style.march?.amplitude >= 0 &&
    style.march?.amplitude <= 1 &&
    unit(style.area_draw_scale) &&
    (["light", "medium", "heavy"] as const).every((k) => rgba(style.cover?.[k])) &&
    [style.color, style.blocked, style.selected].every(rgba);
  if (!ok)
    throw new Error(
      `presentation.overlay.orders: positive widths, lift_m ≥ 0, alphas in (0, 1], rgba color, blocked, selected and cover.{light, medium, heavy}, march.{cycles_per_s ≥ 0, amplitude in [0, 1]}, area_draw_scale in (0, 1]`,
    );
  return style;
}

/** A route's width in metres where one pixel spans `metresPerPx`: the one
 *  line weight the other ground rings (supply, suppression, the objective)
 *  draw with too. */
export function lineWidthM(style: OrderStyle, metresPerPx: number): number {
  return Math.max(style.min_line_m, style.line_px * metresPerPx);
}

export interface MemberOrderMark {
  spot: readonly [number, number];
  coverNow: CoverTierName | null;
  coverThere: CoverTierName | null;
}

export interface OrderView {
  position: readonly [number, number, number];
  goal: readonly [number, number] | null;
  state: string;
  route: readonly (readonly [number, number])[];
  queue: readonly (readonly [number, number])[];
  /** Living soldiers' positions (empty for a vehicle). */
  members: readonly (readonly [number, number, number])[];
  /** Each living soldier's resolved spot and cover, in `members` order. */
  memberOrders: readonly MemberOrderMark[];
  /** The bearing the unit ends its move at. */
  finalFacing: number;
  /** "reverse" on a reverse move (Q31). */
  direction: string | null;
  /** A squad's area round its anchor (27d): the disc its soldiers take
   *  their spots and fight in, which only an order moves; null for a
   *  vehicle. */
  area: { anchor: readonly [number, number]; radius: number } | null;
  /** In the player's selection: the marker under it is `selected`'s. */
  selected?: boolean;
}

export interface OrderOverlayOptions {
  /** Space held (D2+): current markers and cover pips, for every unit given. */
  all?: boolean;
  /** Metres one screen pixel spans at the camera's target: sets the line
   *  widths (`line_px`, `mark_px`). */
  metresPerPx: number;
}

/** Height of the walkable surface (bridge deck where one spans). */
export type SurfaceHeight = (x: number, y: number) => number;

/** Marks stacked on one spot rise in these steps, so none z-fights. */
const STACK_M = 0.04;
const DRAPE_STEP_M = 3;
/** A unit marker's circle radius: a soldier's, and a vehicle's (under it and
 *  at its destination), smaller than a tank's hull. */
const SOLDIER_R = 0.45;
const VEHICLE_R = 1.8;
/** How far behind a vehicle's own marker its travel chevrons start: clear of
 *  a tank's hull, which hides what lies under it. */
const CHEVRONS_BACK_M = 5.2;
/** A unit marker's arrowhead: its length past the rim, over the radius. */
const MARKER_HEAD = 0.6;
/** The arrowhead on a squad's area ring: the same shape, a fixed size. */
const AREA_HEAD_M = 1.6;
/** A travel chevron: length along the travel and spread across it. */
const CHEVRON_M = [0.9, 1.7] as const;

type P2 = readonly [number, number];

const withAlpha = (c: Rgba, a: number): Rgba => [c[0], c[1], c[2], c[3] * a];

/** One draw's widths and style. */
interface Pen {
  style: OrderStyle;
  z: SurfaceHeight;
  /** A route's width. */
  line: number;
  /** An outline's width. */
  stroke: number;
}

function ribbon(
  mesh: MeshBuilder,
  pen: Pen,
  a: P2,
  b: P2,
  color: Rgba,
  width: number,
  { dashed = false, lift = pen.style.lift_m } = {},
) {
  const dx = b[0] - a[0],
    dy = b[1] - a[1];
  const length = Math.hypot(dx, dy);
  if (length < 1e-6) return;
  const nx = (-dy / length) * (width / 2),
    ny = (dx / length) * (width / 2);
  const steps = Math.max(1, Math.ceil(length / DRAPE_STEP_M));
  for (let k = 0; k < steps; k++) {
    if (dashed && k % 2 === 1) continue;
    const [t0, t1] = [k / steps, (k + 1) / steps];
    const p0: P2 = [a[0] + dx * t0, a[1] + dy * t0];
    const p1: P2 = [a[0] + dx * t1, a[1] + dy * t1];
    const at = (p: P2, sx: number, sy: number) =>
      [p[0] + sx, p[1] + sy, pen.z(p[0] + sx, p[1] + sy) + lift] as const;
    mesh.quad(at(p0, -nx, -ny), at(p1, -nx, -ny), at(p1, nx, ny), at(p0, nx, ny), color);
  }
}

/** An outlined circle of `radius`, its line `width` wide centred on it. */
function ring(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  radius: number,
  color: Rgba,
  { width = pen.stroke, lift = pen.style.lift_m, dashed = false } = {},
) {
  const segments = Math.max(24, Math.ceil(radius * 10));
  groundAnnulus(mesh, c, Math.max(0, radius - width / 2), radius + width / 2, {
    z: pen.z,
    lift,
    segments,
    colorIn: color,
    dashed,
  });
}

function pip(mesh: MeshBuilder, pen: Pen, c: P2, color: Rgba) {
  groundAnnulus(mesh, c, 0, pen.style.cover_pip_m, {
    z: pen.z,
    lift: pen.style.lift_m + 2 * STACK_M,
    segments: 14,
    colorIn: color,
  });
}

function crossMark(mesh: MeshBuilder, pen: Pen, c: P2, size: number, color: Rgba) {
  const h = size / 2;
  ribbon(mesh, pen, [c[0] - h, c[1] - h], [c[0] + h, c[1] + h], color, pen.line);
  ribbon(mesh, pen, [c[0] - h, c[1] + h], [c[0] + h, c[1] - h], color, pen.line);
}

/** A point `d` metres from `c` along `bearing`. */
const along = (c: P2, bearing: number, d: number): P2 => [
  c[0] + Math.cos(bearing) * d,
  c[1] + Math.sin(bearing) * d,
];

/** A filled arrowhead on the rim of the circle of radius `r` round `c`,
 *  pointing along `bearing`: its base spans the rim, its tip `head` metres
 *  out (the user's pick among the marker shapes, slice 27e). */
function rimArrowhead(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  bearing: number,
  r: number,
  head: number,
  color: Rgba,
  lift: number,
) {
  const at = (p: P2) => [p[0], p[1], pen.z(p[0], p[1]) + lift] as const;
  const base = (s: number): P2 => {
    const p = along(c, bearing, r - head * 0.25);
    return [p[0] - Math.sin(bearing) * s, p[1] + Math.cos(bearing) * s];
  };
  mesh.triangle(
    at(base(head * 0.75)),
    at(along(c, bearing, r + head)),
    at(base(-head * 0.75)),
    color,
  );
}

/** How far a unit marker of radius `r` reaches along its facing (its
 *  arrowhead's tip): where a route arriving from ahead stops. */
const markerReach = (r: number) => r * (1 + MARKER_HEAD);

/** A unit's marker, "the unit plus its facing": a circle of radius `r` on
 *  `c` with a small filled arrowhead on its rim at `bearing`. */
function unitMarker(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  bearing: number,
  r: number,
  color: Rgba,
  { width = pen.stroke, lift = pen.style.lift_m } = {},
) {
  ring(mesh, pen, c, r, color, { width, lift });
  rimArrowhead(mesh, pen, c, bearing, r, r * MARKER_HEAD, color, lift + STACK_M);
}

/** A soldier's marker, with the cover tier's pip in its middle when he has
 *  cover (Space). */
function soldierMark(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  facing: number,
  cover: CoverTierName | null,
  color: Rgba,
  width = pen.stroke,
) {
  unitMarker(mesh, pen, c, facing, SOLDIER_R, color, { width });
  if (cover) pip(mesh, pen, c, pen.style.cover[cover]);
}

/** The way a moving vehicle travels, as two small chevrons behind its marker
 *  (and its hull): pointing along the marker's facing on a forward move, and
 *  against it on a reverse move (Q31), so a reversing vehicle reads at a
 *  glance. Drawn only under the moving unit, never at its destination. They
 *  go into the `animated` mesh: a pulse runs along them in the direction of
 *  travel (`OrderStyle.march`, on the frame's presentation clock). Each
 *  vertex's normal carries (phase, cycles a second, amplitude) for the
 *  overlay pass. */
function travelChevrons(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  facing: number,
  reverse: boolean,
  color: Rgba,
) {
  const back = facing + Math.PI;
  const travel = reverse ? back : facing;
  const { cycles_per_s, amplitude } = pen.style.march;
  // Nearest the marker first; the pulse reaches the one ahead in the
  // direction of travel later.
  const spots = [CHEVRONS_BACK_M, CHEVRONS_BACK_M + 1.3];
  spots.forEach((d, k) => {
    const lead = reverse ? k : spots.length - 1 - k;
    const normal = [lead * 0.25, cycles_per_s, amplitude] as const;
    const tip = along(c, back, d - (reverse ? 0 : CHEVRON_M[0]));
    marchChevron(mesh, pen, tip, travel, color, normal);
  });
}

/** One open chevron whose vertices carry `normal` for the overlay's march. */
function marchChevron(
  mesh: MeshBuilder,
  pen: Pen,
  tip: P2,
  bearing: number,
  color: Rgba,
  normal: readonly [number, number, number],
) {
  const [length, spread] = CHEVRON_M;
  const width = pen.line;
  const lift = pen.style.lift_m + STACK_M;
  const at = (p: P2) => [p[0], p[1], pen.z(p[0], p[1]) + lift] as const;
  const fx = Math.cos(bearing),
    fy = Math.sin(bearing);
  for (const side of [-1, 1]) {
    const end: P2 = [
      tip[0] - fx * length - fy * side * (spread / 2),
      tip[1] - fy * length + fx * side * (spread / 2),
    ];
    const [dx, dy] = [tip[0] - end[0], tip[1] - end[1]];
    const n = Math.hypot(dx, dy) || 1;
    const over: P2 = [tip[0] + (dx / n) * (width / 2), tip[1] + (dy / n) * (width / 2)];
    const [nx, ny] = [(-dy / n) * (width / 2), (dx / n) * (width / 2)];
    const a = at([end[0] - nx, end[1] - ny]),
      b = at([over[0] - nx, over[1] - ny]),
      e = at([over[0] + nx, over[1] + ny]),
      f = at([end[0] + nx, end[1] + ny]);
    mesh.vertex(a, normal, color).vertex(b, normal, color).vertex(e, normal, color);
    mesh.vertex(a, normal, color).vertex(e, normal, color).vertex(f, normal, color);
  }
}

/** The circle a squad stands in now: round its soldiers, from its position. */
function squadNow(u: OrderView, here: P2): number {
  const radius = u.members.reduce(
    (r, m) => Math.max(r, Math.hypot(m[0] - here[0], m[1] - here[1])),
    0,
  );
  return radius + SOLDIER_R + 0.4;
}

/** `points` from `from` with `head` metres cut off its start and `tail` off
 *  its end, as the segments left. */
function trimmed(from: P2, points: readonly P2[], head: number, tail: number): [P2, P2][] {
  const path: P2[] = [from, ...points];
  const lengths = path.slice(1).map((p, k) => Math.hypot(p[0] - path[k][0], p[1] - path[k][1]));
  const total = lengths.reduce((a, b) => a + b, 0);
  const [s0, s1] = [head, total - tail];
  if (s1 <= s0) return [];
  const out: [P2, P2][] = [];
  let at = 0;
  lengths.forEach((l, k) => {
    const [a, b] = [path[k], path[k + 1]];
    const [t0, t1] = [Math.max(s0, at), Math.min(s1, at + l)];
    if (t1 > t0 && l > 1e-9) {
      const point = (t: number) =>
        vec2.lerp(vec2.create(), a as Vec2, b as Vec2, (t - at) / l) as unknown as P2;
      out.push([point(t0), point(t1)]);
    }
    at += l;
  });
  return out;
}

export function buildOrderOverlay(
  units: readonly OrderView[],
  z: SurfaceHeight,
  style: OrderStyle,
  { all = false, metresPerPx }: OrderOverlayOptions,
): WorldMeshes {
  const pen: Pen = {
    style,
    z,
    line: lineWidthM(style, metresPerPx),
    stroke: Math.max(style.min_line_m, style.mark_px * metresPerPx),
  };
  const current = withAlpha(style.color, style.current_alpha);
  const queued = withAlpha(style.color, style.queued_alpha);
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  const animated = new MeshBuilder();
  const unoccluded = new MeshBuilder();
  for (const u of units) {
    const here: P2 = [u.position[0], u.position[1]];
    const moving = !!u.goal && u.state !== "route_blocked";
    const reverse = u.direction === "reverse";
    const f = u.finalFacing;
    const squad = u.members.length > 0;
    // Under each soldier (with his cover now) or the vehicle, where it is:
    // always for the selection, in its colour; for every unit with Space. A
    // moving vehicle's also shows which way it travels.
    if (all || u.selected) {
      const mark = u.selected ? style.selected : current;
      const mesh = u.selected ? opaque : translucent;
      if (!squad) {
        // Smaller than its hull, so drawn over it (the hull would hide it).
        unitMarker(unoccluded, pen, here, f, VEHICLE_R, mark, { width: pen.line });
        if (moving) travelChevrons(animated, pen, here, f, reverse, mark);
      }
      u.members.forEach((m, k) =>
        soldierMark(
          mesh,
          pen,
          [m[0], m[1]],
          f,
          all ? (u.memberOrders[k]?.coverNow ?? null) : null,
          mark,
          u.selected ? pen.line : pen.stroke,
        ),
      );
    }
    // A squad's rings are drawn at `area_draw_scale` of their radius (the
    // movement area itself is 27d's).
    const drawn = (radius: number) => radius * style.area_draw_scale;
    if (!u.goal) {
      // A holding squad's area round its anchor (27d), where it fights.
      if (all && u.area)
        ring(translucent, pen, u.area.anchor, drawn(u.area.radius), current, { width: pen.line });
      // A holding squad's soldiers walking to their posts (cover, a step out).
      if (all)
        u.memberOrders.forEach((m, k) => {
          const p = u.members[k];
          if (p && Math.hypot(m.spot[0] - p[0], m.spot[1] - p[1]) > 2 * SOLDIER_R) {
            ribbon(translucent, pen, [p[0], p[1]], m.spot, current, pen.stroke);
            soldierMark(opaque, pen, m.spot, f, m.coverThere, style.color);
          }
        });
      continue;
    }
    const blocked = u.state === "route_blocked";
    // A moving squad's area is already its destination's: an order moves it.
    const area = squad && u.area ? { center: u.area.anchor, radius: u.area.radius } : null;
    // The route runs between the edges of what it joins: a squad's circle
    // where it stands and its area ring; a vehicle's marker, where it arrives.
    const last = u.route.length > 1 ? u.route[u.route.length - 2] : here;
    const inbound = Math.atan2(last[1] - u.goal[1], last[0] - u.goal[0]);
    const ahead = Math.cos(inbound - f) > Math.cos(Math.PI / 3);
    const now = squad && !blocked ? drawn(squadNow(u, here)) : 0;
    const tail = blocked
      ? 0
      : area
        ? drawn(area.radius)
        : ahead
          ? markerReach(VEHICLE_R)
          : VEHICLE_R;
    if (now > 0) ring(opaque, pen, here, now, style.color, { width: pen.line });
    for (const [a, b] of trimmed(here, u.route, now, tail))
      ribbon(opaque, pen, a, b, style.color, pen.line);
    if (blocked) {
      ribbon(opaque, pen, here, u.goal, style.blocked, pen.line, { dashed: true });
      crossMark(opaque, pen, u.goal, 6, style.blocked);
      ring(opaque, pen, u.goal, 4.5, style.blocked, { width: pen.line });
    } else if (area) {
      // A squad's final marker: its area ring with the facing's arrowhead on
      // its rim, a big unit marker, and each soldier's spot in it.
      const radius = drawn(area.radius);
      ring(opaque, pen, area.center, radius, style.color, { width: pen.line });
      rimArrowhead(
        opaque,
        pen,
        area.center,
        f,
        radius,
        AREA_HEAD_M,
        style.color,
        style.lift_m + STACK_M,
      );
      for (const m of u.memberOrders)
        soldierMark(opaque, pen, m.spot, f, all ? m.coverThere : null, style.color);
    } else {
      // A vehicle's: its marker, pointing where it will face. Its travel
      // shows under the moving vehicle, not here.
      unitMarker(opaque, pen, u.goal, f, VEHICLE_R, style.color, { width: pen.line });
    }
    // Waiting for the way ahead to clear: a broken ring round the unit.
    if (u.state === "waiting")
      ring(opaque, pen, here, CHEVRONS_BACK_M + 2, style.color, {
        width: pen.line,
        dashed: true,
      });
    let prev: P2 = u.goal;
    for (const q of u.queue) {
      ribbon(translucent, pen, prev, q, queued, pen.line, { dashed: true });
      ring(translucent, pen, q, 2.8, queued);
      prev = q;
    }
  }
  return {
    opaque: opaque.build(),
    translucent: translucent.build(),
    animated: animated.build(),
    unoccluded: unoccluded.build(),
  };
}
