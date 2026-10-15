// @vitest-environment node
// An aircraft's ground marker and drop line say where a hovering airframe
// stands: a rotor aircraft has both, in any layer; a winged one never stops
// over a point and has neither. The flight model decides, never the layer.
import { expect, test } from "vitest";
import type { ObservationView } from "@web/battle/sim/observationView";
import { aircraftLayer } from "@apps/battle-lab/src/battleOverlay";
import type { UnitCatalog } from "@packages/scene-assets/src/units";

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

const marks = (kind: keyof typeof KINDS) => {
  const o = {
    identified: [{ kind, position: [500, 500, 120], yaw: 0 }],
  } as unknown as ObservationView;
  const m = aircraftLayer(units, o, flat);
  return { marker: m.painted?.length ?? 0, dropLine: m.opaque.length };
};

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
