// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { TEST_RULES } from "./catalog";

wasm.initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });

test("the city stress boundary keeps the full map and authoritative early/late load", () => {
  const map = { size: [3200, 2200], fog_cell_m: 8, height_grid_m: 4, slope_cutoff_deg: 35 };
  const early = JSON.parse(
    wasm.city_stress_preparation(JSON.stringify(map), JSON.stringify(TEST_RULES), 4n, false),
  );
  const late = JSON.parse(
    wasm.city_stress_preparation(JSON.stringify(map), JSON.stringify(TEST_RULES), 4n, true),
  );
  expect(early.report.livingUnits).toEqual({ blue: 100, red: 100 });
  expect(late.report.livingUnits).toEqual(early.report.livingUnits);
  const earlySetup = early.scenario;
  const lateSetup = late.scenario;
  expect(earlySetup.map.size).toEqual(map.size);
  expect(lateSetup.map.size).toEqual(map.size);
  for (const setup of [earlySetup, lateSetup]) {
    for (const side of ["blue", "red"])
      expect(
        setup.units.filter(
          (u: { side: string; condition?: unknown }) => u.side === side && u.condition == null,
        ),
      ).toHaveLength(100);
    expect(setup.scripts).toEqual(earlySetup.scripts);
  }
  expect(
    lateSetup.units.reduce(
      (n: number, u: { condition?: { casualties?: number } }) => n + (u.condition?.casualties ?? 0),
      0,
    ),
  ).toBe(20000);
  expect(lateSetup.map.props.length - earlySetup.map.props.length).toBe(2000);
});
