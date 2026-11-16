// @vitest-environment node
// A unit's queued orders chain from where its current order ends, each step
// drawn as that order draws when current, dashed: every leg starts at the
// mark the step before it ends in and runs to the next step's mark.
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh } from "@packages/battle-renderer/src/mesh";
import { buildOrderOverlay, type OrderView } from "@packages/battle-renderer/src/orderOverlay";
import type { OrderStep } from "@packages/battle-renderer/src/orderSteps";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P2 = readonly [number, number];
const R = gameOrderStyle.reticle_m;

function view(steps: OrderStep[]): OrderView {
  return {
    position: [0, 0, 0],
    goal: steps[0].point,
    state: "moving",
    route: [steps[0].point!],
    steps,
    members: [],
    memberOrders: [],
    finalFacing: 0,
    yaw: 0,
    direction: null,
    area: null,
    hullHalfLength: 3,
    building: null,
    reveal: 1,
  };
}

const painted = (v: OrderView): Mesh =>
  buildOrderOverlay([v], () => 0, gameOrderStyle, { stroke: gameStroke(0.05) }).painted ??
  new Float32Array();

/** The stretches of segment a→b (as distances from a) that painted
 *  triangles lying along it, within a metre, cover. */
function covered(mesh: Mesh, a: P2, b: P2): [number, number][] {
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const len = Math.hypot(dx, dy);
  const spans: [number, number][] = [];
  const TRI = 3 * VERTEX_FLOATS;
  for (let i = 0; i + TRI <= mesh.length; i += TRI) {
    const ts: number[] = [];
    for (let k = 0; k < 3; k++) {
      const [px, py] = [mesh[i + k * VERTEX_FLOATS] - a[0], mesh[i + k * VERTEX_FLOATS + 1] - a[1]];
      if (Math.abs(px * dy - py * dx) / len > 1) break;
      ts.push((px * dx + py * dy) / len);
    }
    if (ts.length === 3 && Math.min(...ts) > -1 && Math.max(...ts) < len + 1)
      spans.push([Math.min(...ts), Math.max(...ts)]);
  }
  return spans.sort((p, q) => p[0] - q[0]);
}

// Far enough apart that no leg runs over another step's mark.
const A: P2 = [100, 0];
const B: P2 = [100, 100];
const C: P2 = [200, 100];
const HOUSE = { center: [300, 100] as P2, half: [8, 8] as P2 };
const HOUSE_R = Math.hypot(8, 8) + gameOrderStyle.building_marker_margin_m;
const E: P2 = [300, 250];
const G: P2 = [450, 250];
const steps: OrderStep[] = [
  { kind: "move", point: A },
  { kind: "move", point: B },
  { kind: "attack", point: C },
  { kind: "garrison", point: [288, 100], building: HOUSE },
  { kind: "exit", point: HOUSE.center, building: HOUSE },
  { kind: "board", point: E },
  { kind: "return_to_base", point: G },
];

test("every queued step's leg leaves the mark the step before ends in and reaches its own", () => {
  const mesh = painted(view(steps));
  // Each leg: from the previous mark's centre to the next one's, and how far
  // out from each centre its mark reaches (a ring, the reticle, a house).
  const legs: [P2, number, P2, number][] = [
    [A, 0, B, 2.8],
    [B, 2.8, C, R],
    [C, R, HOUSE.center, HOUSE_R],
    [HOUSE.center, HOUSE_R, E, 0],
    [E, 0, G, 2.8],
  ];
  for (const [from, r0, to, r1] of legs) {
    const spans = covered(mesh, from, to);
    const len = Math.hypot(to[0] - from[0], to[1] - from[1]);
    expect(spans.length, `${from}→${to}`).toBeGreaterThan(2);
    // The dashes start at the first mark's edge and end at the next's (a
    // dash's length, 3 m, of slack at each end).
    expect(spans[0][0], `${from}→${to} start`).toBeLessThan(r0 + 4);
    expect(spans.at(-1)![1], `${from}→${to} end`).toBeGreaterThan(len - r1 - 4);
  }
});

test("a queued attack draws the same reticle as a current one", () => {
  const T: P2 = [80, 0];
  const FROM: P2 = [0, 40];
  // Off the line, from the unit (along y = 0) or from the queued move.
  const offLines = (x: number, y: number) => {
    const [dx, dy] = [T[0] - FROM[0], T[1] - FROM[1]];
    const fromQueued = Math.abs((x - FROM[0]) * dy - (y - FROM[1]) * dx) / Math.hypot(dx, dy);
    return Math.abs(y) > 1 && fromQueued > 1;
  };
  const reticle = (mesh: Mesh) => {
    const out: string[] = [];
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
      const [x, y] = [mesh[i], mesh[i + 1]];
      const d = Math.hypot(x - T[0], y - T[1]);
      if (d < R * 1.6 && offLines(x, y)) out.push(`${x.toFixed(3)},${y.toFixed(3)}`);
    }
    return out.sort();
  };
  const current = { ...view([{ kind: "attack", point: T }]), goal: null, route: [], state: "idle" };
  const queued = view([
    { kind: "move", point: FROM },
    { kind: "attack", point: T },
  ]);
  expect(reticle(painted(current)).length).toBeGreaterThan(0);
  expect(reticle(painted(queued))).toEqual(reticle(painted(current)));
});
