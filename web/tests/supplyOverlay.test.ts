// @vitest-environment node
// The supply overlay is contextual: a stocked supply vehicle's reach shows
// while that vehicle is selected, or every one's while Space is held, and
// nothing is drawn under the units it serves (their panels say it instead).
import { expect, test } from "vitest";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observation";
import { supplyLayer } from "@apps/battle-lab/src/battleOverlay";
import { validateSupplyStyle, type SupplyStyle } from "@packages/battle-renderer/src/supplyOverlay";

const flat = () => 0;
const RADIUS = 170;

const own = (u: Partial<OwnUnitView>) =>
  ({
    position: [0, 0, 0],
    stock: null,
    service: "full",
    state: "idle",
    deployment: null,
    ...u,
  }) as OwnUnitView;

const truck = own({
  id: 1,
  kind: "supply",
  position: [100, 100, 0],
  stock: 40,
  deployment: { progress: 1, target: "deployed" } as OwnUnitView["deployment"],
});
const served = own({ id: 2, kind: "rifle", position: [130, 100, 0], service: "serving" });
const waiting = own({ id: 3, kind: "tank", position: [70, 100, 0], service: "moving" });
const observation = { own: [truck, served, waiting] } as unknown as ObservationView;

/** Distances from `c` of every painted vertex. */
function reach(selected: readonly number[], c: readonly [number, number], all = false) {
  const mesh =
    supplyLayer(observation, RADIUS, flat, selected, undefined, all).painted ?? new Float32Array(0);
  const out: number[] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push(Math.hypot(mesh[i] - c[0], mesh[i + 1] - c[1]));
  return out;
}

test("nothing is drawn while no supply vehicle is selected", () => {
  expect(reach([], [100, 100])).toEqual([]);
  // A served unit selected on its own: still nothing on the ground.
  expect(reach([served.id, waiting.id], [100, 100])).toEqual([]);
});

test("a selected supply vehicle, or every stocked one with Space held, shows its reach and only its reach", () => {
  for (const d of [reach([truck.id], [100, 100]), reach([], [100, 100], true)]) {
    expect(d.length).toBeGreaterThan(0);
    // Every vertex lies on the reach ring: no ring under a unit it serves.
    for (const r of d) expect(Math.abs(r - RADIUS)).toBeLessThan(2);
  }
});

test("a supply style without its reach colour is refused", () => {
  expect(() => validateSupplyStyle({} as SupplyStyle)).toThrow(/reach/);
});
