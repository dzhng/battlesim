// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { initSync, resolve_catalog } from "@wasm/game_wasm.js";
import shipped from "@fixtures/catalog.json";
import { UnitCatalog, type CatalogView } from "@packages/scene-assets/src/units";

test("native and browser admission publish the same disabled roster", () => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  const resolved = JSON.parse(resolve_catalog(JSON.stringify(shipped.documents))) as CatalogView;
  expect(resolved.cards).toEqual(shipped.cards);
  const catalog = new UnitCatalog(resolved);
  expect(catalog.cards.find((card) => card.id === "f_35a")).toMatchObject({
    roster: { factions: ["us", "europe"], family_name: "F-35 Lightning II" },
    disabled_reason: expect.any(String),
  });
  expect(catalog.has("f_35a")).toBe(false);
  expect(() => catalog.type("f_35a")).toThrow("no unit type f_35a");
});
