// The pen every order mark is drawn with: one draw's widths at the camera's
// zoom, and the strokes the marks are made of (a ribbon, a ring, a filled
// rim arrowhead, a unit's circle marker), each lying on the ground.
import { vec2, type Vec2 } from "math";
import {
  glowing,
  groundAnnulus,
  groundRing,
  groundStrip,
  type MeshBuilder,
  type Rgba,
} from "./mesh";
import type { OrderStyle, SurfaceHeight } from "./orderOverlay";
import type { StrokeWidth } from "./strokeWidth";
import { circleReach, markerHead, type UnitCircle } from "./unitCircle";

type P2 = readonly [number, number];

/** One draw's widths and style. */
export interface Pen {
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
export function ribbon(
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
export function ring(
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
export function pip(mesh: MeshBuilder, pen: Pen, c: P2, color: Rgba) {
  groundAnnulus(mesh, c, 0, pen.style.cover_pip_m, {
    z: pen.z,
    segments: 14,
    colorIn: glowing(color, pen.style.cover_glow),
  });
}

export function crossMark(mesh: MeshBuilder, pen: Pen, c: P2, size: number, color: Rgba) {
  const h = size / 2;
  ribbon(mesh, pen, [c[0] - h, c[1] - h], [c[0] + h, c[1] + h], color, pen.line);
  ribbon(mesh, pen, [c[0] - h, c[1] + h], [c[0] + h, c[1] - h], color, pen.line);
}

/** A point `d` metres from `c` along `bearing`. */
export const along = (c: P2, bearing: number, d: number): P2 => [
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
export function circleMarker(mesh: MeshBuilder, pen: Pen, m: UnitCircle, color: Rgba) {
  unitMarker(mesh, pen, m.c, m.facing, m.r, color, { width: pen.line });
}

/** A travel chevron: length along the travel and spread across it. */
export const CHEVRON_M: readonly [number, number] = [0.9, 1.7];

/** A route from the unit's own circle to a destination circle, clipped at
 *  both: it leaves from the first's rim and ends at the second's, at the
 *  arrowhead's tip where it leaves or arrives along the facing. Either circle may be absent
 *  (not drawn): the route runs to the centre there. */
export function routeBetween(
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

/** One open chevron whose vertices carry `normal` for the overlay's march. */
export function marchChevron(
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

/** An unload chevron: its length over its circle's radius, and at least
 *  this many route widths, so it still reads when the view pulls out; its
 *  spread over its length; its gap from the rim, and from the chevron
 *  inside it, in route widths. */
const UNLOAD_CHEVRON = 0.3;
const UNLOAD_CHEVRON_LINES = 2;
const UNLOAD_SPREAD = 1.6;
const UNLOAD_GAP_LINES = 1;
const STILL: readonly [number, number, number] = [0, 0, 0];

/** The unload mark on a destination's circle: off each side of `bearing`
 *  (the way it ends up facing), two chevrons pointing out the way its
 *  passengers get out, the travel chevrons' form for going that way. */
export function unloadMark(
  mesh: MeshBuilder,
  pen: Pen,
  c: UnitCircle,
  bearing: number,
  color: Rgba,
) {
  const length = Math.max(c.r * UNLOAD_CHEVRON, pen.line * UNLOAD_CHEVRON_LINES);
  const gap = pen.line * UNLOAD_GAP_LINES;
  for (const side of [-1, 1]) {
    const out = bearing + (side * Math.PI) / 2;
    for (const k of [1, 2]) {
      const tip = along(c.c, out, c.r + k * (gap + length));
      marchChevron(mesh, pen, tip, out, color, STILL, [length, length * UNLOAD_SPREAD]);
    }
  }
}

/** The pen for one draw: the style's widths at the camera's zoom. */
export function orderPen(z: SurfaceHeight, style: OrderStyle, stroke: StrokeWidth): Pen {
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
