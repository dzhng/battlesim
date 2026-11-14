// @vitest-environment node
// The unit under the pointer (its model, or its card) shows its circle on the
// ground, the same circle a click would select, in a look of its own: an
// idle unit nobody selected draws nothing until it is hovered, a selected
// one keeps the selection's look, and an ally's is still an ally's.
// Each unit stands far from the others, so a mark is found by where it lies.
import { expect, test } from "vitest";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import { screenAxes } from "@packages/renderer-core/src/camera3d";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { NOTHING_REVEALED } from "@web/battle/present/orderReveal";
import type { ObservationView } from "@web/battle/sim/observationView";
import { buildBattleOverlay } from "@apps/battle-lab/src/battleOverlay";
import { gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P3 = [number, number, number];
const units = {
  type: () => ({ mobility: {} }),
  hull: (kind: string) => (kind === "test_rifle" ? null : { half_extents_m: [6, 2, 1.5] }),
} as unknown as UnitCatalog;
const rules = { air: {}, tick_hz: 30 } as unknown as Parameters<typeof buildBattleOverlay>[1];
const facing = {
  ...screenAxes({ yaw: 0.3, pitch: 0.4 }),
  stroke: gameStroke(0.05),
  size: gameStroke(0.05),
};

const VEHICLE: P3 = [0, 0, 0];
const SQUAD: P3 = [1000, 0, 0];
const ALLIED: P3 = [2000, 0, 0];
const ALLY_ID = 3;

const vehicle = (id: number, position: P3) => ({
  id,
  kind: "test_tank",
  position,
  yaw: 0,
  hold: null,
  stock: null,
  members: [],
  memberOrders: [],
  area: null,
  goal: null,
  route: [],
  queue: [],
  state: "idle",
});
const observation = {
  tick: 0,
  own: [
    vehicle(1, VEHICLE),
    {
      ...vehicle(2, SQUAD),
      kind: "test_rifle",
      members: [
        [SQUAD[0] + 2, 0, 0],
        [SQUAD[0] - 2, 0, 0],
      ],
      memberOrders: [],
      area: { anchor: [SQUAD[0], 0], radius: 8 },
    },
    vehicle(ALLY_ID, ALLIED),
  ],
  identified: [],
  contacts: [],
  encounter: null,
  skirmish: null,
} as unknown as ObservationView;

/** The battle view's overlay with `selected` selected and `hovered` under the
 *  pointer. */
function overlay(selected: number[], hovered: number | null) {
  return buildBattleOverlay(
    units,
    rules,
    observation,
    selected,
    () => 0,
    { supplyRadius: 0, zone: null, deployments: [] },
    {
      showOrders: false,
      reveal: NOTHING_REVEALED,
      contacts: [],
      facing,
      allies: new Map([[ALLY_ID, "ally"]]),
      hovered,
    },
  );
}

/** The ground paint within 50 m of `at`, still or marching. */
function paintAt(m: WorldMeshes, at: P3): number[] {
  const out: number[] = [];
  for (const mesh of [m.painted, m.paintedMarching])
    for (let i = 0; mesh && i < mesh.length; i += VERTEX_FLOATS)
      if (Math.hypot(mesh[i] - at[0], mesh[i + 1] - at[1]) < 50)
        out.push(...mesh.subarray(i, i + VERTEX_FLOATS));
  return out;
}

test("an idle unit shows its circle only while it is hovered", () => {
  for (const [id, at] of [
    [1, VEHICLE],
    [2, SQUAD],
    [ALLY_ID, ALLIED],
  ] as const) {
    expect(paintAt(overlay([], null), at), `unit ${id}, nothing hovered`).toEqual([]);
    expect(paintAt(overlay([], id), at).length, `unit ${id} hovered`).toBeGreaterThan(0);
    expect(paintAt(overlay([], id === 1 ? 2 : 1), at), `unit ${id}, another hovered`).toEqual([]);
  }
});

test("a hovered circle is not the selection's, and a selected unit keeps the selection's", () => {
  for (const at of [VEHICLE, SQUAD]) {
    const id = at === VEHICLE ? 1 : 2;
    expect(paintAt(overlay([], id), at), `unit ${id}`).not.toEqual(
      paintAt(overlay([id], null), at),
    );
    expect(paintAt(overlay([id], id), at), `unit ${id}`).toEqual(paintAt(overlay([id], null), at));
  }
});

test("an ally's unit hovered keeps the ally's marker", () => {
  expect(paintAt(overlay([], ALLY_ID), ALLIED)).toEqual(paintAt(overlay([ALLY_ID], null), ALLIED));
});
