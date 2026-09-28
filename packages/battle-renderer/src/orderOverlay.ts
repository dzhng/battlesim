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
import { concatMeshes, groundAnnulus, isRgba, MeshBuilder, type Rgba } from "./mesh";
import type { WorldMeshes } from "./scene";

export type CoverTierName = "light" | "medium" | "heavy";

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
  /** A selected unit's own circle and its arrowhead (a squad's round its
   *  soldiers, a vehicle's under its hull): true colour, composited after
   *  tone mapping (overlay), so it pops as nothing lit in the world can. */
  selected: Rgba;
  /** The marker under each soldier of a selected squad. */
  soldier_selected: Rgba;
  /** Where each colour role draws: painted in the world ("world": lit,
   *  fogged, under smoke) or composited after tone mapping ("overlay": true
   *  colour, over smoke), still hidden by bodies either way. */
  layers: Record<ColourRole, MarkLayer>;
  /** That soldier paint glows this much brighter than its colour's full
   *  value (up to 2, the paint's range). */
  selected_glow: number;

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
}

/** The colour roles the orders draw in: every order mark (routes,
 *  destination and area rings, spots, arrowheads, the current and queued
 *  marks); a selected unit's own circle; a selected squad's soldiers'
 *  markers. */
export type ColourRole = "order" | "selected" | "soldier";
export type MarkLayer = "world" | "overlay";
const ROLES: readonly ColourRole[] = ["order", "selected", "soldier"];

/** `presentation.overlay.orders` as authored: the colour roles come from the
 *  scheme `scheme` names in `schemes` (a fixture switch, one scheme per
 *  look), each role a colour and a layer. */
export interface AuthoredOrderStyle extends Omit<
  OrderStyle,
  "color" | "selected" | "soldier_selected" | "layers"
> {
  scheme: string;
  schemes: Record<string, Record<ColourRole, { color: Rgba; layer: MarkLayer }>>;
}

/** The authored style with its scheme's roles in place. */
export function resolveOrderScheme(authored: AuthoredOrderStyle): OrderStyle {
  const scheme = authored.schemes?.[authored.scheme];
  if (
    !scheme ||
    !ROLES.every((r) => scheme[r] && (scheme[r].layer === "world" || scheme[r].layer === "overlay"))
  )
    throw new Error(
      `presentation.overlay.orders: scheme "${authored.scheme}" must name one of schemes, each with order, selected and soldier as { color, layer: "world" | "overlay" }`,
    );
  const { scheme: _, schemes: __, ...rest } = authored;
  return {
    ...rest,
    color: scheme.order.color,
    selected: scheme.selected.color,
    soldier_selected: scheme.soldier.color,
    layers: {
      order: scheme.order.layer,
      selected: scheme.selected.layer,
      soldier: scheme.soldier.layer,
    },
  };
}

export function validateOrderStyle(style: OrderStyle): OrderStyle {
  const unit = (v: number) => v > 0 && v <= 1;
  const ok =
    style.line_px > 0 &&
    style.mark_px > 0 &&
    style.soldier_mark_px > 0 &&
    style.soldier_line_px > 0 &&
    style.min_line_m > 0 &&
    style.lift_m >= 0 &&
    style.cover_pip_m > 0 &&
    unit(style.queued_alpha) &&
    unit(style.current_alpha) &&
    style.march?.cycles_per_s >= 0 &&
    style.march?.amplitude >= 0 &&
    style.march?.amplitude <= 1 &&
    unit(style.area_draw_scale) &&
    style.selected_glow >= 1 &&
    style.selected_glow <= 2 &&
    style.cover_glow >= 1 &&
    style.cover_glow <= 2 &&
    style.vehicle_marker_margin_m > 0 &&
    (["light", "medium", "heavy"] as const).every((k) => isRgba(style.cover?.[k])) &&
    [style.color, style.blocked, style.selected, style.soldier_selected].every(isRgba) &&
    ROLES.every((r) => style.layers?.[r] === "world" || style.layers?.[r] === "overlay");
  if (!ok)
    throw new Error(
      `presentation.overlay.orders: positive widths, lift_m ≥ 0, alphas in (0, 1], rgba color, blocked, selected and cover.{light, medium, heavy}, march.{cycles_per_s ≥ 0, amplitude in [0, 1]}, area_draw_scale in (0, 1], selected_glow and cover_glow in [1, 2], vehicle_marker_margin_m > 0`,
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
  /** The bearing it faces now (its published yaw). */
  yaw: number;
  /** "reverse" on a reverse move (Q31). */
  direction: string | null;
  /** A squad's area round its anchor (27d): the disc its soldiers take
   *  their spots and fight in, which only an order moves; null for a
   *  vehicle. */
  area: { anchor: readonly [number, number]; radius: number } | null;
  /** A vehicle's hull half-length (its footprint's, `physics`); 0 for a
   *  squad. Its marker circle is this plus `vehicle_marker_margin_m`. */
  hullHalfLength: number;
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
/** A queued waypoint's ring. */
const QUEUED_R = 2.8;
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
  /** A soldier's marker's width, and a selected soldier's. */
  soldier: number;
  soldierSelected: number;
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

/** A cover pip in the middle of a soldier's marker. */
function pip(mesh: MeshBuilder, pen: Pen, c: P2, color: Rgba) {
  const g = pen.style.cover_glow;
  const bright: Rgba = [color[0] * g, color[1] * g, color[2] * g, color[3]];
  groundAnnulus(mesh, c, 0, pen.style.cover_pip_m, {
    z: pen.z,
    lift: pen.style.lift_m + 2 * STACK_M,
    segments: 14,
    colorIn: bright,
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
const markerReach = (r: number) => r + markerHead(r);

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
  rimArrowhead(mesh, pen, c, bearing, r, markerHead(r), color, lift + STACK_M);
}

/** A unit's circle marker: where it stands or where it is going. */
interface Circle {
  c: P2;
  r: number;
  facing: number;
}

/** A circle marker: the circle with the filled arrowhead on its rim at its
 *  facing (`unitMarker`), in the route's line weight. */
function circleMarker(mesh: MeshBuilder, pen: Pen, m: Circle, color: Rgba) {
  unitMarker(mesh, pen, m.c, m.facing, m.r, color, { width: pen.line });
}

/** A route from the unit's own circle to a destination circle, clipped at
 *  both: it leaves from the first's rim and ends at the second's, at the
 *  arrowhead's tip where it leaves or arrives along the facing. Either circle may be absent
 *  (not drawn): the route runs to the centre there. */
function routeBetween(
  mesh: MeshBuilder,
  pen: Pen,
  from: Circle | null,
  route: readonly P2[],
  to: Circle | null,
  color: Rgba,
  { dashed = false } = {},
) {
  const start = from?.c ?? route[0];
  if (!start || route.length === 0) return;
  // Where it leaves or arrives along a circle's facing, it clears the
  // circle's arrowhead (to its tip), never running over it.
  const reach = (c: Circle, bearing: number) =>
    Math.cos(bearing - c.facing) > Math.cos(Math.PI / 3) ? markerReach(c.r) : c.r;
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

/** A soldier's marker, with the cover tier's pip in its middle when he has
 *  cover (Space). */
function soldierMark(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  facing: number,
  cover: CoverTierName | null,
  color: Rgba,
  width = pen.soldier,
) {
  unitMarker(mesh, pen, c, facing, SOLDIER_R, color, { width });
  if (cover) pip(mesh, pen, c, pen.style.cover[cover]);
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
    soldier: Math.max(style.min_line_m, style.soldier_mark_px * metresPerPx),
    soldierSelected: Math.max(style.min_line_m, style.soldier_line_px * metresPerPx),
  };
  const current = withAlpha(style.color, style.current_alpha);
  // Each colour role draws in its scheme's layer: painted in the world, or
  // overlay after tone mapping (depth-tested, so bodies still hide it).
  const world = { opaque: new MeshBuilder(), translucent: new MeshBuilder() };
  const over = { opaque: new MeshBuilder(), translucent: new MeshBuilder() };
  const meshOf = (role: ColourRole, faint = false) =>
    (style.layers[role] === "overlay" ? over : world)[faint ? "translucent" : "opaque"];
  // A selection painted in the world glows past its colour's full value;
  // after tone mapping its colour is exact.
  const glowing = (role: ColourRole, c: Rgba): Rgba => {
    const g = style.layers[role] === "world" ? style.selected_glow : 1;
    return [c[0] * g, c[1] * g, c[2] * g, c[3]];
  };
  const selectedColour = glowing("selected", style.selected);
  const soldierSelected = glowing("soldier", style.soldier_selected);
  const queued = withAlpha(style.color, style.queued_alpha);
  // Every order mark draws in the order role's layer; the warning (a
  // blocked route) and the travel chevrons are always paint.
  const opaque = meshOf("order");
  const translucent = meshOf("order", true);
  const animated = new MeshBuilder();
  for (const u of units) {
    const here: P2 = [u.position[0], u.position[1]];
    const moving = !!u.goal && u.state !== "route_blocked";
    const reverse = u.direction === "reverse";
    const f = u.finalFacing;
    const squad = u.members.length > 0;
    const vehicleR = u.hullHalfLength + style.vehicle_marker_margin_m;
    // A squad's rings are drawn at `area_draw_scale` of their radius (the
    // movement area itself is 27d's).
    const drawn = (radius: number) => radius * style.area_draw_scale;
    // The unit's own circle marker, one style for squads and vehicles: round
    // a squad's soldiers, or a vehicle's marker painted under its hull, its
    // arrowhead at the facing it has now. Drawn under a moving unit, and
    // under a vehicle when selected or with Space; yellow when selected.
    const own: Circle | null =
      moving || (!squad && (all || u.selected))
        ? { c: here, r: squad ? drawn(squadNow(u, here)) : vehicleR, facing: u.yaw }
        : null;
    if (own)
      circleMarker(
        u.selected ? meshOf("selected") : moving ? opaque : translucent,
        pen,
        own,
        u.selected ? selectedColour : moving ? style.color : current,
      );
    // Under each soldier (with his cover now), where he is: always for the
    // selection, in its colour; for every unit with Space. A moving
    // vehicle's travel shows as chevrons behind its hull.
    if (all || u.selected) {
      const mark = u.selected ? soldierSelected : current;
      const mesh = u.selected ? meshOf("soldier") : translucent;
      if (!squad && moving) travelChevrons(animated, pen, here, vehicleR, f, reverse, mark);
      u.members.forEach((m, k) =>
        soldierMark(
          mesh,
          pen,
          [m[0], m[1]],
          f,
          all ? (u.memberOrders[k]?.coverNow ?? null) : null,
          mark,
          u.selected ? pen.soldierSelected : pen.soldier,
        ),
      );
    }
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
    // The destination's circle marker: a squad's area ring (an order moves
    // its area, so a moving squad's is already the destination's), with
    // each soldier's spot in it; a vehicle's marker. Both point the final
    // facing.
    const dest: Circle | null = blocked
      ? null
      : {
          c: squad && u.area ? u.area.anchor : u.goal,
          r: squad && u.area ? drawn(u.area.radius) : vehicleR,
          facing: f,
        };
    routeBetween(opaque, pen, own, u.route, dest, style.color);
    if (blocked) {
      ribbon(world.opaque, pen, here, u.goal, style.blocked, pen.line, { dashed: true });
      crossMark(world.opaque, pen, u.goal, 6, style.blocked);
      ring(world.opaque, pen, u.goal, 4.5, style.blocked, { width: pen.line });
    } else if (dest) {
      circleMarker(opaque, pen, dest, style.color);
      if (squad)
        for (const m of u.memberOrders)
          soldierMark(opaque, pen, m.spot, f, all ? m.coverThere : null, style.color);
    }
    // Waiting for the way ahead to clear: a broken ring round the unit.
    if (u.state === "waiting")
      ring(opaque, pen, here, vehicleR + CHEVRONS_GAP_M + 3, style.color, {
        width: pen.line,
        dashed: true,
      });
    let prev: Circle | null = dest;
    for (const q of u.queue) {
      const next: Circle = { c: q, r: QUEUED_R, facing: 0 };
      routeBetween(translucent, pen, prev, [q], next, queued, { dashed: true });
      ring(translucent, pen, q, QUEUED_R, queued);
      prev = next;
    }
  }
  // The overlay roles' marks, and the paint (`frame/paintedMarks.ts`).
  return {
    opaque: over.opaque.build(),
    translucent: over.translucent.build(),
    painted: concatMeshes([world.opaque.build(), world.translucent.build()]),
    paintedMarching: animated.build(),
  };
}
