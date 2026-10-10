// @vitest-environment node
// Where an aircraft is over the ground (D18): its marker lies on the ground
// under it as paint, and a drop line joins it to the airframe in the
// overlay. Every aircraft the side sees carries them, its own and an
// identified enemy's, whatever it is doing; a ground unit's marker stays
// contextual.
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { ObservationView, OwnUnitView, IdentifiedView } from "@web/battle/sim/observation";
import { aircraftLayer, orderLayer } from "@apps/battle-lab/src/battleOverlay";
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
const sameRgb = (a: readonly number[], b: readonly number[]) =>
  [0, 1, 2].every((k) => Math.abs(a[k] - b[k]) < 1e-5);

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

test("an own aircraft, neither selected nor ordered, still shows its ground marker and a drop line to it", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat));
  // The marker lies on the ground, round the point under the airframe, the
  // size a vehicle's marker is from its hull.
  expect(m.ring.length).toBeGreaterThan(0);
  expect(Math.max(...m.ringZ)).toBeLessThan(0.05);
  expect(Math.max(...m.ring)).toBeGreaterThan(HULL);
  // The drop line runs from the ground to the airframe, straight down.
  expect(Math.min(...m.lineZ)).toBeCloseTo(0, 5);
  expect(Math.max(...m.lineZ)).toBeCloseTo(HEIGHT, 5);
  expect(Math.max(...m.lineOff)).toBeLessThan(0.5);
  // The marker is quiet, the line keeps its own opacity: it still reads
  // over pale ground.
  for (const v of m.line) expect(v.c[3]).toBeCloseTo(STYLE.drop_line_alpha, 5);
});

test("a ground vehicle neither selected nor ordered still shows nothing", () => {
  const jeep = own({ kind: "test_jeep", position: [100, 50, 0] });
  const layer = orderLayer(UNITS, view({ own: [jeep] }), [], new Map(), flat);
  expect(layer.painted?.length ?? 0).toBe(0);
  expect(layer.opaque.length).toBe(0);
});

test("selected, an aircraft's marker and its drop line both take the selection's colour", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [1], new Map(), flat));
  expect(m.line.length).toBeGreaterThan(0);
  for (const v of m.line) expect(sameRgb(v.c, SELECTED)).toBe(true);
  const layer = orderLayer(UNITS, view({ own: [own({})] }), [1], new Map(), flat);
  for (const v of vertices(layer.painted)) expect(sameRgb(v.c, SELECTED)).toBe(true);
});

test("an identified enemy aircraft shows its marker and drop line in the enemy's colour; a ground enemy none", () => {
  const o = view({
    identified: [seen({}), seen({ id: 10, kind: "test_tank", position: [200, 50, 0] })],
  });
  const layer = aircraftLayer(UNITS, o, flat);
  const m = marks(layer);
  expect(Math.max(...m.ring)).toBeLessThan(HULL + 3);
  expect(Math.max(...m.lineZ)).toBeCloseTo(HEIGHT, 5);
  for (const v of [...vertices(layer.painted), ...m.line]) expect(sameRgb(v.c, ENEMY)).toBe(true);
});

test("the drop line keeps its weight on screen as the camera pulls out, thinning by the stroke rule", () => {
  const width = (metresPerPx: number) => {
    const layer = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat, metresPerPx);
    return 2 * Math.max(...marks(layer).lineOff);
  };
  // A square tube: its diagonal is √2 × its width. Every mark's stroke
  // comes from the one rule, the drop line's too.
  for (const metresPerPx of [OPENING_METRES_PER_PX, 0.4])
    expect(width(metresPerPx) / Math.SQRT2).toBeCloseTo(
      gameStroke(metresPerPx)(STYLE.drop_line_px),
      5,
    );
  // Pulled out to the map, it is still wider on the ground than close up.
  expect(width(0.4)).toBeGreaterThan(width(OPENING_METRES_PER_PX));
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
  expect(Math.max(...held)).toBeCloseTo(game.air.cruise_agl_m, 5);
  // An order's flash shows no ghost, so no line stands over its end.
  expect(overGoal(false)).toEqual([]);
});
