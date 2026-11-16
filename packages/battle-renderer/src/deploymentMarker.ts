// The reinforcement entry marker: where a side's road meets the playable
// edge, its lane's number painted on the ground and three chevrons marching
// inward past it (lane slots D9). Teammates call lanes by number ("push 2"),
// so the number sits nearest the edge, the way a road is labelled where it
// starts, and the chevrons carry on from it into the map, drawn with the
// chevrons' strokes. Every mark is ground paint in the marching layer, drawn
// before the units' marks so a selection ring lies over it; the number holds
// still and the chevrons pulse.
// The digits are stroke glyphs: a lane is 1, 2 or 3, so there is no font.
import {
  concatMeshes,
  fadeAlpha,
  glowing,
  groundAnnulus,
  groundStrip,
  MeshBuilder,
  unpulsed,
  type Mesh,
  type Rgba,
} from "./mesh";
import type { OrderStyle, SurfaceHeight } from "./orderOverlay";
import { along, marchChevron, orderPen, type Pen } from "./orderPen";
import type { StrokeWidth } from "./strokeWidth";

type P2 = readonly [number, number];
type Rgb = readonly [number, number, number];

/** One of the viewer's side's roads in. */
export interface EntryMarker {
  /** Where the road meets the playable edge. */
  center: P2;
  /** The bearing into the map. */
  facing: number;
  /** Its number on the side, from `sim::entries` (`lanesOf`). */
  lane: number;
}

/** Each digit's strokes: polylines in units of its height, x across (right
 *  as seen from the edge, looking in) and y from its foot (0) to its top (1). */
const arc = (cx: number, cy: number, r: number, fromDeg: number, toDeg: number): P2[] =>
  Array.from({ length: 13 }, (_, k) => {
    const a = ((fromDeg + ((toDeg - fromDeg) * k) / 12) * Math.PI) / 180;
    return [cx + Math.cos(a) * r, cy + Math.sin(a) * r] as const;
  });
const DIGITS: Readonly<Record<number, readonly (readonly P2[])[]>> = {
  1: [
    [
      [-0.2, 0.78],
      [0.04, 1],
      [0.04, 0],
    ],
    [
      [-0.22, 0],
      [0.3, 0],
    ],
  ],
  2: [[...arc(0, 0.7, 0.3, 155, -35), [-0.3, 0], [0.32, 0]]],
  3: [[...arc(0, 0.765, 0.235, 150, -90), ...arc(0, 0.265, 0.265, 90, -150).slice(1)]],
};

/** The digit's height on the ground, in metres. */
const DIGIT_M = 7;
/** Pulled far out, the digit grows only to this many route widths, strokes
 *  and all: a small, whole digit, not a label that reads at every zoom. */
const DIGIT_LINES = 3.6;
/** The digit's height, in route widths, that its stroke weights are drawn
 *  for; a shorter digit thins its strokes in step. */
const STROKED_LINES = 9;
/** The chevrons' strokes, in route widths, which the digit shares: the dark
 *  outline under every chevron, and an outer chevron's fill. */
const OUTLINE_LINES = 2.6;
const FILL_LINES = 1.25;
/** From the digit's top to the first chevron's arms. */
const DIGIT_GAP_M = 3;
/** A chevron: its outline's and its fill's length and spread, and the step
 *  from one to the next. */
const CHEVRON_OUTLINE: readonly [number, number] = [7.8, 12.5];
const CHEVRON_FILL: readonly [number, number] = [6.8, 10.8];
const CHEVRON_STEP_M = 9;

/** The entry `marker` in `markerColor` (the side's accent), its digit and
 *  chevrons sharing one outline colour and stroke weights; all marching
 *  paint, the digit never pulsing. */
export function buildDeploymentMarker(
  marker: EntryMarker,
  z: SurfaceHeight,
  style: OrderStyle,
  markerColor: Rgb,
  { stroke }: { stroke: StrokeWidth },
): { paintedMarching: Mesh } {
  const pen = orderPen(z, style, stroke);
  const { center, facing } = marker;
  const color = glowing([...markerColor, 1], style.glow.selected);
  const outline: Rgba = [markerColor[0] * 0.45, markerColor[1] * 0.55, markerColor[2] * 0.6, 1];
  const height = Math.max(DIGIT_M, pen.line * DIGIT_LINES);
  const weight = pen.line * Math.min(1, height / (pen.line * STROKED_LINES));
  const outlineWidth = weight * OUTLINE_LINES;
  // The digit stands just inside the edge, nearest the border: its
  // outline clear of the border and the HUD pane past it.
  const foot = pen.line * 2 + outlineWidth / 2;
  const painted = new MeshBuilder();
  const glyph = { origin: along(center, facing, foot), facing, height };
  digit(painted, pen, glyph, marker.lane, outline, outlineWidth);
  digit(painted, pen, glyph, marker.lane, color, weight * FILL_LINES);

  const marching = new MeshBuilder();
  const { cycles_per_s, amplitude } = style.march;
  const firstTip = foot + height + DIGIT_GAP_M + CHEVRON_OUTLINE[0] + pen.line * 1.3;
  [0, 1, 2].forEach((i) => {
    const tip = along(center, facing, firstTip + i * CHEVRON_STEP_M);
    // A dark, slightly oversized underlay leaves the crisp outlined edge seen
    // in the reference around each painted chevron.
    marchChevron(
      marching,
      pen,
      tip,
      facing,
      outline,
      [0, 0, 0],
      CHEVRON_OUTLINE,
      pen.line * OUTLINE_LINES,
    );
    // The pulse advances from the outer chevron toward the spawn point and
    // back again. At any instant one chevron is visibly bright.
    marchChevron(
      marching,
      pen,
      tip,
      facing,
      i === 1 ? color : fadeAlpha(color, 0.42),
      [i === 1 ? 0 : i === 0 ? 1 / 3 : 2 / 3, cycles_per_s, Math.min(amplitude, 0.32)],
      CHEVRON_FILL,
      i === 1 ? pen.line * 3.2 : pen.line * FILL_LINES,
    );
  });
  return { paintedMarching: concatMeshes([unpulsed(painted.build()), marching.build()]) };
}

/** `lane`'s digit, upright as seen from the edge looking along `facing`, its
 *  foot at `origin`: each stroke `width` wide with round joins and ends. */
function digit(
  mesh: MeshBuilder,
  pen: Pen,
  { origin, facing, height }: { origin: P2; facing: number; height: number },
  lane: number,
  color: Rgba,
  width: number,
) {
  const [fx, fy] = [Math.cos(facing), Math.sin(facing)];
  const at = ([x, y]: P2): P2 => [
    origin[0] + (fy * x + fx * y) * height,
    origin[1] + (-fx * x + fy * y) * height,
  ];
  for (const line of DIGITS[lane]) {
    const points = line.map(at);
    points.forEach((p, k) => {
      if (k > 0) groundStrip(mesh, points[k - 1], p, width, color, { z: pen.z });
      groundAnnulus(mesh, p, 0, width / 2, { z: pen.z, segments: 12, colorIn: color });
    });
  }
}
