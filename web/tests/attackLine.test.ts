// @vitest-environment node
// A ground unit's current attack: in range, a solid line from its circle to
// a reticle on the target; chasing, its route ends in that reticle instead
// of a destination circle. A winged aircraft draws none of it on the ground:
// its attack line stands in the air, from the airframe to its aim.
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh } from "@packages/battle-renderer/src/mesh";
import type { OrderView } from "@packages/battle-renderer/src/orderOverlay";
import type { OrderStep } from "@packages/battle-renderer/src/orderSteps";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { attackAim, orderLayer, type SeenTargets } from "@apps/battle-lab/src/battleOverlay";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";
import type {
  MountTargetView,
  ObservationView,
  OrderStepView,
} from "@web/battle/sim/observationView";
import { aloft, along, orderView, painted } from "./support/orderMesh";

type P2 = readonly [number, number];
const TARGET: P2 = [80, 0];
const HULL = 3;
const CIRCLE_R = HULL + gameOrderStyle.vehicle_marker_margin_m;
const attack: OrderStep = { kind: "attack", point: TARGET };

const view = (over: Partial<OrderView>) =>
  orderView({ hullHalfLength: HULL, steps: [attack], ...over });

/** The stretches of the x axis (y = 0), ahead of the unit, that painted
 *  triangles lying on it cover. */
const onAxis = (mesh: Mesh) => along(mesh, [0, 0], [1000, 0]);

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

test("a winged aircraft's attack is a line in the air from the airframe to its aim, nothing on the ground", () => {
  const top = [0, 0, 300] as const;
  const aim = [100, 0, 0] as const;
  const jet = { fixedWing: true, airborne: true, position: top };
  expect(painted(view({ ...jet, attackAim: aim }))).toEqual(painted(view({ ...jet, steps: [] })));
  const line = aloft(view({ ...jet, attackAim: aim }));
  expect(line.length).toBeGreaterThan(0);
  // Every vertex lies on the segment, and the marks run from end to end.
  const [dx, dz] = [aim[0] - top[0], aim[2] - top[2]];
  const len = Math.hypot(dx, dz);
  const ts: number[] = [];
  for (let i = 0; i < line.length; i += VERTEX_FLOATS) {
    const [x, y, z] = [line[i] - top[0], line[i + 1], line[i + 2] - top[2]];
    expect(Math.abs(x * dz - z * dx) / len + Math.abs(y)).toBeLessThan(3);
    ts.push((x * dx + z * dz) / len);
  }
  expect(Math.min(...ts)).toBeLessThan(len * 0.1);
  expect(Math.max(...ts)).toBeGreaterThan(len * 0.9);
  // With no aim, nothing stands in the air.
  expect(aloft(view(jet)).length).toBe(0);
});

test("a ground point's reticle carries a centre cross; a unit's has none", () => {
  const ground: OrderStep = { kind: "attack", point: TARGET, target: { kind: "ground" } };
  const centre = (step: OrderStep) => around(painted(view({ steps: [step] })), TARGET, 0, 1.5);
  expect(centre(ground)).toBeGreaterThan(0);
  expect(centre(attack)).toBe(0);
});

test("a jet's attack aims at an enemy or a contact where it is seen, at a ground point as ordered, else on the ground", () => {
  const ground = (x: number, y: number) => x / 10 + y;
  const seen: SeenTargets = {
    identified: [{ id: 9, position: [41, 3, 250] }],
    contacts: [
      { id: 5, center: [60, 8, 240], aloft: true },
      { id: 6, center: [70, 9, 0], aloft: false },
    ],
  };
  const aimAt = (target: MountTargetView) =>
    attackAim({ steps: [{ kind: "attack", point: [40, 2], target }] }, seen, ground);
  expect(aimAt({ kind: "identified", id: 9 })).toEqual([41, 3, 250]);
  expect(aimAt({ kind: "ground", point: [40, 2, 7] })).toEqual([40, 2, 7]);
  // An air contact where it hangs; a contact on the ground, on the ground.
  expect(aimAt({ kind: "contact", id: 5 })).toEqual([60, 8, 240]);
  expect(aimAt({ kind: "contact", id: 6 })).toEqual([70, 9, 16]);
  // An enemy the side no longer sees: its step's point, on the ground.
  expect(aimAt({ kind: "identified", id: 4 })).toEqual([40, 2, 6]);
  const moving: OrderStepView = { kind: "move", point: [1, 1] };
  expect(attackAim({ steps: [moving] }, { identified: [], contacts: [] }, ground)).toBeUndefined();
});

/** An own aircraft of `flight` at 300 m over the origin, attacking the enemy
 *  the side sees at [100, 0, 250]. */
function attackingAircraft(flight: "fixed_wing" | "rotor") {
  const units = {
    type: () => ({
      mobility: {
        air: { cruise_kmh: 500, turn_deg_s: 30, climb_mps: 8, layer: "low_air", flight },
      },
    }),
    hull: () => ({ half_extents_m: [6, 2, 1.5] }),
  } as unknown as UnitCatalog;
  const unit = {
    id: 1,
    kind: "plane",
    position: [0, 0, 300],
    yaw: 0,
    hold: null,
    stock: null,
    members: [],
    memberOrders: [],
    area: null,
    goal: null,
    route: [],
    finalFacing: 0,
    direction: null,
    state: "idle",
    steps: [{ kind: "attack", point: [100, 0], target: { kind: "identified", id: 9 } }],
  };
  const o = {
    tick: 0,
    own: [unit],
    identified: [{ id: 9, kind: "plane", position: [100, 0, 250] }],
    knownProps: [],
    contacts: [],
    encounter: null,
    skirmish: null,
  } as unknown as ObservationView;
  const air = {} as Parameters<typeof orderLayer>[1];
  return (reveal: number) =>
    orderLayer(units, air, o, [], new Map(reveal ? [[1, reveal]] : []), () => 0, {
      stroke: gameStroke(0.05),
    });
}

/** Vertices of `mesh` more than `d` metres from the vertical through the origin. */
const awayFromOrigin = (mesh: Mesh, d: number) => {
  let n = 0;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if (Math.hypot(mesh[i], mesh[i + 1]) > d) n++;
  return n;
};

test("the battle view gives a revealed jet's attack its line in the air, and nothing more", () => {
  const jet = attackingAircraft("fixed_wing");
  // Revealed: dots run out from the airframe toward the enemy.
  expect(awayFromOrigin(jet(1).opaque, 20)).toBeGreaterThan(0);
  // Not revealed: no order marks, so nothing stands in the air.
  expect(awayFromOrigin(jet(0).opaque, 20)).toBe(0);
});

test("a helicopter's attack keeps its marks on the ground and draws no line in the air", () => {
  const heli = attackingAircraft("rotor");
  // Its drop line stands straight under it; nothing runs off toward the enemy.
  expect(awayFromOrigin(heli(1).opaque, 20)).toBe(0);
  // Its reticle is painted on the ground at the enemy.
  expect(awayFromOrigin(heli(1).painted ?? new Float32Array(), 90)).toBeGreaterThan(0);
});
