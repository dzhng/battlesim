// @vitest-environment node
// A vehicle's presentation class comes from its physics (how it moves, how
// heavy its hull is, whether it is a truck), never its unit id, so a new
// vehicle sounds and articulates as its kind with no table edit.
import { expect, test } from "vitest";
import { gameAudio } from "@apps/battle-lab/src/soundFeed";
import game from "@fixtures/game.json";
import {
  vehicleClass,
  type Hull,
  type UnitType,
  type WeightClass,
} from "@packages/scene-assets/src/units";

type Physics = Pick<UnitType, "body" | "mobility" | "roles">;

/** A fake vehicle: only what its class is derived from. */
function fake(mobility: "tracked" | "wheeled", weight: WeightClass, roles: string[] = []): Physics {
  const drive = { offroad_kmh: 30, road_kmh: 60, turn_deg_s: 30, reverse_fraction: 0.3 };
  return {
    body: { hull: { weight_class: weight } as Hull },
    mobility:
      mobility === "tracked" ? { tracked: drive } : { wheeled: { ...drive, turning_radius_m: 8 } },
    roles,
  };
}

test("a vehicle's class is its mobility, its hull's weight and whether it hauls supply", () => {
  expect(vehicleClass(fake("tracked", "heavy", ["mbt"]))).toBe("tracked_heavy");
  expect(vehicleClass(fake("wheeled", "medium", ["recon"]))).toBe("wheeled_medium");
  expect(vehicleClass(fake("wheeled", "medium", ["logistics"]))).toBe("wheeled_medium_logistics");
});

test("an aircraft's class is air and its hull's weight; it has no running gear", () => {
  const heli: Physics = {
    body: { hull: { weight_class: "light" } as Hull },
    mobility: { air: { cruise_kmh: 220, turn_deg_s: 90, climb_mps: 6 } },
    roles: ["light_vehicle"],
  };
  expect(vehicleClass(heli)).toBe("air_light");
});

test("a squad has no vehicle class", () => {
  const squad: Physics = {
    body: { squad: { slots: ["test_rifleman"] } },
    mobility: { foot: { offroad_kmh: 5, road_kmh: 6 } },
    roles: ["infantry"],
  };
  expect(vehicleClass(squad)).toBeNull();
});

test("each kind of hull the game fields has its own engine sound and track gauge", () => {
  const kinds = [
    fake("tracked", "heavy"),
    fake("tracked", "medium"),
    fake("wheeled", "light"),
    fake("wheeled", "medium"),
    fake("wheeled", "medium", ["logistics"]),
  ];
  for (const kind of kinds) {
    const cls = vehicleClass(kind)!;
    expect(gameAudio.vehicles[cls], cls).toBeDefined();
    expect((game.presentation.pose.gauge as Record<string, number>)[cls], cls).toBeDefined();
  }
});

test("each weight of airframe has its own rotor sound row", () => {
  for (const weight of ["light", "medium", "heavy"] as const) {
    const cls = vehicleClass({
      body: { hull: { weight_class: weight } as Hull },
      mobility: { air: { cruise_kmh: 220, turn_deg_s: 90, climb_mps: 6 } },
      roles: ["light_vehicle"],
    })!;
    expect(gameAudio.vehicles[cls], cls).toBeDefined();
  }
});
