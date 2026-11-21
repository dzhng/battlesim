import { shippedRules } from "@web/shippedRules";
// @vitest-environment node
// A side's players share its roads in, and a purchase may arrive by any of
// them, so every road in of the viewer's side carries its beacons, and no
// road in of the other side does. Each also paints its lane's number nearer
// the edge, and a unit's marks draw over it. Pulled far out, where the
// painted marker shrinks to a few pixels, it glows across a patch of screen
// so the roads in still stand out.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { lanesOf } from "@web/battle/prepare/lanes";
import type { ObservationView } from "@web/battle/sim/observationView";
import { battleOverlayScenario, buildBattleOverlay } from "@apps/battle-lab/src/battleOverlay";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { gameStroke, OPENING_METRES_PER_PX } from "@apps/battle-lab/src/gameOverlay";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";

/** The way Blue's roads run in from its edge. */
const BLUE_IN = -Math.PI / 2;
const BLUE_ROADS: [number, number][] = [
  [200, 990],
  [800, 990],
];
const RED_ROADS: [number, number][] = [
  [200, 10],
  [800, 10],
];
const scenario = {
  map: { size: [1000, 1000] as [number, number] },
  rules: { service: { radius_m: 40 } },
  encounter: null,
  skirmish: {
    sites: {
      entries: [
        ...BLUE_ROADS.map((center) => ({ side: "blue" as const, center, yaw: BLUE_IN })),
        ...RED_ROADS.map((center) => ({ side: "red" as const, center, yaw: Math.PI / 2 })),
      ],
      objectives: [],
    },
  },
};
const seat = (id: string, human: boolean) => ({
  id,
  side: id.startsWith("blue") ? ("blue" as const) : ("red" as const),
  controller: human ? ("human" as const) : ("active_ai" as const),
  human_capable: human,
});
const observation = {
  tick: 0,
  own: [],
  identified: [],
  contacts: [],
  skirmish: null,
  encounter: null,
} as unknown as ObservationView;

beforeAll(() => {
  wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
});

type P = [number, number];
interface Vertex {
  at: P;
  alpha: number;
  /** The marching pass reads the normal as pulse data; still marks have none. */
  pulses: boolean;
  /** Where in its cycle the pulse peaks: a later phase lights later. */
  phase: number;
}
const TANK = {
  id: 1,
  kind: "test_tank",
  position: [245, 975, 0],
  yaw: 0,
  hold: null,
  stock: null,
  members: [],
  steps: [],
  goal: null,
  state: "idle",
};
const catalog = {
  type: () => ({ mobility: {} }),
  hull: () => ({ half_extents_m: [4, 2, 1.5] }),
} as unknown as UnitCatalog;

/** The marching paint's vertices in draw order, as `side` sees the battle
 *  with Blue 1 human, and Blue 1's tank beside its road selected or not,
 *  through a camera where one pixel spans `metresPerPx`. */
function marching(
  side: "blue" | "red",
  selected: number[] = [],
  metresPerPx = OPENING_METRES_PER_PX,
): Vertex[] {
  const players = ["blue-1", "blue-2", "red-1", "red-2"].map((id) => seat(id, id === "blue-1"));
  const lanes = lanesOf(wasm, JSON.stringify(scenario.skirmish.sites), players).entries;
  const own = { ...observation, own: selected.length ? [TANK] : [] } as unknown as ObservationView;
  const meshes = buildBattleOverlay(
    catalog,
    shippedRules,
    own,
    selected,
    () => 0,
    battleOverlayScenario(scenario, lanes, side),
    { showOrders: false, reveal: new Map(), contacts: [], facing: () => 0 } as never,
    null,
    gameStroke(metresPerPx),
  );
  const mesh = meshes.paintedMarching ?? new Float32Array();
  const out: Vertex[] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push({
      at: [mesh[i], mesh[i + 1]],
      alpha: mesh[i + 9],
      pulses: mesh[i + 5] !== 0,
      phase: mesh[i + 3],
    });
  return out;
}

const near = (vertices: Vertex[], at: P, within = 60) =>
  vertices.filter(({ at: [x, y] }) => Math.hypot(x - at[0], y - at[1]) < within);
/** How far `p` lies into the map from Blue's road at `road`, along the way
 *  it runs in (its entry's heading), and how far to one side of it. */
const inward = (road: P, [x, y]: P) =>
  (x - road[0]) * Math.cos(BLUE_IN) + (y - road[1]) * Math.sin(BLUE_IN);
const aside = (road: P, [x, y]: P) =>
  -(x - road[0]) * Math.sin(BLUE_IN) + (y - road[1]) * Math.cos(BLUE_IN);

test("every road in of the viewer's side carries beacons, and the other side's none", () => {
  const blue = marching("blue");
  for (const road of BLUE_ROADS)
    expect(near(blue, road).length, `blue road at ${road}`).toBeGreaterThan(0);
  for (const road of RED_ROADS) expect(near(blue, road), `red road at ${road}`).toEqual([]);
  const red = marching("red");
  for (const road of RED_ROADS)
    expect(near(red, road).length, `red road at ${road}`).toBeGreaterThan(0);
});

test("each road in paints its lane's number between the edge and its beacons", () => {
  for (const road of BLUE_ROADS) {
    const marks = near(marching("blue"), road);
    // Past the outline's overhang behind the beacons' arms, a still mark
    // nearer the edge than every pulsing chevron is the number.
    const beacons = Math.min(...marks.filter((v) => v.pulses).map((v) => inward(road, v.at)));
    const number = marks.filter((v) => !v.pulses && inward(road, v.at) < beacons - 2.5);
    expect(number.length, `blue road at ${road}`).toBeGreaterThan(0);
  }
});

test("each road's beacons march straight in along it, not toward the map's centre", () => {
  // Both roads run straight in, well to either side of the map's centre:
  // their beacons stay within a beacon's spread of the road's own line.
  for (const road of BLUE_ROADS) {
    const beacons = near(marching("blue"), road).filter((v) => v.pulses);
    const widest = Math.max(...beacons.map((v) => Math.abs(aside(road, v.at))));
    expect(widest, `blue road at ${road}`).toBeLessThan(8);
  }
});

test("each road's beacons light one after another, inward from the edge, like a runway's", () => {
  for (const road of BLUE_ROADS) {
    const beacons = near(marching("blue"), road)
      .filter((v) => v.pulses)
      .sort((a, b) => inward(road, a.at) - inward(road, b.at));
    const phases = beacons.map((v) => v.phase);
    // No beacon lights before one nearer the edge, they don't all light
    // together, and the run fits in one cycle, so it never reads backwards.
    expect(phases, `blue road at ${road}`).toEqual([...phases].sort((a, b) => a - b));
    expect(new Set(phases).size, `blue road at ${road}`).toBeGreaterThan(1);
    expect(phases[phases.length - 1] - phases[0], `blue road at ${road}`).toBeLessThan(1);
  }
});

test("a unit's marks draw over the road in it stands beside", () => {
  const road = BLUE_ROADS[0];
  const drawn = marching("blue", [TANK.id]);
  const order = (v: Vertex) => drawn.indexOf(v);
  // The marker spans a few metres across its road; the tank's ring lies
  // 45 m to one side of it.
  const across = (v: Vertex) => Math.abs(v.at[0] - road[0]);
  const marker = near(drawn, road).filter((v) => across(v) < 20);
  const ring = near(drawn, [TANK.position[0], TANK.position[1]], 15).filter((v) => across(v) > 30);
  expect(marker.length).toBeGreaterThan(0);
  expect(ring.length).toBeGreaterThan(0);
  expect(Math.max(...marker.map(order))).toBeLessThan(Math.min(...ring.map(order)));
});

/** One pixel at the camera's target, pulled all the way out on a 1080-pixel
 *  viewport. */
const FARTHEST_M_PER_PX = metresPerPxAt(gameCamera.config.zoom_max, gameCamera.lens.fovY, 1080);

/** How far across its road, either way, a road in's visible paint reaches. */
const spread = (vertices: Vertex[], road: P) =>
  Math.max(
    ...near(vertices, road, 120)
      .filter((v) => v.alpha > 0.02)
      .map((v) => Math.abs(aside(road, v.at))),
  );

test("pulled all the way out, each road in still glows across a patch of screen", () => {
  // The painted marker is a few metres wide: a pixel or two from this far.
  // A dozen pixels either way of the road reads as a mark at a glance.
  const far = marching("blue", [], FARTHEST_M_PER_PX);
  for (const road of BLUE_ROADS)
    expect(spread(far, road) / FARTHEST_M_PER_PX, `blue road at ${road}`).toBeGreaterThan(12);
});

test("close in, a road in is only its painted marker, with no glow around it", () => {
  const near = marching("blue");
  // The widest beacon's radius is the marker's own width.
  for (const road of BLUE_ROADS) expect(spread(near, road), `blue road at ${road}`).toBeLessThan(4);
});
