// The mechanics editor's server as the tests fake it: the snapshot it loads
// (every editable unit, the test units included), a structural variant, and
// a JSON reply.
import { shippedRules } from "@web/shippedRules";
import gameCatalog from "../../../fixtures/catalog.json";
import infantry from "../../../fixtures/units/test/infantry.json";
import soldiers from "../../../fixtures/units/test/soldiers.json";
import tanks from "../../../fixtures/units/test/tanks.json";
import type { JsonObject, MechanicsSnapshot } from "../../../apps/mechanics-editor/src/protocol";
import { UNITS, WEAPONS } from "../catalog";

/** Every editable unit, the test units included, as the editor resolves them. */
export const catalog = { ...UNITS.view, weapons: WEAPONS };

export const snapshot: MechanicsSnapshot = {
  revision: "test-one",
  catalog: catalog as unknown as JsonObject,
  game: gameCatalog as unknown as JsonObject,
  documents: [
    { path: "fixtures/units/test/infantry.json", value: infantry },
    { path: "fixtures/units/test/soldiers.json", value: soldiers },
    { path: "fixtures/units/test/tanks.json", value: tanks as unknown as JsonObject },
    { path: "fixtures/rules.json", value: shippedRules },
  ],
};
export const structuralSnapshot = (): MechanicsSnapshot => {
  const fixture = structuredClone(snapshot);
  delete fixture.catalog.documents;
  const units = fixture.catalog.units as JsonObject[];
  units.find((unit) => unit.id === "test_rifle")!.body = {
    squad: { slots: ["test_rifleman", "test_grenadier"] },
  };
  fixture.catalog.soldiers = {
    test_rifleman: { hp: 100, mounts: [{ id: "rifles", weapons: ["rifle"] }] },
    test_grenadier: { hp: 100, mounts: [{ id: "launcher", weapons: ["grenade"] }] },
  };
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  weapons.rifle.magazine = { rounds: 30, shot_interval_s: 0.1, burst: null };
  return fixture;
};
export const respond = (value: unknown) =>
  new Response(JSON.stringify(value), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
