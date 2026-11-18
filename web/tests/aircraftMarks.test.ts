// @vitest-environment node
// An aircraft's ground marker and drop line say where a hovering airframe
// stands: a flying rotor aircraft has both, in any layer; a winged one never
// stops over a point and has neither. A grounded rotor wears the vehicle's mark.
import { expect, test } from "vitest";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observationView";
import { aircraftLayer, orderLayer } from "@apps/battle-lab/src/battleOverlay";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { orderView } from "./support/orderMesh";

const flat = () => 0;
const KINDS = {
  rotor_low: { layer: "low_air", flight: "rotor" },
  rotor_high: { layer: "high_air", flight: "rotor" },
  wing_low: { layer: "low_air", flight: "fixed_wing" },
  wing_high: { layer: "high_air", flight: "fixed_wing" },
} as const;

const units = {
  type: (kind: keyof typeof KINDS) => ({
    mobility: { air: { cruise_kmh: 200, turn_deg_s: 30, climb_mps: 8, ...KINDS[kind] } },
  }),
  hull: () => ({ half_extents_m: [6, 2, 1.5] }),
} as unknown as UnitCatalog;

const marks = (kind: keyof typeof KINDS, grounded = false) => {
  const o = {
    identified: [{ kind, position: [500, 500, 120], yaw: 0, grounded }],
  } as unknown as ObservationView;
  const m = aircraftLayer(units, o, flat);
  return { marker: m.painted?.length ?? 0, dropLine: m.opaque.length };
};

test("touchdown gives a selected rotor aircraft one ground-vehicle marker and no drop line", () => {
  // Keep its published position above the surface: the grounded state, not
  // a height guess, must remove the line.
  const unit = { ...orderView({ position: [0, 0, 2], reveal: 0 }), id: 1, kind: "rotor_low" };
  const draw = (grounded: boolean, catalog = units, selected = [1]) =>
    orderLayer(
      catalog,
      {} as Parameters<typeof orderLayer>[1],
      { own: [{ ...unit, grounded } as unknown as OwnUnitView] } as ObservationView,
      selected,
      new Map(),
      flat,
    );
  const flying = draw(false);
  const landed = draw(true);
  const groundCatalog = { ...units, type: () => ({ mobility: {} }) } as unknown as UnitCatalog;
  const vehicle = draw(true, groundCatalog);
  expect(flying.opaque.length).toBeGreaterThan(0);
  expect(flying.painted!.length).toBeGreaterThan(0);
  expect(landed.opaque.length).toBe(0);
  expect(landed.painted).toEqual(vehicle.painted);
  expect(landed.painted).toEqual(flying.painted);
  expect(draw(false, units, []).painted!.length).toBeGreaterThan(0);
  expect(draw(true, units, []).painted?.length ?? 0).toBe(0);
});

test("a grounded identified aircraft has no always-shown air marker or drop line", () => {
  expect(marks("rotor_low", true)).toEqual({ marker: 0, dropLine: 0 });
});

test("a rotor aircraft has a ground marker and drop line in either air layer", () => {
  for (const kind of ["rotor_low", "rotor_high"] as const) {
    const { marker, dropLine } = marks(kind);
    expect(marker, kind).toBeGreaterThan(0);
    expect(dropLine, kind).toBeGreaterThan(0);
  }
});

test("a fixed-wing aircraft has neither, in either air layer", () => {
  for (const kind of ["wing_low", "wing_high"] as const)
    expect(marks(kind), kind).toEqual({ marker: 0, dropLine: 0 });
});
