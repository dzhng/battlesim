// @vitest-environment node
// Fixed-wing aircraft carry their position and state in floating cards. They
// never get the ground circles used by helicopters and ground units.
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { ObservationView, OwnUnitView, IdentifiedView } from "@web/battle/sim/observationView";
import { aircraftLayer, orderLayer } from "@apps/battle-lab/src/battleOverlay";
import { fullBright } from "@packages/battle-renderer/src/orderOverlay";
import {
  gameOrderStyle as STYLE,
  gameStroke,
  OPENING_METRES_PER_PX,
} from "@apps/battle-lab/src/gameOverlay";
import { gameHud } from "@web/battle/present/hudTheme";
import { UNITS } from "./catalog";
import game from "../../fixtures/game.json";

const flat = () => 0;
const HEIGHT = 20;
const AT: [number, number, number] = [100, 50, HEIGHT];
const glow = (c: Rgba, g: number): Rgba => [c[0] * g, c[1] * g, c[2] * g, c[3]];
const SELECTED = glow(STYLE.selected, STYLE.glow.selected);
const ENEMY = glow([...gameHud.enemy, 1], STYLE.glow.order);
const HULL = UNITS.hull("test_heli")!.half_extents_m[0];

const own = (u: Partial<OwnUnitView>) =>
  ({
    id: 1,
    kind: "test_heli",
    position: AT,
    yaw: 0,
    goal: null,
    state: "idle",
    route: [],
    queue: [],
    members: [],
    memberOrders: [],
    area: null,
    garrison: null,
    finalFacing: 0,
    direction: null,
    ...u,
  }) as OwnUnitView;
const seen = (u: Partial<IdentifiedView>) =>
  ({ id: 9, kind: "test_heli", position: AT, yaw: 0, members: [], ...u }) as IdentifiedView;
const view = (o: Partial<ObservationView>) =>
  ({ own: [], identified: [], ...o }) as unknown as ObservationView;

const vertices = (mesh: Mesh | undefined) => {
  const out: { p: number[]; c: number[] }[] = [];
  if (!mesh) return out;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push({
      p: Array.from(mesh.slice(i, i + 3)),
      c: Array.from(mesh.slice(i + 6, i + 10)),
    });
  return out;
};
/** `c` as the dots draw it, by the overlay's own rule. */
const asDots = (c: Rgba): Rgba => [...fullBright(c), c[3]];
const sameRgb = (a: readonly number[], b: readonly number[]) =>
  [0, 1, 2].every((k) => Math.abs(a[k] - b[k]) < 1e-5);

/** The drop line's dots, bottom up: each one's height span and opacity. */
function dotsOf(line: { p: number[]; c: number[] }[]) {
  // Dots are drawn bottom up, each from its top: a vertex above the dot
  // being read starts the next one.
  const groups: (typeof line)[] = [];
  for (const v of line) {
    const last = groups.at(-1);
    if (last && v.p[2] <= Math.max(...last.map((u) => u.p[2])) + 1e-6) last.push(v);
    else groups.push([v]);
  }
  return groups
    .map((g) => {
      const z = g.map((v) => v.p[2]);
      const centre = [0, 1, 2].map(
        (k) => (Math.min(...g.map((v) => v.p[k])) + Math.max(...g.map((v) => v.p[k]))) / 2,
      );
      return {
        from: Math.min(...z),
        to: Math.max(...z),
        alpha: g[0].c[3],
        reach: g.map((v) => Math.hypot(v.p[0] - centre[0], v.p[1] - centre[1], v.p[2] - centre[2])),
        // Points straight over or under its centre.
        poles: g.filter((v) => Math.hypot(v.p[0] - centre[0], v.p[1] - centre[1]) < 1e-6).length,
      };
    })
    .sort((a, b) => a.from - b.from);
}

/** The ground ring's radius span round `c`, and the drop line's extent. */
function marks(layer: { painted?: Mesh; opaque: Mesh }, c = AT) {
  const ring = vertices(layer.painted).map((v) => Math.hypot(v.p[0] - c[0], v.p[1] - c[1]));
  const line = vertices(layer.opaque);
  return {
    ring,
    ringZ: vertices(layer.painted).map((v) => v.p[2]),
    line,
    lineZ: line.map((v) => v.p[2]),
    lineOff: line.map((v) => Math.hypot(v.p[0] - c[0], v.p[1] - c[1])),
  };
}

test("an own aircraft has no ground marker or drop line", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat));
  expect(m.ring).toEqual([]);
  expect(m.line).toEqual([]);
});

test("an aircraft overlay has no marker geometry", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat));
  expect(m.ring).toEqual([]);
  expect(m.line).toEqual([]);
});

test("aircraft produce no drop line solids", () => {
  const layer = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat);
  const normals = new Set<string>();
  for (let i = 0; i < layer.opaque.length; i += VERTEX_FLOATS)
    normals.add(Array.from(layer.opaque.slice(i + 3, i + 6)).join());
  expect([...normals]).toEqual([]);
});

test("aircraft have no dot geometry", () => {
  const dots = dotsOf(marks(orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat)).line);
  expect(dots).toEqual([]);
});

test("aircraft remain marker-free at close zoom", () => {
  const close = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat, 0.001);
  expect(dotsOf(marks(close).line).length).toBe(0);
});

test("a ground vehicle neither selected nor ordered still shows nothing", () => {
  const jeep = own({ kind: "test_jeep", position: [100, 50, 0] });
  const layer = orderLayer(UNITS, view({ own: [jeep] }), [], new Map(), flat);
  expect(layer.painted?.length ?? 0).toBe(0);
  expect(layer.opaque.length).toBe(0);
});

test("selection does not add an aircraft ground marker", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [1], new Map(), flat));
  expect(m.line.length).toBe(0);
  const layer = orderLayer(UNITS, view({ own: [own({})] }), [1], new Map(), flat);
  expect(vertices(layer.painted)).toEqual([]);
});

test("an identified enemy aircraft also has no ground marker", () => {
  const o = view({
    identified: [seen({}), seen({ id: 10, kind: "test_tank", position: [200, 50, 0] })],
  });
  const layer = aircraftLayer(UNITS, o, flat);
  const m = marks(layer);
  expect(m.ring).toEqual([]);
  expect(m.line).toEqual([]);
});

test("aircraft remain marker-free as the camera pulls out", () => {
  const width = (metresPerPx: number) => {
    const layer = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat, metresPerPx);
    return 2 * Math.max(...marks(layer).lineOff);
  };
  // A ball: as wide as its diameter (the mesh is float32). Every mark's stroke comes from the
  // one rule, the drop line's dots too.
  for (const metresPerPx of [OPENING_METRES_PER_PX, 0.4]) expect(width(metresPerPx)).toBe(0);
});

test("with Space held, an aircraft's ghost where its orders end is joined to that end's marker by a drop line", () => {
  const GOAL: [number, number] = [200, 50];
  const ordered = own({ goal: GOAL, state: "moving", route: [GOAL], finalFacing: 0 });
  const o = view({ own: [ordered] });
  const reveal = new Map([[1, 1]]);
  /** Drop-line vertices standing over the goal, by their heights. */
  const overGoal = (ghosts: boolean) =>
    vertices(orderLayer(UNITS, o, [], reveal, flat, undefined, ghosts).opaque)
      .filter((v) => Math.hypot(v.p[0] - GOAL[0], v.p[1] - GOAL[1]) < 0.5)
      .map((v) => v.p[2]);
  const held = overGoal(true);
  expect(Math.min(...held)).toBeCloseTo(0, 5);
  expect(Math.max(...held)).toBeGreaterThan(game.air.cruise_agl_m * STYLE.drop_line_reach * 0.8);
  expect(Math.max(...held)).toBeLessThanOrEqual(
    game.air.cruise_agl_m * STYLE.drop_line_reach + 1e-6,
  );
  // An order's flash shows no ghost, so no line stands over its end.
  expect(overGoal(false)).toEqual([]);
});
