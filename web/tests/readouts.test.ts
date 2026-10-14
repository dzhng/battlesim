// @vitest-environment node
import { expect, test } from "vitest";
import { readFileSync } from "node:fs";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { labScenario, UNITS } from "./catalog";
import { loadMap } from "@web/maps/node";
import type { ObservationLayout } from "../src/battle/sim/observationView";
import { easeNudge } from "../src/battle/present/readouts";
import { REASON_MARK } from "../src/battle/present/infoPanel";
import { REASON_TEXT } from "@apps/battle-lab/src/reasonText";
import { ownStateRows, type PanelRules } from "../src/battle/present/panelRows";
import type { OwnUnitView } from "../src/battle/sim/observationView";

const geometry = loadMap("geometry").definition;

test("every published action reason has player words and a mark, and every garrison phase a row", () => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  const battle = new Battle(labScenario(geometry, []), 1);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  battle.free();
  const reasons = layout.actionReasons;
  expect(reasons.length).toBeGreaterThan(10);
  for (const v of reasons) expect(REASON_TEXT[v], v).toBeTruthy();
  // And a panel mark, or a decision to show none.
  for (const v of reasons) expect(REASON_MARK[v], v).not.toBeUndefined();
  const phases = layout.garrisonPhases;
  expect(phases.length).toBeGreaterThan(2);
  const rules = { tick_hz: 30, weapons: {}, service: { radius_m: 1 } } as PanelRules;
  for (const phase of phases) {
    const u = { garrison: { phase, progress: 0.5 }, suppression: "none", service: "", stock: null };
    expect(ownStateRows(UNITS, u as unknown as OwnUnitView, [], rules), phase).toHaveLength(1);
  }
});

test("a callout eases to a new nudge over about 150 ms of presentation clock, and snaps under a held one", () => {
  const from = { dx: 0, dy: 0 },
    to = { dx: 0, dy: -40 };
  // A frame at 60 Hz moves it part of the way, never past.
  const step = easeNudge(from, to, 1 / 60);
  expect(step.dy).toBeLessThan(0);
  expect(step.dy).toBeGreaterThan(-40);
  // Nine such frames (150 ms) put it within a few pixels.
  let n = from;
  for (let k = 0; k < 9; k++) n = easeNudge(n, to, 1 / 60);
  expect(Math.abs(n.dy - to.dy)).toBeLessThan(3);
  // A held clock, a rewound one or a first placement: the settled layout.
  expect(easeNudge(from, to, 0)).toEqual(to);
  expect(easeNudge(from, to, -1)).toEqual(to);
  expect(easeNudge(undefined, to, 1 / 60)).toEqual(to);
});
