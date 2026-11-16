// @vitest-environment node
// A house several squads share reads as one held position: one ring round it
// whichever of its occupants are selected, revealed or leaving, in the
// brightest look any of them would draw, and one outline round it whichever
// of its holders the side knows, own or identified enemy, by whatever part.
// Each squad keeps its own soldiers' marks and its own route.
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh } from "@packages/battle-renderer/src/mesh";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { RevealedOrders } from "@web/battle/present/orderReveal";
import type { ObservationView } from "@web/battle/sim/observationView";
import { heldBuildingsLayer, orderLayer } from "@apps/battle-lab/src/battleOverlay";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P2 = [number, number];
const units = {
  type: () => ({ mobility: {} }),
  hull: () => null,
} as unknown as UnitCatalog;
const air = {} as Parameters<typeof orderLayer>[1];
const stroke = gameStroke(0.05);

/** Two houses far apart, each 12 m square; its structure owner is its id. */
const HOUSE = { id: 7, center: [0, 0] as P2, half: [6, 6] as P2 };
const OTHER = { id: 8, center: [1000, 0] as P2, half: [6, 6] as P2 };
/** The building ring's radius: the house's corners plus the style's margin. */
const RING_R = Math.hypot(6, 6) + gameOrderStyle.building_marker_margin_m;

/** A rifle squad of two soldiers at the windows of `house`, inside or
 *  stepping out toward `goal`. */
function occupant(id: number, house: typeof HOUSE, goal: P2 | null = null) {
  const [x, y] = house.center;
  return {
    id,
    kind: "test_rifle",
    position: [x, y, 0],
    yaw: 0,
    hold: {
      host: { kind: "building", ...house },
      phase: goal ? "exiting" : "inside",
      progress: goal ? 0.2 : 1,
    },
    stock: null,
    members: [
      [x + 5.5, y + id - 4, 0],
      [x - 5.5, y + id - 4, 0],
    ],
    memberOrders: [],
    area: { anchor: goal ?? [x, y], radius: 6 },
    goal,
    route: goal ? [goal] : [],
    queue: [],
    finalFacing: 0,
    direction: null,
    state: goal ? "moving" : "idle",
  };
}

const observation = (own: ReturnType<typeof occupant>[]) =>
  ({
    tick: 0,
    own,
    identified: [],
    knownProps: [],
    contacts: [],
    encounter: null,
    skirmish: null,
  }) as unknown as ObservationView;

function orders(
  own: ReturnType<typeof occupant>[],
  selected: number[],
  reveal: RevealedOrders = new Map(),
) {
  const m = orderLayer(units, air, observation(own), selected, reveal, () => 0, { stroke });
  return [m.painted, m.paintedMarching];
}

/** The paint within the band of `house`'s ring, as vertices (east of its
 *  centre only, with `east`: clear of a route leaving west). */
function ring(meshes: (Mesh | undefined)[], house = HOUSE, east = false): number[][] {
  const out: number[][] = [];
  for (const mesh of meshes)
    for (let i = 0; mesh && i < mesh.length; i += VERTEX_FLOATS) {
      const [dx, dy] = [mesh[i] - house.center[0], mesh[i + 1] - house.center[1]];
      if (Math.abs(Math.hypot(dx, dy) - RING_R) < 0.75 && (!east || dx > 0))
        out.push([...mesh.subarray(i, i + VERTEX_FLOATS)]);
    }
  return out;
}

/** The paint between the ring and `goal`, along the way out. */
function wayOut(meshes: (Mesh | undefined)[], goal: P2): number {
  let n = 0;
  for (const mesh of meshes)
    for (let i = 0; mesh && i < mesh.length; i += VERTEX_FLOATS) {
      const d = Math.hypot(mesh[i], mesh[i + 1]);
      const toGoal = Math.hypot(mesh[i] - goal[0], mesh[i + 1] - goal[1]);
      if (d > RING_R + 1 && toGoal > 10 && toGoal < Math.hypot(...goal)) n++;
    }
  return n;
}

const A = occupant(1, HOUSE);
const B = occupant(2, HOUSE);

test("two selected squads in one house draw the ring one selected occupant draws", () => {
  const one = ring(orders([A, B], [A.id]));
  expect(one.length).toBeGreaterThan(0);
  expect(ring(orders([A, B], [A.id, B.id]))).toEqual(one);
});

test("the Space view draws one ring per house, and a selected occupant's look wins", () => {
  const all: RevealedOrders = new Map([
    [A.id, 1],
    [B.id, 1],
  ]);
  expect(ring(orders([A, B], [], all))).toEqual(ring(orders([A], [], all)));
  expect(ring(orders([A, B], [A.id], all))).toEqual(ring(orders([A, B], [A.id])));
});

test("a squad leaving on an order keeps its route but adds no ring", () => {
  const goal: P2 = [-60, 0];
  const leaving = occupant(2, HOUSE, goal);
  const shown = orders([A, leaving], [A.id], new Map([[leaving.id, 1]]));
  expect(ring(shown, HOUSE, true)).toEqual(ring(orders([A, leaving], [A.id]), HOUSE, true));
  expect(wayOut(shown, goal)).toBeGreaterThan(0);
});

test("squads in different houses each keep their own ring", () => {
  const elsewhere = occupant(3, OTHER);
  const meshes = orders([A, B, elsewhere], [A.id, B.id, elsewhere.id]);
  expect(ring(meshes, OTHER)).toEqual(ring(orders([elsewhere], [elsewhere.id]), OTHER));
  expect(ring(meshes)).toEqual(ring(orders([A], [A.id])));
});

/** House 7 as the map draws it: two parts, props 7 and 9. */
const part = (id: number, x: number) => ({
  id,
  kind: "test_house",
  center: [x, 0] as const,
  yaw: 0,
  half: [3, 6, 6] as const,
  baseZ: 0,
});
const buildings = {
  partBuilding: new Map([
    [7, 0],
    [9, 0],
  ]),
  parts: [[part(7, -3), part(9, 3)]],
};

/** The held-building outlines for `o`'s holders. */
function outlines(o: Partial<ObservationView>): number[] {
  const view = { ...observation([]), ...o } as ObservationView;
  return [...(heldBuildingsLayer(view, buildings, 0.3, () => 0).painted ?? [])];
}

test("a house two own squads hold has one outline", () => {
  const one = outlines({ own: [A] as unknown as ObservationView["own"] });
  expect(one.length).toBeGreaterThan(0);
  expect(outlines({ own: [A, B] as unknown as ObservationView["own"] })).toEqual(one);
});

test("identified enemies in one house, seen by any part or its remains, draw one outline", () => {
  const enemy = (id: number, building: number) => ({ id, kind: "test_rifle", building });
  const one = outlines({ identified: [enemy(20, 7)] as unknown as ObservationView["identified"] });
  expect(one.length).toBeGreaterThan(0);
  expect(
    outlines({
      identified: [
        enemy(20, 7),
        enemy(21, 9),
        enemy(22, 100),
      ] as unknown as ObservationView["identified"],
      // Prop 100 is the ruin standing in place of part 9.
      knownProps: [{ id: 100, authoredProp: 9 }] as unknown as ObservationView["knownProps"],
    }),
  ).toEqual(one);
});
