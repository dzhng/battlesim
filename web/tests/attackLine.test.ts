// @vitest-environment node
// A ground unit's current attack: in range, a solid line from its circle to
// a reticle on the target; chasing, its route ends in that reticle instead
// of a destination circle. A winged aircraft draws none of it (part 2).
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh } from "@packages/battle-renderer/src/mesh";
import { buildOrderOverlay, type OrderView } from "@packages/battle-renderer/src/orderOverlay";
import type { OrderStep } from "@packages/battle-renderer/src/orderSteps";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P2 = readonly [number, number];
const TARGET: P2 = [80, 0];
const HULL = 3;
const CIRCLE_R = HULL + gameOrderStyle.vehicle_marker_margin_m;
const attack: OrderStep = { kind: "attack", point: TARGET };

function view(over: Partial<OrderView>): OrderView {
  return {
    position: [0, 0, 0],
    goal: null,
    state: "idle",
    route: [],
    steps: [attack],
    members: [],
    memberOrders: [],
    finalFacing: 0,
    yaw: 0,
    direction: null,
    area: null,
    hullHalfLength: HULL,
    building: null,
    reveal: 1,
    ...over,
  };
}

const painted = (v: OrderView): Mesh =>
  buildOrderOverlay([v], () => 0, gameOrderStyle, { stroke: gameStroke(0.05) }).painted ??
  new Float32Array();

/** Each painted triangle as its three ground points. */
function triangles(mesh: Mesh): P2[][] {
  const out: P2[][] = [];
  const TRI = 3 * VERTEX_FLOATS;
  for (let i = 0; i + TRI <= mesh.length; i += TRI)
    out.push([0, 1, 2].map((k) => [mesh[i + k * VERTEX_FLOATS], mesh[i + k * VERTEX_FLOATS + 1]]));
  return out;
}

/** The stretches of the x axis (y = 0) that triangles lying on it cover. */
function onAxis(mesh: Mesh): [number, number][] {
  const spans = triangles(mesh)
    .filter((t) => t.every(([, y]) => Math.abs(y) < 1))
    .map(
      (t) => [Math.min(...t.map(([x]) => x)), Math.max(...t.map(([x]) => x))] as [number, number],
    )
    .sort((a, b) => a[0] - b[0]);
  const merged: [number, number][] = [];
  for (const s of spans) {
    const last = merged.at(-1);
    if (last && s[0] <= last[1] + 1e-6) last[1] = Math.max(last[1], s[1]);
    else merged.push([...s]);
  }
  return merged;
}

/** Painted vertices whose distance from `c` lies in [r0, r1]. */
function around(mesh: Mesh, c: P2, r0: number, r1: number): number {
  let n = 0;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
    const d = Math.hypot(mesh[i] - c[0], mesh[i + 1] - c[1]);
    if (d >= r0 && d <= r1) n++;
  }
  return n;
}

test("in range, one solid line runs from the unit's circle to the reticle on its target", () => {
  const mesh = painted(view({}));
  const r = gameOrderStyle.reticle_m;
  // The line: one unbroken stretch from the circle's edge (its facing
  // arrowhead, here) to the reticle's. The circle and arrowhead lie on the
  // axis too, short of it.
  const line = onAxis(mesh).filter(([, b]) => b > CIRCLE_R + 5);
  expect(line).toHaveLength(1);
  expect(line[0][0]).toBeLessThan(CIRCLE_R + 2);
  expect(line[0][1]).toBeGreaterThan(TARGET[0] - r - 1.5);
  // The reticle: its corners stand out at the square's diagonal.
  expect(around(mesh, TARGET, r * 1.2, r * 1.5)).toBeGreaterThan(0);
});

test("chasing, the route ends in the reticle, with no destination circle at its goal", () => {
  const goal: P2 = [78, 0];
  const chase = view({ goal, route: [goal], state: "moving" });
  const moving = view({
    goal,
    route: [goal],
    state: "moving",
    steps: [{ kind: "move", point: goal }],
  });
  const ringBand = (v: OrderView) => around(painted(v), goal, CIRCLE_R - 0.3, CIRCLE_R + 0.3);
  expect(ringBand(moving)).toBeGreaterThan(0);
  expect(ringBand(chase)).toBe(0);
  const r = gameOrderStyle.reticle_m;
  expect(around(painted(chase), TARGET, r * 1.2, r * 1.5)).toBeGreaterThan(0);
  // The route still runs up to the reticle.
  const route = onAxis(painted(chase)).filter(([a, b]) => b > CIRCLE_R && a < TARGET[0] - r);
  expect(route.at(-1)![1]).toBeGreaterThan(TARGET[0] - r - 1.5);
});

test("a winged aircraft's attack draws nothing new", () => {
  const jet = { fixedWing: true, airborne: true, position: [0, 0, 300] as const };
  expect(painted(view(jet))).toEqual(painted(view({ ...jet, steps: [] })));
});

test("a ground point's reticle carries a centre cross; a unit's has none", () => {
  const ground: OrderStep = { kind: "attack", point: TARGET, target: { kind: "ground" } };
  const centre = (step: OrderStep) => around(painted(view({ steps: [step] })), TARGET, 0, 1.5);
  expect(centre(ground)).toBeGreaterThan(0);
  expect(centre(attack)).toBe(0);
});
