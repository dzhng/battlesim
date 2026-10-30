import { shippedRules } from "@web/shippedRules";
// @vitest-environment node
// A side's players share its roads in, and a purchase may arrive by any of
// them, so every road in of the viewer's side carries its chevrons, and no
// road in of the other side does.
import { expect, test } from "vitest";
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
const scenario = JSON.stringify({
  map: { size: [1000, 1000] },
  rules: { service: { radius_m: 40 } },
  encounter: null,
  skirmish: {
    sites: {
      entries: [
        ...BLUE_ROADS.map((center) => ({ side: "blue", center, yaw: -Math.PI / 2 })),
        ...RED_ROADS.map((center) => ({ side: "red", center, yaw: Math.PI / 2 })),
      ],
      objectives: [],
    },
  },
});
const observation = {
  tick: 0,
  own: [],
  identified: [],
  contacts: [],
  skirmish: null,
  encounter: null,
} as unknown as ObservationView;

/** The ground points the marching paint covers. */
function marchingPoints(side: "blue" | "red"): [number, number][] {
  const meshes = buildBattleOverlay(
    {} as UnitCatalog,
    shippedRules,
    observation,
    [],
    () => 0,
    battleOverlayScenario(scenario, side),
    { showOrders: false, reveal: new Map(), contacts: [], facing: () => 0 } as never,
  );
  const mesh = meshes.paintedMarching ?? new Float32Array();
  const points: [number, number][] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) points.push([mesh[i], mesh[i + 1]]);
  return points;
}

const near = (points: [number, number][], at: [number, number]) =>
  points.some(([x, y]) => Math.hypot(x - at[0], y - at[1]) < 60);

test("every road in of the viewer's side carries chevrons, and the other side's none", () => {
  const blue = marchingPoints("blue");
  for (const road of BLUE_ROADS) expect(near(blue, road), `blue road at ${road}`).toBe(true);
  for (const road of RED_ROADS) expect(near(blue, road), `red road at ${road}`).toBe(false);
  const red = marchingPoints("red");
  for (const road of RED_ROADS) expect(near(red, road), `red road at ${road}`).toBe(true);
});
