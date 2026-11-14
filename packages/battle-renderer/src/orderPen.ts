// The pen every order mark is drawn with: one draw's widths at the camera's
// zoom, and the strokes the marks are made of (a ribbon, a ring, a filled
// rim arrowhead, a unit's circle marker), each lying on the ground.
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
import { markerHead, type UnitCircle } from "./unitCircle";

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
