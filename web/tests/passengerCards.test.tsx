// A carrier's card says its seats, and the squads riding it show as compact
// cards under it: the Seats row from the passengers' published holds and the
// catalog's seats, and the nested callout the battle's readout layer draws.
import { shippedRules } from "@web/shippedRules";
import { cleanup, render as renderView } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, expect, test } from "vitest";
import { UnitCatalog } from "@packages/scene-assets/src/units";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import { enemyPanel, ownPanel, type PanelRules } from "../src/battle/present/panelRows";
import { ReadoutLayer, type ReadoutLayerHandle } from "../src/battle/present/readouts";
import type { OwnUnitView } from "../src/battle/sim/observationView";
import { TEST_CATALOG, WithTestCatalog } from "./catalog";

afterEach(cleanup);

const RULES = shippedRules as unknown as PanelRules;

/** The test jeep given `seats`, so the test sets the bound it reads. */
function carrying(seats: number): UnitCatalog {
  const view = TEST_CATALOG.units.view;
  return new UnitCatalog({
    ...view,
    units: view.units.map((t) =>
      t.id === "test_jeep" ? { ...t, capabilities: { ...t.capabilities, carriage: { seats } } } : t,
    ),
  });
}

const aboard = (id: number, carrier: number, memberHp: number[], phase = "inside") =>
  specimenUnit(TEST_CATALOG, "test_rifle", {
    id,
    memberHp,
    hold: { host: { kind: "carrier", id: carrier }, phase, progress: phase === "inside" ? 1 : 0.5 },
  });

const seatsOf = (units: UnitCatalog, carrier: OwnUnitView, own: OwnUnitView[]) =>
  ownPanel(units, carrier, own, RULES).states.filter((r) => r.state === "seats");

test("a carrier's card counts the living soldiers of the squads it holds against its seats", () => {
  const units = carrying(8);
  const carrier = specimenUnit(TEST_CATALOG, "test_jeep", { id: 1 });
  const other = specimenUnit(TEST_CATALOG, "test_jeep", { id: 2 });
  const own = [
    carrier,
    other,
    // Five aboard, one of them dead; two more boarding; three in the other.
    aboard(10, 1, [100, 100, 0, 60, 100]),
    aboard(11, 1, [100, 100], "entering"),
    aboard(12, 2, [100, 100, 100]),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 13 }),
  ];
  const [seats] = seatsOf(units, carrier, own);
  expect(seats.word).toBe("SEATS 6/8");
  // A count only: no pips, and a full carrier is not a warning.
  expect(seats.fill).toBeNull();
  expect(seats.tone).toBeNull();
  expect(seatsOf(units, other, own)[0].word).toBe("SEATS 3/8");
});

test("an empty carrier still says its seats; a unit with none says nothing of them", () => {
  const units = carrying(16);
  const carrier = specimenUnit(TEST_CATALOG, "test_jeep", { id: 1 });
  expect(seatsOf(units, carrier, [carrier]).map((r) => r.word)).toEqual(["SEATS 0/16"]);
  const tank = specimenUnit(TEST_CATALOG, "test_tank", { id: 3 });
  expect(seatsOf(units, tank, [tank])).toEqual([]);
});

test("an enemy carrier's card never shows seats", () => {
  const units = carrying(8);
  expect(enemyPanel(units, "test_jeep", RULES).states).toEqual([]);
});

test("a passenger's card says nothing of a building it is not in", () => {
  const units = carrying(8);
  const carrier = specimenUnit(TEST_CATALOG, "test_jeep", { id: 1 });
  const rider = aboard(10, 1, [100, 100]);
  const words = ownPanel(units, rider, [carrier, rider], RULES).states.map((r) => r.word);
  expect(words.filter((w) => /BUILDING|ENTERING|LEAVING/.test(w))).toEqual([]);
});

/** The readout layer over `own`, placed once with every card hanging from
 *  one screen point, as a carrier and its riders do. `hover` is a unit the
 *  pointer is on, or "card" for the pointer on the stack's cards (jsdom
 *  lays every box at the origin). */
function placed(own: OwnUnitView[], expanded: boolean, hover: number | "card" | null = null) {
  const handle = createRef<ReadoutLayerHandle>();
  const view = renderView(<ReadoutLayer own={own} rules={RULES} selected={[]} handle={handle} />, {
    wrapper: WithTestCatalog,
  });
  const hovered =
    hover === null
      ? null
      : hover === "card"
        ? { x: 0, y: 0, unit: null, enemy: null }
        : { x: -100, y: -100, unit: hover, enemy: null };
  handle.current!.place(() => [500, 400], gameCamera.opening(), {}, null, expanded, hovered);
  return view;
}

test("each passenger's compact card stacks under its carrier's, and stays compact when the carrier's opens", () => {
  const carrier = specimenUnit(TEST_CATALOG, "test_apc", { id: 1 });
  const own = [carrier, aboard(10, 1, [100, 100, 100]), aboard(11, 1, [100, 100])];
  // Compact; Space held; the carrier hovered; a rider's card hovered.
  for (const [expanded, hover] of [
    [false, null],
    [true, null],
    [false, 1],
    [false, "card"],
  ] as const) {
    const view = placed(own, expanded, hover);
    const card = view.container.querySelector<HTMLElement>('[data-unit="1"]')!;
    // The riders ride in the carrier's callout, in the side's order, and
    // have no callout of their own.
    const riders = [...card.querySelectorAll<HTMLElement>("[data-passenger]")];
    expect(riders.map((r) => r.dataset.unit)).toEqual(["10", "11"]);
    expect(view.container.querySelectorAll(".ro-callout")).toHaveLength(1);
    const state = `expanded ${expanded} hover ${hover}`;
    for (const r of riders) expect(r.dataset.zoom, state).toBe("compressed");
    // The carrier's own card opens as before.
    expect(card.dataset.zoom, state).toBe(expanded || hover !== null ? "default" : "compressed");
    cleanup();
  }
});

test("a squad on foot keeps its own callout beside a carrier", () => {
  const own = [
    specimenUnit(TEST_CATALOG, "test_apc", { id: 1 }),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 12 }),
  ];
  const view = placed(own, false);
  expect(view.container.querySelectorAll(".ro-callout")).toHaveLength(2);
  expect(view.container.querySelector("[data-passenger]")).toBeNull();
});

test("a rider's card picks its carrier, never the rider", () => {
  const carrier = specimenUnit(TEST_CATALOG, "test_apc", { id: 1 });
  const handle = createRef<ReadoutLayerHandle>();
  renderView(
    <ReadoutLayer
      own={[carrier, aboard(10, 1, [100, 100, 100])]}
      rules={RULES}
      selected={[]}
      handle={handle}
    />,
    { wrapper: WithTestCatalog },
  );
  handle.current!.place(() => [500, 400], gameCamera.opening(), {}, null, false, null);
  const rider = document.querySelector<HTMLElement>("[data-passenger]")!.getBoundingClientRect();
  // The point is on the rider's card, which lies on its carrier's.
  expect(handle.current!.pick(rider.left, rider.top)).toEqual({
    unit: 1,
    enemy: null,
    contact: null,
  });
  expect(handle.current!.inRect(10, { x0: -1e6, x1: 1e6, y0: -1e6, y1: 1e6 })).toBe(false);
});

test("a rider's card takes its own owner's tone, as every card does", () => {
  // The ally's carrier 2 holds its own squad 20; your carrier 1 holds your
  // squad 10 and the ally's squad 11 (any squad on the side may board).
  const ally = new Map([
    [2, "BLUE 2 · AI"],
    [11, "BLUE 2 · AI"],
    [20, "BLUE 2 · AI"],
  ]);
  const view = renderView(
    <ReadoutLayer
      own={[
        specimenUnit(TEST_CATALOG, "test_apc", { id: 1 }),
        aboard(10, 1, [100, 100]),
        aboard(11, 1, [100, 100]),
        specimenUnit(TEST_CATALOG, "test_apc", { id: 2 }),
        aboard(20, 2, [100, 100]),
      ]}
      allies={ally}
      rules={RULES}
      selected={[]}
      handle={createRef<ReadoutLayerHandle>()}
    />,
    { wrapper: WithTestCatalog },
  );
  const tone = (id: number) =>
    view.container.querySelector<HTMLElement>(`[data-unit="${id}"]`)!.dataset.owner;
  expect([1, 10, 11, 2, 20].map(tone)).toEqual(["own", "own", "ally", "ally", "ally"]);
});
