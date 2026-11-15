// @vitest-environment node
// A held building's outline is one line round all its parts: a farmyard's
// barn, house and dovecote read as the one building a squad holds. A small
// part on a corner of the yard keeps its outward wall on the line, rather
// than being cut across, and no part ever stands outside the line.
import { expect, test } from "vitest";
import { heldOutline } from "@packages/battle-renderer/src/garrisonOverlay";

type P = readonly [number, number];
const box = (center: P, half: P, yaw = 0) => ({ center, half, yaw });

/** Whether `p` lies inside or on the closed polygon `ring`, by winding. */
function encloses(ring: readonly P[], [x, y]: P): boolean {
  let winding = 0;
  ring.forEach((a, i) => {
    const b = ring[(i + 1) % ring.length];
    const cross = (b[0] - a[0]) * (y - a[1]) - (x - a[0]) * (b[1] - a[1]);
    if (
      Math.abs(cross) < 1e-9 &&
      Math.min(a[0], b[0]) - 1e-9 <= x &&
      x <= Math.max(a[0], b[0]) + 1e-9
    )
      winding = Number.NaN; // on the line
    else if (a[1] <= y && b[1] > y && cross > 0) winding++;
    else if (a[1] > y && b[1] <= y && cross < 0) winding--;
  });
  return Number.isNaN(winding) || winding !== 0;
}

const corners = ({ center: [x, y], half: [hx, hy] }: ReturnType<typeof box>): P[] => [
  [x - hx, y - hy],
  [x + hx, y - hy],
  [x + hx, y + hy],
  [x - hx, y + hy],
];

test("one part's outline is its own box", () => {
  const house = box([5, 5], [4, 3]);
  expect(heldOutline([house], 0)).toHaveLength(4);
  for (const c of corners(house)) expect(encloses(heldOutline([house], 0), c)).toBe(true);
});

test("a farmyard is one outline that keeps the dovecote's outward wall", () => {
  // A barn across the back, a house down the right and a dovecote in the
  // front left corner whose front wall stands well back from the house's.
  const barn = box([0, 20], [17, 6]);
  const dovecote = box([-10, 0], [3, 3]);
  const house = box([8, -3], [5, 8]);
  const ring = heldOutline([barn, dovecote, house], 0);
  for (const part of [barn, dovecote, house])
    for (const c of corners(part)) expect(encloses(ring, c), `${c}`).toBe(true);
  // The dovecote's front wall, its two front corners in turn, is on the line.
  const at = ([x, y]: P) => ring.findIndex(([u, v]) => Math.hypot(u - x, v - y) < 1e-9);
  const [left, right] = [at([-13, -3]), at([-7, -3])];
  expect(left).toBeGreaterThanOrEqual(0);
  expect(right).toBe((left + 1) % ring.length);
});
