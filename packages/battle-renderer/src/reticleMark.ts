// The mark an attack puts on its target: four corner brackets round it, and a
// centre cross where it aims at a point of ground. One mark wherever an
// attack ends: the line from a unit in range, a chase's route, a queued leg.
import type { MeshBuilder, Rgba } from "./mesh";
import { ribbon, type Pen } from "./orderPen";

type P2 = readonly [number, number];

/** A bracket's arm along each side, over the reticle's half side. */
const ARM = 0.45;
/** The ground cross's arm, over the reticle's half side. */
const CROSS = 0.3;

/** A reticle of half side `r` on `c`, square to the world's axes, in the
 *  route's line weight; with `cross`, the ground point's centre cross. */
export function reticleMark(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  r: number,
  color: Rgba,
  { cross = false } = {},
) {
  const arm = r * ARM;
  for (const [sx, sy] of [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ]) {
    const corner: P2 = [c[0] + sx * r, c[1] + sy * r];
    ribbon(mesh, pen, corner, [corner[0] - sx * arm, corner[1]], color, pen.line);
    ribbon(mesh, pen, corner, [corner[0], corner[1] - sy * arm], color, pen.line);
  }
  if (!cross) return;
  const h = r * CROSS;
  ribbon(mesh, pen, [c[0] - h, c[1]], [c[0] + h, c[1]], color, pen.line);
  ribbon(mesh, pen, [c[0], c[1] - h], [c[0], c[1] + h], color, pen.line);
}
