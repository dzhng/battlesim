// @vitest-environment node
import { expect, test } from "vitest";
import { readFileSync } from "node:fs";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { labScenario, TEST_CATALOG, UNITS } from "./catalog";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
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
  const phases = layout.holdPhases;
  expect(phases.length).toBeGreaterThan(2);
  const rules = { tick_hz: 30, weapons: {}, service: { radius_m: 1 } } as PanelRules;
  for (const phase of phases) {
    const u = {
      kind: "test_rifle",
      hold: {
        host: { kind: "building", id: 3, center: [0, 0], half: [10, 10], seats: 12 },
        phase,
        progress: 0.5,
      },
      suppression: "none",
      service: "",
      stock: null,
    };
    expect(ownStateRows(UNITS, u as unknown as OwnUnitView, [], rules), phase).toHaveLength(1);
  }
});

test("a house's row counts the living soldiers of every own squad holding it against its published seats", () => {
  const rules = { tick_hz: 30, weapons: {}, service: { radius_m: 1 } } as PanelRules;
  const house = (id: number, seats: number) => ({
    kind: "building" as const,
    id,
    center: [0, 0] as [number, number],
    half: [10, 10] as [number, number],
    seats,
  });
  const holding = (id: number, host: ReturnType<typeof house>, phase: string, memberHp: number[]) =>
    specimenUnit(TEST_CATALOG, "test_rifle", {
      id,
      memberHp,
      hold: { host, phase, progress: phase === "inside" ? 1 : 0.5 },
    });
  const own = [
    // Four inside (two wounded), two stepping in, two stepping out.
    holding(1, house(3, 12), "inside", [100, 40, 60, 100]),
    holding(2, house(3, 12), "entering", [100, 100]),
    holding(3, house(3, 12), "exiting", [100, 40]),
    // Another house, and a squad in the open, take none of these seats.
    holding(4, house(5, 9), "inside", [100, 100, 100, 100, 100]),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 6, memberHp: [100, 100] }),
  ];
  /** The row's used and total seats, read from its numbers alone. */
  const seats = (u: OwnUnitView) => {
    const rows = ownStateRows(UNITS, u, own, rules).filter((r) => r.state === "in_building");
    return rows.map((r) => r.word.match(/\d+/g)!.map(Number));
  };
  // Stepping in and out count, as a carrier's Seats row counts boarding and
  // getting out; the total is the one the simulation published.
  expect(seats(own[0])).toEqual([[8, 12]]);
  expect(seats(own[3])).toEqual([[5, 9]]);
  // Squads still stepping in or out keep their timer rows instead.
  expect(seats(own[1])).toEqual([]);
  expect(seats(own[2])).toEqual([]);
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
