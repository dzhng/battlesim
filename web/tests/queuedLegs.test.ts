// @vitest-environment node
// A unit's queued legs chain from where its current order ends. A blocked
// unit has no destination circle, only its blocked mark at the goal, so its
// first queued leg leaves that mark; a winged aircraft circles its goal and
// keeps today's drawing (no ground leg to its first queued point).
import { expect, test } from "vitest";
import type { OrderView } from "@packages/battle-renderer/src/orderOverlay";
import { along, orderView, painted } from "./support/orderMesh";

type P2 = readonly [number, number];
const GOAL: P2 = [40, 0];
const QUEUED: P2 = [40, 60];

const view = (over: Partial<OrderView>) =>
  orderView({
    goal: GOAL,
    state: "moving",
    route: [GOAL],
    steps: [
      { kind: "move", point: GOAL },
      { kind: "move", point: QUEUED },
    ],
    ...over,
  });

/** The painted stretches of the segment a→b, as fractions of its length,
 *  away from the marks at its two ends. */
function dashes(v: OrderView, a: P2, b: P2): [number, number][] {
  const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
  return along(painted(v), a, b)
    .map(([p, q]): [number, number] => [p / len, q / len])
    .filter(([p, q]) => q > 0.15 && p < 0.85);
}

test("a blocked unit's first queued leg runs dashed from its blocked mark to the queued ring", () => {
  const leg = dashes(view({ state: "route_blocked" }), GOAL, QUEUED);
  // Paint runs most of the way between the two marks, in several dashes.
  expect(leg.length).toBeGreaterThan(2);
  expect(leg[0][0]).toBeLessThan(0.2);
  expect(leg.at(-1)![1]).toBeGreaterThan(0.8);
});

test("a winged aircraft draws no ground leg to its first queued point", () => {
  const jet = view({ fixedWing: true, airborne: true });
  expect(dashes(jet, GOAL, QUEUED)).toEqual([]);
});
