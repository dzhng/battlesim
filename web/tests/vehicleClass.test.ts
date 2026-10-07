// @vitest-environment node
// A vehicle's presentation class comes from its physics (how it moves, how
// heavy its hull is, whether it is a truck), never its unit id, so a new
// vehicle sounds and articulates as its kind with no table edit.
import { expect, test } from "vitest";
import { gameAudio } from "@apps/battle-lab/src/soundFeed";
import { gamePose } from "@apps/battle-lab/src/poseFeed";
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

test("a squad has no vehicle class", () => {
  const squad: Physics = {
    body: { squad: { slots: ["rifleman"] } },
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
    expect(gamePose.gauge[cls], cls).toBeDefined();
  }
});
