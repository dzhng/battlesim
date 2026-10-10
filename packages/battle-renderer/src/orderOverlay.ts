// Order presentation (D2, D2+): the one Space view, drawn for each unit the
// reveal shows (`OrderView.reveal`: every own unit with Space held, the units
// of an order just given as it flashes and fades) at that opacity: remaining
// routes draped on the walkable surface, the final marker with its facing
// (Total War style), queued waypoints, a blocked route's mark, each
// soldier's resolved spot, a marker under each soldier's and vehicle's
// current position and a cover pip on each soldier's marker (the cover he
// has now at his current marker, the cover his spot gives at the final one).
// Apart from them, whatever the reveal, the selection's own markers under
// each selected unit: they are not order marks. Built only from the
// observing side's own-unit view.
//
// It is drawn as a holo-tactical projection: thin lines sized on screen
// (`OrderStyle.line_px` at the camera's target, thinned as the camera pulls
// out by the one stroke rule, `strokeWidth.ts`), no filled discs, all of it
// paint on the ground (`frame/paintedMarks.ts`) but for an aircraft's drop
// line, which stands in the air in the overlay (D18), glowing past its colour's
// full value by its role's `OrderStyle.glow`. Every ground unit's circle, under it or at
// its destination, is one marker, "the unit plus its facing" (`unitMarker`):
// a circle with a small filled arrowhead on its rim (the user's pick). A
// vehicle's lies under its hull; a squad's is the ring round its soldiers
// while it moves and its area ring while it holds, its arrowhead at the
// squad's heading or facing; the destination's points the final facing, and
// the route runs between the two circles' edges. Each soldier's marker is a
// plain ring holding his cover pip. A moving vehicle's travel shows as two
// chevrons behind it, against its facing on a reverse move, with a pulse
// marching along them. Colour carries meaning only where a player needs it:
// the cover tiers, a blocked route, and the selection.
import { vec2, type Vec2 } from "math";
import {
  concatMeshes,
  fadeAlpha,
  glowing,
  groundAnnulus,
  groundRing,
  groundStrip,
  isRgba,
  MeshBuilder,
  VERTEX_FLOATS,
  paintOnly,
  rgbA,
  type Mesh,
  type Rgba,
} from "./mesh";
import type { WorldMeshes } from "./scene";
import type { StrokeWidth } from "./strokeWidth";

type CoverTierName = "light" | "medium" | "heavy";

/** `presentation.overlay.orders`: every colour and width the orders draw. */
export interface OrderStyle {
  /** A route's width on screen at the camera's target. */
  line_px: number;
  /** The width of a marker's outline. */
  mark_px: number;
  /** A soldier's own markers (under him and at his spot) keep a finer
   *  weight: his outline, and a selected soldier's. */
  soldier_mark_px: number;
  soldier_line_px: number;
  /** Every route and marker, whatever the order's kind. */
  color: Rgba;
  /** Queued waypoints and the current markers are the colour at these
   *  alphas. */
  queued_alpha: number;
  current_alpha: number;
  /** A route the unit can't take: the HUD's one "can't" colour. */
  blocked: Rgba;
  /** A selected unit's own marker (its circle and arrowhead) and its
   *  soldiers' markers. */
  selected: Rgba;
  /** How far each role's paint glows past its colour's full value (up to 2,
   *  the paint's range): the order marks (and a blocked route), and the
   *  selection's. Tuned apart, since the tone mapper pulls each hue toward
   *  white at its own rate. */
  glow: { order: number; selected: number };

  /** The cover pips' paint glows this much past their colour's full value:
   *  a dark green pip on grass still reads. */
  cover_glow: number;
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
  /** How far a vehicle's marker circle (under it and at its destination)
   *  reaches past its hull's half-length (`OrderView.hullHalfLength`): the
   *  ring and its arrowhead peek out from under any hull, a jeep's or a
   *  tank's, from the kind's own footprint. */
  vehicle_marker_margin_m: number;
  /** How far a garrisoned squad's circle reaches past its building's
   *  corners (`OrderView.building`): the ring encloses the whole house. */
  building_marker_margin_m: number;
  /** An aircraft's drop line (D18), dots rising from its marker's centre
   *  on the ground to its airframe: a dot's size on screen, by the stroke
   *  rule like every mark's; the pitch from one dot to the next, in dot
   *  sizes; its opacity at the ground, whatever its marker's (the marker's
   *  hue, so a quiet marker's dots still read over pale ground); and the
   *  share of that it fades to at the airframe. */
  drop_line_px: number;
  drop_line_spacing: number;
  drop_line_alpha: number;
  drop_line_fade: number;
}

export function validateOrderStyle(style: OrderStyle): OrderStyle {
  const unit = (v: number) => v > 0 && v <= 1;
  const ok =
    style.line_px > 0 &&
    style.mark_px > 0 &&
    style.soldier_mark_px > 0 &&
    style.soldier_line_px > 0 &&
    style.cover_pip_m > 0 &&
    unit(style.queued_alpha) &&
    unit(style.current_alpha) &&
    style.march?.cycles_per_s >= 0 &&
    style.march?.amplitude >= 0 &&
    style.march?.amplitude <= 1 &&
    unit(style.area_draw_scale) &&
    [style.glow?.order, style.glow?.selected].every((g) => g > 0 && g <= 2) &&
    style.cover_glow >= 1 &&
    style.cover_glow <= 2 &&
    style.vehicle_marker_margin_m > 0 &&
    style.building_marker_margin_m > 0 &&
    style.drop_line_px > 0 &&
    style.drop_line_spacing > 1 &&
    unit(style.drop_line_alpha) &&
    unit(style.drop_line_fade) &&
    (["light", "medium", "heavy"] as const).every((k) => isRgba(style.cover?.[k])) &&
    [style.color, style.blocked, style.selected].every(isRgba);
  if (!ok)
    throw new Error(
      `presentation.overlay.orders: positive widths, alphas in (0, 1], rgba color, blocked, selected and cover.{light, medium, heavy}, march.{cycles_per_s ≥ 0, amplitude in [0, 1]}, area_draw_scale in (0, 1], glow.{order, selected} in (0, 2], cover_glow in [1, 2], vehicle_marker_margin_m, building_marker_margin_m and drop_line_px > 0, drop_line_spacing > 1, drop_line_alpha and drop_line_fade in (0, 1]`,
    );
  return style;
}

interface MemberOrderMark {
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
  /** The bearing it faces now (its published yaw). */
  yaw: number;
  /** "reverse" on a reverse move (Q31). */
  direction: string | null;
  /** A squad's area round its anchor: the disc its soldiers take
   *  their spots and fight in, which only an order moves; null for a
   *  vehicle. */
  area: { anchor: readonly [number, number]; radius: number } | null;
  /** A vehicle's hull half-length (its footprint's, `physics`); 0 for a
   *  squad. Its marker circle is this plus `vehicle_marker_margin_m`. */
  hullHalfLength: number;
  /** The building a garrisoned squad holds (inside or leaving): its
   *  footprint's centre and half extents; null otherwise. The squad's circle
   *  encloses it, `building_marker_margin_m` past its corners. */
  building: { center: readonly [number, number]; half: readonly [number, number] } | null;
  /** An aircraft: its marker stays on the ground under it whatever it is
   *  doing, joined to the airframe (its `position`) by a drop line. */
  aircraft?: boolean;
  /** How high its ghost stands over where its orders end while one shows
   *  (an aircraft's cruise height, Space held): a drop line joins the ghost
   *  to that end's marker, as one joins the airframe to its own. 0 or
   *  absent: no ghost, no line. */
  ghostAloft?: number;
  /** In the player's selection: the marker under it is `selected`'s. */
  selected?: boolean;
  /** The opacity its order marks (the Space view) draw at, in [0, 1]: 0 or
   *  absent draws none, and a selected unit then shows only the selection's
   *  own markers. */
  reveal?: number;
}

interface OrderOverlayOptions {
  /** The stroke widths at the camera's zoom (`strokeWidth`): turns the
   *  style's pixel widths (`line_px`, `mark_px`, ...) into metres. */
  stroke: StrokeWidth;
}

/** Height of the walkable surface (bridge deck where one spans). */
export type SurfaceHeight = (x: number, y: number) => number;

/** A soldier's marker circle radius (a vehicle's is the style's
 *  its hull's half-length plus `vehicle_marker_margin_m`). */
const SOLDIER_R = 0.45;
/** How far outside a vehicle's marker its travel chevrons start. */
const CHEVRONS_GAP_M = 1;
/** A unit marker's arrowhead: its length past the rim, over the radius. */
const MARKER_HEAD = 0.6;
/** …never longer than this: a vehicle's reads as a pointer, not a wedge. */
const MARKER_HEAD_MAX_M = 1.5;
const markerHead = (r: number) => Math.min(r * MARKER_HEAD, MARKER_HEAD_MAX_M);
/** A queued waypoint's ring, and a blocked route's warning ring. */
const QUEUED_R = 2.8;
const BLOCKED_R = 4.5;
/** A travel chevron: length along the travel and spread across it. */
const CHEVRON_M: readonly [number, number] = [0.9, 1.7];

type P2 = readonly [number, number];

/** One draw's widths and style. */
interface Pen {
  style: OrderStyle;
  z: SurfaceHeight;
  /** A route's width. */
  line: number;
  /** An outline's width. */
  stroke: number;
  /** A soldier's marker's width, and a selected soldier's. */
  soldier: number;
  soldierSelected: number;
  /** An aircraft's drop line's width. */
  drop: number;
}

/** A route's or a mark's straight line, `width` wide. */
function ribbon(
  mesh: MeshBuilder,
  pen: Pen,
  a: P2,
  b: P2,
  color: Rgba,
  width: number,
  { dashed = false } = {},
) {
  groundStrip(mesh, a, b, width, color, { z: pen.z, dashed });
}

/** An outlined circle of `radius`, its line `width` wide centred on it. */
function ring(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  radius: number,
  color: Rgba,
  { width = pen.stroke, dashed = false } = {},
) {
  groundRing(mesh, c, radius, width, color, { z: pen.z, dashed });
}

/** A cover pip in the middle of a soldier's marker. */
function pip(mesh: MeshBuilder, pen: Pen, c: P2, color: Rgba) {
  groundAnnulus(mesh, c, 0, pen.style.cover_pip_m, {
    z: pen.z,
    segments: 14,
    colorIn: glowing(color, pen.style.cover_glow),
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
 *  out (the user's pick among the marker shapes). */
function rimArrowhead(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  bearing: number,
  r: number,
  head: number,
  color: Rgba,
) {
  const at = (p: P2) => [p[0], p[1], pen.z(p[0], p[1])] as const;
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
const markerReach = (r: number) => r + markerHead(r);

/** How far from its centre a line leaving (or reaching) circle `c` along
 *  `bearing` meets it: its rim, or its arrowhead's tip near its facing (a
 *  circle with no facing has no arrowhead). */
export function circleReach(c: Pick<UnitCircle, "r" | "facing">, bearing: number): number {
  return c.facing !== null && Math.cos(bearing - c.facing) > Math.cos(Math.PI / 3)
    ? markerReach(c.r)
    : c.r;
}

/** How far along the ground from `from` toward `to` a line leaves `circle`
 *  (clearing its arrowhead where it leaves along the facing): 0 with no
 *  circle, or from outside one it doesn't cross. From the circle's centre it
 *  is `circleReach`; with `to` inside the circle it is past `to`. */
export function circleExit(
  from: readonly number[],
  to: readonly number[],
  circle: UnitCircle | null,
): number {
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

/** A unit's marker, "the unit plus its facing": a circle of radius `r` on
 *  `c` with a small filled arrowhead on its rim at `bearing`. */
function unitMarker(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  bearing: number | null,
  r: number,
  color: Rgba,
  { width = pen.stroke } = {},
) {
  ring(mesh, pen, c, r, color, { width });
  if (bearing !== null) rimArrowhead(mesh, pen, c, bearing, r, markerHead(r), color);
}

/** A circle marker: the circle with the filled arrowhead on its rim at its
 *  facing, if it has one (`unitMarker`), in the route's line weight. */
function circleMarker(mesh: MeshBuilder, pen: Pen, m: UnitCircle, color: Rgba) {
  unitMarker(mesh, pen, m.c, m.facing, m.r, color, { width: pen.line });
}

/** An airframe this close to the ground under it draws no drop line. */
const DROP_MIN_M = 0.5;
/** The most dots a drop line draws: past it, a close camera spaces them wider. */
const DROP_MAX_DOTS = 48;

/** Where an aircraft is over the ground (D18): its marker on the ground
 *  under it (paint, in `paint`), and a dotted drop line rising from the
 *  marker's centre to the airframe (`top`, its published position). The
 *  dots stand in the air, so they are overlay geometry (in `line`),
 *  depth-tested like every overlay: a roof under the airframe hides the
 *  ones below it, and the marker hides under the roof as paint on the
 *  ground does. */
function aircraftMarker(
  paint: MeshBuilder,
  line: MeshBuilder,
  pen: Pen,
  top: readonly [number, number, number],
  marker: UnitCircle,
  color: Rgba,
) {
  circleMarker(paint, pen, marker, color);
  dropLine(line, pen, top, color);
}

/** A drop line of dots from the ground under `top` up to `top`, in
 *  `color`'s hue: `drop_line_alpha` at the ground, fading to its
 *  `drop_line_fade` share at the top. Under an airframe or its ghost. */
function dropLine(
  line: MeshBuilder,
  pen: Pen,
  top: readonly [number, number, number],
  color: Rgba,
) {
  const ground = pen.z(top[0], top[1]);
  const height = top[2] - ground;
  if (height < DROP_MIN_M) return;
  const dot = Math.min(pen.drop, height / 2);
  const pitch = Math.max(dot * pen.style.drop_line_spacing, height / DROP_MAX_DOTS);
  // The first dot sits on the ground and the last ends at the top.
  const gaps = Math.max(1, Math.floor((height - dot) / pitch));
  const { drop_line_alpha: alpha, drop_line_fade: fade } = pen.style;
  for (let i = 0; i <= gaps; i++) {
    const t = i / gaps;
    const z = ground + dot / 2 + t * (height - dot);
    line.mark([top[0], top[1], z], dot / 2, rgbA(color, alpha * (1 - t * (1 - fade))));
  }
}

/** A route from the unit's own circle to a destination circle, clipped at
 *  both: it leaves from the first's rim and ends at the second's, at the
 *  arrowhead's tip where it leaves or arrives along the facing. Either circle may be absent
 *  (not drawn): the route runs to the centre there. */
function routeBetween(
  mesh: MeshBuilder,
  pen: Pen,
  from: UnitCircle | null,
  route: readonly P2[],
  to: UnitCircle | null,
  color: Rgba,
  { dashed = false } = {},
) {
  const start = from?.c ?? route[0];
  if (!start || route.length === 0) return;
  // Where it leaves or arrives along a circle's facing, it clears the
  // circle's arrowhead (to its tip), never running over it.
  const reach = circleReach;
  const first = route[0];
  const head = from ? reach(from, Math.atan2(first[1] - start[1], first[0] - start[0])) : 0;
  let tail = 0;
  if (to) {
    const last = route.length > 1 ? route[route.length - 2] : start;
    const end = route[route.length - 1];
    tail = reach(to, Math.atan2(last[1] - end[1], last[0] - end[0]));
  }
  for (const [a, b] of trimmed(start, route, head, tail))
    ribbon(mesh, pen, a, b, color, pen.line, { dashed });
}

/** A soldier's marker: a plain ring (the unit's marker carries the facing),
 *  with a cover tier's pip (in `pipColor`) in its middle when he has cover
 *  and the order marks show. */
function soldierMark(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  pipColor: Rgba | null,
  color: Rgba,
  width = pen.soldier,
) {
  ring(mesh, pen, c, SOLDIER_R, color, { width });
  if (pipColor) pip(mesh, pen, c, pipColor);
}

/** The way a moving vehicle travels, as two small chevrons behind its marker
 *  (and its hull), painted on the ground like it: pointing along the marker's facing on a forward move, and
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
  radius: number,
  facing: number,
  reverse: boolean,
  color: Rgba,
) {
  const back = facing + Math.PI;
  const travel = reverse ? back : facing;
  const { cycles_per_s, amplitude } = pen.style.march;
  // Nearest the marker first; the pulse reaches the one ahead in the
  // direction of travel later.
  const start = radius + CHEVRONS_GAP_M;
  const spots = [start, start + 1.3];
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
  size = CHEVRON_M,
  line = pen.line,
) {
  const [length, spread] = size;
  const width = line;
  const at = (p: P2) => [p[0], p[1], pen.z(p[0], p[1])] as const;
  const fx = Math.cos(bearing),
    fy = Math.sin(bearing);
  for (const side of [-1, 1]) {
    const end: P2 = [
      tip[0] - fx * length - fy * side * (spread / 2),
      tip[1] - fy * length + fx * side * (spread / 2),
    ];
    const [dx, dy] = [tip[0] - end[0], tip[1] - end[1]];
    const n = Math.hypot(dx, dy) || 1;
    const [nx, ny] = [(-dy / n) * (width / 2), (dx / n) * (width / 2)];
    // Keep each arm at a constant width, then use a short mitered point for
    // the join. Letting both full-width strips run all the way to `tip`
    // creates the square artifact visible in a close crop.
    const near: P2 = [tip[0] - (dx / n) * width * 0.55, tip[1] - (dy / n) * width * 0.55];
    const a = at([end[0] - nx, end[1] - ny]);
    const b = at([end[0] + nx, end[1] + ny]);
    const c = at([near[0] - nx, near[1] - ny]);
    const d = at([near[0] + nx, near[1] + ny]);
    const point = at(tip);
    mesh.vertex(a, normal, color).vertex(b, normal, color).vertex(d, normal, color);
    mesh.vertex(a, normal, color).vertex(d, normal, color).vertex(c, normal, color);
    mesh.vertex(c, normal, color).vertex(d, normal, color).vertex(point, normal, color);
  }
}

/** The circle a squad stands in now: round its soldiers, from its position. */
function squadNow(u: Pick<OrderView, "members">, here: P2): number {
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

/** A unit's circle on the ground, with its arrowhead's facing (null: none). */
export interface UnitCircle {
  c: P2;
  r: number;
  facing: number | null;
}

/** The circular interior, excluding the facing arrowhead. */
export function circleContains(circle: UnitCircle, point: readonly [number, number]): boolean {
  return vec2.squaredDistance(circle.c as Vec2, point as Vec2) <= circle.r * circle.r;
}

/** The circle the orders draw round a unit where it stands, or null where
 *  they draw none, one marker for every ground unit: a vehicle's under its
 *  hull (moving, selected or revealed; an aircraft's always, on the ground
 *  under it), a moving squad's round its soldiers,
 *  and, selected or revealed, a holding squad's area ring round its anchor,
 *  each with its arrowhead at the unit's facing (a moving squad's heading, a
 *  holding one's facing). A garrisoned squad's circle, moving (leaving on an
 *  order), selected or revealed, encloses its building, with no arrowhead. Lines to or from the unit (routes, the range ruler) meet this
 *  circle at its border (`circleReach`). */
export function unitCircle(
  u: Pick<
    OrderView,
    | "position"
    | "goal"
    | "state"
    | "members"
    | "area"
    | "hullHalfLength"
    | "building"
    | "yaw"
    | "selected"
    | "reveal"
    | "aircraft"
  >,
  style: Pick<
    OrderStyle,
    "area_draw_scale" | "vehicle_marker_margin_m" | "building_marker_margin_m"
  >,
): UnitCircle | null {
  const shown = (u.reveal ?? 0) > 0;
  const here: P2 = [u.position[0], u.position[1]];
  const moving = !!u.goal && u.state !== "route_blocked";
  if (u.building) {
    const { center, half } = u.building;
    const r = Math.hypot(half[0], half[1]) + style.building_marker_margin_m;
    return moving || shown || u.selected ? { c: center, r, facing: null } : null;
  }
  // A squad's rings are drawn at `area_draw_scale` of their radius (the
  // movement area itself is the simulation's).
  const drawn = (radius: number) => radius * style.area_draw_scale;
  if (u.members.length === 0)
    return moving || shown || u.selected || u.aircraft
      ? { c: here, r: u.hullHalfLength + style.vehicle_marker_margin_m, facing: u.yaw }
      : null;
  if (moving) return { c: here, r: drawn(squadNow(u, here)), facing: u.yaw };
  return (shown || u.selected) && !u.goal && u.area
    ? { c: u.area.anchor, r: drawn(u.area.radius), facing: u.yaw }
    : null;
}

/** The circle a unit's order ends in: a squad's area ring round its new
 *  anchor, a vehicle's marker at its goal, pointing the final facing. */
export function destinationCircle(
  u: Pick<OrderView, "goal" | "area" | "members" | "hullHalfLength" | "finalFacing">,
  goal: P2,
  style: Pick<OrderStyle, "area_draw_scale" | "vehicle_marker_margin_m">,
): UnitCircle {
  return u.members.length > 0 && u.area
    ? { c: u.area.anchor, r: u.area.radius * style.area_draw_scale, facing: u.finalFacing }
    : { c: goal, r: u.hullHalfLength + style.vehicle_marker_margin_m, facing: u.finalFacing };
}

function orderPen(z: SurfaceHeight, style: OrderStyle, stroke: StrokeWidth): Pen {
  return {
    style,
    z,
    line: stroke(style.line_px),
    stroke: stroke(style.mark_px),
    soldier: stroke(style.soldier_mark_px),
    soldierSelected: stroke(style.soldier_line_px),
    drop: stroke(style.drop_line_px),
  };
}

export interface DestinationMarker extends UnitCircle {
  placed: boolean;
  opacity: number;
}

/** A destination and its facing while the player holds a right-drag. */
export function buildDestinationPreview(
  marks: readonly DestinationMarker[],
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
) {
  const mesh = new MeshBuilder();
  const pen = orderPen(z, style, stroke);
  for (const mark of marks) {
    if (!mark.placed) continue;
    const color = glowing(style.color, style.glow.order);
    circleMarker(mesh, pen, mark, fadeAlpha(color, mark.opacity));
  }
  return mesh.build();
}

/** The reinforcement entry marker: three inward chevrons stacked along the
 *  battle direction. Their bright fill pulses in sequence, so the glow reads
 *  as movement into the map rather than as a static button-shaped glyph. */
export function buildDeploymentMarker(
  destination: readonly [number, number],
  facing: number,
  z: SurfaceHeight,
  style: OrderStyle,
  markerColor: readonly [number, number, number],
  { stroke }: OrderOverlayOptions,
): Mesh {
  const mesh = new MeshBuilder();
  const pen = orderPen(z, style, stroke);
  const color = glowing([...markerColor, 1], style.glow.selected);
  const { cycles_per_s, amplitude } = style.march;
  const offsets = [-9, 0, 9];
  const outline: Rgba = [markerColor[0] * 0.45, markerColor[1] * 0.55, markerColor[2] * 0.6, 1];
  offsets.forEach((offset, i) => {
    const tip = along(destination, facing, offset);
    // A dark, slightly oversized underlay leaves the crisp outlined edge seen
    // in the reference around each painted chevron.
    marchChevron(mesh, pen, tip, facing, outline, [0, 0, 0], [7.8, 12.5], pen.line * 2.6);
    // The pulse advances from the outer chevron toward the spawn point and
    // back again. At any instant one chevron is visibly bright.
    marchChevron(
      mesh,
      pen,
      tip,
      facing,
      i === 1 ? color : fadeAlpha(color, 0.42),
      [i === 1 ? 0 : i === 0 ? 1 / 3 : 2 / 3, cycles_per_s, Math.min(amplitude, 0.32)],
      [6.8, 10.8],
      i === 1 ? pen.line * 3.2 : pen.line * 1.25,
    );
  });
  return mesh.build();
}

/** A display-space copy of a deployment marker. Ground paint carries its
 * marching phase in the vertex normal; the un-fogged overlay needs a real
 * horizontal normal so the overlay shader does not interpret that phase as
 * lighting. The ground copy remains the animated source. */
export function deploymentOverlayMarker(marker: Mesh): Mesh {
  const out = new Float32Array(marker);
  for (let i = 0; i < out.length; i += VERTEX_FLOATS) {
    out[i + 3] = 0;
    out[i + 4] = 0;
    out[i + 5] = 1;
  }
  return out;
}

export function buildOrderOverlay(
  units: readonly OrderView[],
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
): WorldMeshes {
  const pen = orderPen(z, style, stroke);
  // Every order mark is paint, still or marching (the travel chevrons),
  // each colour glowing past its full value. Soldiers' markers draw last, so
  // no other unit's ring or route crossing one covers his cover pip.
  const paint = new MeshBuilder();
  const soldiers = new MeshBuilder();
  const animated = new MeshBuilder();
  // Aircraft drop lines: overlay, the one mark standing in the air.
  const lines = new MeshBuilder();
  const order = glowing(style.color, style.glow.order);
  const selected = glowing(style.selected, style.glow.selected);
  const cannot = glowing(style.blocked, style.glow.order);
  for (const u of units) {
    // Its order marks (the Space view) at their opacity, or none; the
    // selection's own markers either way.
    const reveal = u.reveal ?? 0;
    const shown = reveal > 0;
    if (!shown && !u.selected && !u.aircraft) continue;
    const fade = (c: Rgba): Rgba => fadeAlpha(c, reveal);
    const color = fade(order);
    const current = fade(fadeAlpha(order, style.current_alpha));
    const queued = fade(fadeAlpha(order, style.queued_alpha));
    const blockedColor = fade(cannot);
    const pipOf = (cover: CoverTierName | null | undefined) =>
      shown && cover ? fade(style.cover[cover]) : null;
    const moving = !!u.goal && u.state !== "route_blocked";
    const squad = u.members.length > 0;
    // The unit's marker: its circle and its soldiers' markers are one mark,
    // shown and hidden together (`unitCircle` decides when: moving, selected
    // or revealed), in the selection's colours when selected. The circle is a
    // squad's area ring round its anchor while it holds, the ring round its
    // soldiers while it moves, the ring round its building while it holds
    // one, or a vehicle's marker under its hull.
    const circle = unitCircle(u, style);
    // A squad with no circle to draw (no area) still shows its soldiers'
    // markers when the unit's marker would.
    if (circle || shown || u.selected) {
      if (circle && u.aircraft)
        // An aircraft's marker shows whatever the reveal: at the current
        // markers' alpha, brighter only while a shown order moves it.
        aircraftMarker(
          paint,
          lines,
          pen,
          u.position,
          circle,
          u.selected
            ? selected
            : fadeAlpha(
                order,
                moving ? Math.max(reveal, style.current_alpha) : style.current_alpha,
              ),
        );
      else if (circle)
        circleMarker(paint, pen, circle, u.selected ? selected : moving ? color : current);
      // Each soldier's marker, with his cover now when shown.
      const mark = u.selected ? selected : current;
      u.members.forEach((m, k) =>
        soldierMark(
          soldiers,
          pen,
          [m[0], m[1]],
          pipOf(u.memberOrders[k]?.coverNow),
          mark,
          u.selected ? pen.soldierSelected : pen.soldier,
        ),
      );
    }
    // A moving vehicle's travel shows as chevrons behind its hull.
    if (!squad && moving && circle)
      travelChevrons(
        animated,
        pen,
        circle.c,
        circle.r,
        u.finalFacing,
        u.direction === "reverse",
        u.selected ? selected : current,
      );
    // The rest are order marks only.
    if (!shown) continue;
    if (!u.goal) {
      // A holding squad's soldiers walking to their posts (cover, a step
      // out): each post is a marker only. An order is the unit's, one route
      // for the unit; no line ever runs from a soldier.
      u.memberOrders.forEach((m, k) => {
        const p = u.members[k];
        if (p && Math.hypot(m.spot[0] - p[0], m.spot[1] - p[1]) > 2 * SOLDIER_R)
          soldierMark(soldiers, pen, m.spot, pipOf(m.coverThere), color);
      });
      continue;
    }
    const blocked = u.state === "route_blocked";
    // The destination's circle marker: a squad's area ring (an order moves
    // its area, so a moving squad's is already the destination's), with
    // each soldier's spot in it; a vehicle's marker. Both point the final
    // facing.
    const dest: UnitCircle | null = blocked ? null : destinationCircle(u, u.goal, style);
    routeBetween(paint, pen, circle, u.route, dest, color);
    if (blocked) {
      const warning: UnitCircle = { c: u.goal, r: BLOCKED_R, facing: null };
      routeBetween(paint, pen, circle, [u.goal], warning, blockedColor, { dashed: true });
      crossMark(paint, pen, u.goal, 6, blockedColor);
      ring(paint, pen, u.goal, BLOCKED_R, blockedColor, { width: pen.line });
    } else if (dest) {
      circleMarker(paint, pen, dest, color);
      if (squad)
        for (const m of u.memberOrders)
          soldierMark(soldiers, pen, m.spot, pipOf(m.coverThere), color);
    }
    let prev: UnitCircle | null = dest;
    for (const q of u.queue) {
      const next: UnitCircle = { c: q, r: QUEUED_R, facing: null };
      routeBetween(paint, pen, prev, [q], next, queued, { dashed: true });
      ring(paint, pen, q, QUEUED_R, queued);
      prev = next;
    }
    // Its ghost over where its orders end, joined to that end's marker.
    if (u.ghostAloft) {
      const end = u.queue.at(-1) ?? u.goal;
      const top = [end[0], end[1], pen.z(end[0], end[1]) + u.ghostAloft] as const;
      dropLine(lines, pen, top, blocked ? blockedColor : u.queue.length ? queued : color);
    }
  }
  return {
    ...paintOnly(concatMeshes([paint.build(), soldiers.build()]), animated.build()),
    opaque: lines.build(),
  };
}

/** The side's view of an aircraft that isn't its own (an identified enemy):
 *  where it is, its facing and its hull's half length. */
export interface AircraftView {
  position: readonly [number, number, number];
  yaw: number;
  hullHalfLength: number;
}

/** An identified aircraft's marker circle, on the ground under it: its
 *  hull's half length plus `vehicle_marker_margin_m`, as an own vehicle's. */
export function aircraftCircle(
  a: AircraftView,
  style: Pick<OrderStyle, "vehicle_marker_margin_m">,
): UnitCircle {
  return {
    c: [a.position[0], a.position[1]],
    r: a.hullHalfLength + style.vehicle_marker_margin_m,
    facing: a.yaw,
  };
}

/** Every one of `aircraft` as its ground marker and drop line
 *  (`aircraftMarker`), in `color` (its role's, glowing by the orders'). */
export function buildAircraftMarks(
  aircraft: readonly AircraftView[],
  color: Rgba,
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
): WorldMeshes {
  const pen = orderPen(z, style, stroke);
  const paint = new MeshBuilder();
  const lines = new MeshBuilder();
  const glow = glowing(color, style.glow.order);
  for (const a of aircraft)
    aircraftMarker(paint, lines, pen, a.position, aircraftCircle(a, style), glow);
  return { ...paintOnly(paint.build()), opaque: lines.build() };
}
