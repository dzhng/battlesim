// The sight lobe on the ground, a debug overlay: for each own unit, the
// outline of how far its eyes reach in every direction (the published shape
// swept around its forward bearing), a ring at the unit, a heading arrow
// and a tinted fill for directional (vehicle) sight so the lobe reads as an
// area. Directional lobes are cyan and bold; even 360° (infantry) sight is a
// thin pale outline, so the shaped lobes stand out. Built only from the
// observing side's own-unit view at the published tick, never interpolated.
import { MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { Vec3 } from "math";

/** Sight multipliers dead ahead, abeam and astern. */
export interface SightShape {
  front: number;
  side: number;
  rear: number;
}

/** One unit's published sight (the observation's `sight`). */
export interface SightLobe {
  eyes: readonly (readonly [number, number, number])[];
  /** World bearing it looks along, counter-clockwise from +X. */
  forward: number;
  shape: SightShape;
  /** Ground range in the open, before the shape. */
  range: number;
}

/**
 * The shape's multiplier `off` radians from forward:
 * `side·sin² + (front or rear)·cos²`. Mirrors `sim::sight::multiplier`, the
 * rule's one owner; spotting and the fog sweep use that one.
 */
export function sightMultiplier(shape: SightShape, off: number): number {
  const c = Math.cos(off);
  return shape.side * (1 - c * c) + (c >= 0 ? shape.front : shape.rear) * c * c;
}

type Style = { edge: Rgba; width: number; fill: Rgba | null };

const SIGHT_STYLES: Record<"directional" | "even", Style> = {
  directional: { edge: [0.3, 0.95, 1.0, 1], width: 3, fill: [0.3, 0.95, 1.0, 0.12] },
  even: { edge: [0.9, 0.92, 0.95, 1], width: 1.5, fill: null },
};
const HEADING_COLOR: Rgba = [1.0, 0.9, 0.3, 1];

const SEGMENTS = 128;
const MARKER_RADIUS_M = 9;
const ARROW_LENGTH_M = 45;
const ARROW_HEAD_M = 12;
const ARROW_WIDTH_M = 2.5;
/** Above route ribbons, so the lobe is never hidden. */
const LIFT_M = 1.2;
const FILL_LIFT_M = 0.5;

const isEven = (s: SightShape) => s.front === s.side && s.side === s.rear;

/** Outline, marker and heading (opaque) and fill (translucent) of every lobe. */
export function buildSightOverlay(lobes: readonly SightLobe[], z: SurfaceHeight) {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  const ground = (x: number, y: number, lift: number): Readonly<Vec3> => [x, y, z(x, y) + lift];
  for (const lobe of lobes) {
    if (lobe.eyes.length === 0) continue;
    const style = isEven(lobe.shape) ? SIGHT_STYLES.even : SIGHT_STYLES.directional;
    // A garrison's eyes ring its building: draw the lobe from their centre.
    const cx = lobe.eyes.reduce((s, e) => s + e[0], 0) / lobe.eyes.length;
    const cy = lobe.eyes.reduce((s, e) => s + e[1], 0) / lobe.eyes.length;
    const at = (a: number, r: number, lift: number) =>
      ground(cx + Math.cos(a) * r, cy + Math.sin(a) * r, lift);
    const reach = (a: number) => lobe.range * sightMultiplier(lobe.shape, a - lobe.forward);
    for (let k = 0; k < SEGMENTS; k++) {
      const a0 = (k / SEGMENTS) * Math.PI * 2;
      const a1 = ((k + 1) / SEGMENTS) * Math.PI * 2;
      const [r0, r1] = [reach(a0), reach(a1)];
      opaque.quad(
        at(a0, r0 - style.width, LIFT_M),
        at(a0, r0, LIFT_M),
        at(a1, r1, LIFT_M),
        at(a1, r1 - style.width, LIFT_M),
        style.edge,
      );
      // The unit marker: a ring the lobe's colour, so each lobe points home.
      opaque.quad(
        at(a0, MARKER_RADIUS_M - style.width, LIFT_M),
        at(a0, MARKER_RADIUS_M, LIFT_M),
        at(a1, MARKER_RADIUS_M, LIFT_M),
        at(a1, MARKER_RADIUS_M - style.width, LIFT_M),
        style.edge,
      );
      if (style.fill) {
        // Fan wedges, split along the radius so the fill follows the ground.
        const steps = 4;
        for (let j = 0; j < steps; j++) {
          const [f0, f1] = [j / steps, (j + 1) / steps];
          translucent.quad(
            at(a0, (r0 - style.width) * f0, FILL_LIFT_M),
            at(a0, (r0 - style.width) * f1, FILL_LIFT_M),
            at(a1, (r1 - style.width) * f1, FILL_LIFT_M),
            at(a1, (r1 - style.width) * f0, FILL_LIFT_M),
            style.fill,
          );
        }
      }
    }
    // The heading: a short arrow from the marker along the forward bearing.
    // Even sight has no facing that matters, so it gets none.
    if (!style.fill) continue;
    const f = lobe.forward;
    const side = (d: number, off: number, lift: number) =>
      ground(
        cx + Math.cos(f) * d - Math.sin(f) * off,
        cy + Math.sin(f) * d + Math.cos(f) * off,
        lift,
      );
    const w = ARROW_WIDTH_M / 2;
    const shaft = ARROW_LENGTH_M - ARROW_HEAD_M;
    const lift = LIFT_M + 0.05;
    opaque.quad(
      side(MARKER_RADIUS_M, -w, lift),
      side(shaft, -w, lift),
      side(shaft, w, lift),
      side(MARKER_RADIUS_M, w, lift),
      HEADING_COLOR,
    );
    opaque.triangle(
      side(shaft, -ARROW_HEAD_M / 2, lift),
      side(ARROW_LENGTH_M, 0, lift),
      side(shaft, ARROW_HEAD_M / 2, lift),
      HEADING_COLOR,
    );
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
