import { shippedRules } from "@web/shippedRules";
// @vitest-environment node
// A side's players share its roads in, and a purchase may arrive by any of
// them, so every road in of the viewer's side carries its chevrons, and no
// road in of the other side does. Each also paints its lane's number nearer
// the edge, and the viewer's own lane's number stands out.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { lanesOf } from "@web/battle/prepare/lanes";
import type { ObservationView } from "@web/battle/sim/observationView";
import { battleOverlayScenario, buildBattleOverlay } from "@apps/battle-lab/src/battleOverlay";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";

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
        ...BLUE_ROADS.map((center) => ({ side: "blue" as const, center, yaw: -Math.PI / 2 })),
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
/** The ground points each kind of paint covers, as `side` sees the battle
 *  with `viewer` seated (Blue 1 and 2, Red 1 and 2, the viewer human). */
function paint(side: "blue" | "red", viewer: string): { still: P[]; marching: P[] } {
  const players = ["blue-1", "blue-2", "red-1", "red-2"].map((id) => seat(id, id === viewer));
  const lanes = lanesOf(wasm, JSON.stringify(scenario.skirmish.sites), players).entries;
  const meshes = buildBattleOverlay(
    {} as UnitCatalog,
    shippedRules,
    observation,
    [],
    () => 0,
    battleOverlayScenario(scenario, lanes, side, viewer),
    { showOrders: false, reveal: new Map(), contacts: [], facing: () => 0 } as never,
  );
  const points = (mesh = new Float32Array()) => {
    const out: P[] = [];
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) out.push([mesh[i], mesh[i + 1]]);
    return out;
  };
  return { still: points(meshes.painted), marching: points(meshes.paintedMarching) };
}

const near = (points: P[], at: P) =>
  points.filter(([x, y]) => Math.hypot(x - at[0], y - at[1]) < 60);
/** How far `p` lies into the map from the road at `road`, along the way in
 *  (toward the map's centre), and how far across it. */
const way = (road: P) => {
  const a = Math.atan2(500 - road[1], 500 - road[0]);
  return [Math.cos(a), Math.sin(a)] as const;
};
const inward = (road: P, p: P) => {
  const [fx, fy] = way(road);
  return (p[0] - road[0]) * fx + (p[1] - road[1]) * fy;
};
const across = (road: P, p: P) => {
  const [fx, fy] = way(road);
  return (p[0] - road[0]) * fy - (p[1] - road[1]) * fx;
};

test("every road in of the viewer's side carries chevrons, and the other side's none", () => {
  const blue = paint("blue", "blue-1").marching;
  for (const road of BLUE_ROADS)
    expect(near(blue, road).length, `blue road at ${road}`).toBeGreaterThan(0);
  for (const road of RED_ROADS) expect(near(blue, road), `red road at ${road}`).toEqual([]);
  const red = paint("red", "red-1").marching;
  for (const road of RED_ROADS)
    expect(near(red, road).length, `red road at ${road}`).toBeGreaterThan(0);
});

test("each road in paints its lane's number between the edge and its chevrons", () => {
  const { still, marching } = paint("blue", "blue-1");
  for (const road of BLUE_ROADS) {
    const number = near(still, road);
    expect(number.length, `blue road at ${road}`).toBeGreaterThan(0);
    const deepest = Math.max(...number.map((p) => inward(road, p)));
    const chevrons = Math.min(...near(marching, road).map((p) => inward(road, p)));
    expect(deepest, `blue road at ${road}`).toBeLessThan(chevrons);
  }
  for (const road of RED_ROADS) expect(near(still, road), `red road at ${road}`).toEqual([]);
});

test("the viewer's own lane's number is drawn larger than the same number not its own", () => {
  const width = (viewer: string, road: P) => {
    const xs = near(paint("blue", viewer).still, road).map((p) => across(road, p));
    return Math.max(...xs) - Math.min(...xs);
  };
  // Blue 1 holds one road, Blue 2 the other: each road, seen as its holder
  // and as the other, is the same number either way.
  for (const road of BLUE_ROADS) {
    const own = Math.max(width("blue-1", road), width("blue-2", road));
    const other = Math.min(width("blue-1", road), width("blue-2", road));
    expect(own, `blue road at ${road}`).toBeGreaterThan(other);
  }
});
