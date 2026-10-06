// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { initSync, resolve_catalog } from "@wasm/game_wasm.js";
import { UnitCatalog, type CatalogView } from "@packages/scene-assets/src/units";

test("a planned unit is a card the browser shows but cannot field", () => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  const documents = [
    { roles: { air: { name: "Air", description: "", symbol: ["air"] } } },
    {
      units: {
        test_jet: {
          name: "Test jet",
          description: "Test only.",
          faction: "us",
          family: "test_jet",
          roles: [],
          cost: 400,
          roster: { factions: ["us", "europe"], category: "air", variant: "A", family_name: "Jet" },
          planned: { reason: "Not flown yet", profile: "fighter", weapons: [] },
        },
      },
    },
  ];
  const catalog = new UnitCatalog(
    JSON.parse(resolve_catalog(JSON.stringify(documents))) as CatalogView,
  );
  expect(catalog.cards.find((card) => card.id === "test_jet")).toMatchObject({
    roster: { factions: ["us", "europe"], family_name: "Jet" },
    disabled_reason: "Not flown yet",
  });
  expect(catalog.has("test_jet")).toBe(false);
  expect(() => catalog.type("test_jet")).toThrow("no unit type test_jet");
});
