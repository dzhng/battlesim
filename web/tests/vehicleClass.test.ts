// @vitest-environment node
// A vehicle sounds and runs as its class, which the catalog derives from its
// physics (`contract::catalog::UnitType::vehicle_class`, carried by the view
// as `vehicle_class`; its derivation is tested there), never its unit id.
// The classes' rows are here: each ground class any catalog fields has its
// own engine sound and track gauge, and each weight of airframe its rotor.
import { presentationSettings } from "@web/presentationSettings";
import { expect, test } from "vitest";
import { gameAudio } from "@apps/battle-lab/src/soundFeed";
import { nodeCatalogSet } from "@web/battle/catalog/node";

test("the view carries each type's class: a vehicle's from its physics, none for a squad", async () => {
  const { units } = (await nodeCatalogSet("test")).units.view;
  const of = (id: string) => units.find((u) => u.id === id)!.vehicle_class;
  expect(of("test_tank")).toBe("tracked_heavy");
  expect(of("test_rifle")).toBeNull();
});

test("every ground vehicle class the game fields has its own engine sound and track gauge", async () => {
  const gauges = presentationSettings.pose.gauge as Record<string, number>;
  const classes = new Set<string>();
  for (const set of ["game", "capture", "test"] as const)
    for (const unit of (await nodeCatalogSet(set)).units.view.units)
      if (unit.vehicle_class && !unit.vehicle_class.includes("air"))
        classes.add(unit.vehicle_class);
  expect(classes.size).toBeGreaterThan(0);
  for (const cls of classes) {
    expect(gameAudio.vehicles[cls], cls).toBeDefined();
    expect(gauges[cls], cls).toBeDefined();
  }
});

test("each weight of airframe has its own rotor sound row", () => {
  for (const weight of ["light", "medium", "heavy"]) {
    const cls = `air_${weight}`;
    expect(gameAudio.vehicles[cls], cls).toBeDefined();
  }
});
