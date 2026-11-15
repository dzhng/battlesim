// A capture area reads as the viewer's or the enemy's, whichever side the
// player plays: its ring on the ground, its floating flag and its flag in the
// score strip all take the HUD's own-side colour for the viewer's side and
// the enemy colour for the other, never a fixed colour per side. A held one
// gives off a faint curtain of that light, rising from its ring and fading out.
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { createRef } from "react";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { screenAxes } from "@packages/renderer-core/src/camera3d";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { gameHud } from "@web/battle/present/hudTheme";
import { NOTHING_REVEALED } from "@web/battle/present/orderReveal";
import {
  ObjectiveMarkers,
  type ObjectiveMarkersHandle,
} from "@web/battle/present/objectiveMarkers";
import type { ObjectiveView, ObservationView } from "@web/battle/sim/observationView";
import type { SideName } from "@web/battle/sim/protocol";
import { battleOverlayParts } from "@apps/battle-lab/src/battleOverlay";
import { SkirmishStatus } from "@apps/battle-lab/src/battleStatus";
import { gameStroke } from "@apps/battle-lab/src/gameOverlay";

afterEach(cleanup);

const objective = (id: string, owner: "blue" | "red" | null, x: number): ObjectiveView => ({
  id,
  center: [x, 0],
  radiusM: 50,
  owner,
  contested: false,
  capturing: null,
  captureProgress: 0,
});
// Blue holds the first, Red the second.
const OBJECTIVES = [objective("alpha", "blue", 0), objective("bravo", "red", 1000)];

/** Each of `objectives`' marks, as `viewer` sees them. */
function objectiveParts(viewer: SideName, objectives: ObjectiveView[]) {
  const o = {
    tick: 0,
    own: [],
    identified: [],
    contacts: [],
    encounter: null,
    skirmish: { objectives },
    projectiles: [],
    guided: [],
  } as unknown as ObservationView;
  const parts = battleOverlayParts(
    { type: () => ({ mobility: {} }) } as unknown as UnitCatalog,
    { air: {}, tick_hz: 30 } as never,
    o,
    [],
    () => 0,
    { side: viewer, supplyRadius: 170, zone: null, deployments: [] },
    {
      showOrders: false,
      reveal: NOTHING_REVEALED,
      contacts: [],
      facing: {
        ...screenAxes({ yaw: 0, pitch: 0.4 }),
        stroke: gameStroke(0.05),
        size: gameStroke(0.05),
      },
    },
  );
  return parts.slice(0, objectives.length);
}

/** The rgb each objective's ring is painted in, as `viewer` sees it. */
function ringColours(viewer: SideName) {
  return objectiveParts(viewer, OBJECTIVES).map((part) => {
    const mesh = part.painted!;
    expect(mesh.length).toBeGreaterThan(0);
    return Array.from(mesh.slice(6, 9)).map((c) => Number(c.toFixed(3)));
  });
}
const rgb = (c: readonly number[]) => c.map((v) => Number(Math.fround(v).toFixed(3)));

test("a held capture area's ring is the viewer's colour when its side holds it, the enemy's otherwise", () => {
  for (const [viewer, ownIndex] of [
    ["blue", 0],
    ["red", 1],
  ] as const) {
    const rings = ringColours(viewer);
    expect(rings[ownIndex]).toEqual(rgb(gameHud.accent));
    expect(rings[1 - ownIndex]).toEqual(rgb(gameHud.enemy));
  }
});

test("a capture area's flags read as the viewer's when its side holds it", () => {
  const handle = createRef<ObjectiveMarkersHandle>();
  for (const viewer of ["blue", "red"] as const) {
    const view = render(
      <>
        <ObjectiveMarkers objectives={OBJECTIVES} viewer={viewer} handle={handle} />
        <SkirmishStatus
          match={{
            phase: "active",
            players: [],
            preparationRemainingS: 0,
            scores: [0, 0],
            result: null,
            objectives: OBJECTIVES,
            report: null,
          }}
          viewer={viewer}
        />
      </>,
    );
    const held = OBJECTIVES.find((o) => o.owner === viewer)!;
    const index = OBJECTIVES.indexOf(held);
    const marker = view.getByLabelText(`Capture area ${index + 1}`);
    const flag = view.getByLabelText(`Objective ${index + 1}: yours`);
    const enemyMarker = view.getByLabelText(`Capture area ${2 - index}`);
    const enemyFlag = view.getByLabelText(`Objective ${2 - index}: enemy`);
    expect(marker.dataset.owner).toBe(flag.dataset.owner);
    expect(enemyMarker.dataset.owner).toBe(enemyFlag.dataset.owner);
    expect(marker.dataset.owner).not.toBe(enemyMarker.dataset.owner);
    // The same standing whichever side the player plays.
    expect(marker.dataset.owner).toBe("own");
    expect(enemyMarker.dataset.owner).toBe("enemy");
    cleanup();
  }
});

test("a held capture area gives off light that rises from its ring in its colour and fades out; an unheld one gives off none", () => {
  const [held, unheld] = objectiveParts("blue", [
    objective("alpha", "blue", 0),
    objective("bravo", null, 1000),
  ]);
  expect(unheld.translucent?.length ?? 0).toBe(0);
  const curtain = held.translucent!;
  const ring = Array.from(held.painted!.slice(6, 9));
  const vertices = [];
  for (let i = 0; i < curtain.length; i += VERTEX_FLOATS) {
    vertices.push({
      z: curtain[i + 2],
      rgb: Array.from(curtain.slice(i + 6, i + 9)),
      a: curtain[i + 9],
    });
    // It stands on the ring's edge.
    expect(Math.hypot(curtain[i], curtain[i + 1])).toBeLessThanOrEqual(50);
    expect(Math.hypot(curtain[i], curtain[i + 1])).toBeGreaterThan(45);
  }
  for (const v of vertices) expect(v.rgb).toEqual(ring);
  const top = Math.max(...vertices.map((v) => v.z));
  expect(top).toBeGreaterThan(0);
  // Strongest at the ground, gone at its top, and never brighter higher up.
  const byHeight = [...new Map(vertices.map((v) => [v.z, v.a])).entries()].sort(
    (a, b) => a[0] - b[0],
  );
  expect(byHeight[0]).toEqual([0, expect.any(Number)]);
  expect(byHeight[0][1]).toBeGreaterThan(0);
  expect(byHeight.at(-1)![1]).toBe(0);
  for (let k = 1; k < byHeight.length; k++) expect(byHeight[k][1]).toBeLessThan(byHeight[k - 1][1]);
});
