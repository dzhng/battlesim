// Opening cards moves nothing: holding Space opens every card where it
// stands, and hovering one opens it there too, every other card keeping its
// place and leader, the open card drawn above every other card, the
// high-air ones too.
import { shippedRules } from "@web/shippedRules";
import { cleanup, render as renderView } from "@testing-library/react";
import { createRef } from "react";
import { afterAll, afterEach, beforeAll, expect, test } from "vitest";
import { specimenUnit } from "@apps/battle-lab/src/panelSpecimens";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import { highAir } from "@packages/scene-assets/src/units";
import type { PanelRules } from "../src/battle/present/panelRows";
import { ReadoutLayer, type ReadoutLayerHandle } from "../src/battle/present/readouts";
import { TEST_CATALOG, WithTestCatalog } from "./catalog";

afterEach(cleanup);

/** jsdom lays nothing out: a card measures small compact and large open,
 *  as the browser's do. */
const restore: (() => void)[] = [];
beforeAll(() => {
  for (const [prop, compact, open] of [
    ["offsetWidth", 110, 260],
    ["offsetHeight", 14, 90],
  ] as const) {
    const had = Object.getOwnPropertyDescriptor(HTMLElement.prototype, prop)!;
    Object.defineProperty(HTMLElement.prototype, prop, {
      configurable: true,
      get(this: HTMLElement) {
        return this.dataset.zoom === "default" ? open : this.dataset.zoom ? compact : 0;
      },
    });
    restore.push(() => Object.defineProperty(HTMLElement.prototype, prop, had));
  }
});
afterAll(() => restore.forEach((r) => r()));

const RULES = shippedRules as unknown as PanelRules;

test("Space and hover open cards in place, moving no card, the hovered one drawn above every card", () => {
  // A cluster tight enough that an open card has no free room: ground
  // units, and a jet whose card draws in the high plane.
  const jet = TEST_CATALOG.units.view.units.find((t) => highAir(t))!.id;
  const own = [
    specimenUnit(TEST_CATALOG, "test_tank", { id: 1 }),
    specimenUnit(TEST_CATALOG, "test_apc", { id: 2 }),
    specimenUnit(TEST_CATALOG, "test_rifle", { id: 3 }),
    specimenUnit(TEST_CATALOG, jet, { id: 4 }),
  ];
  const screen: Record<number, [number, number]> = {
    1: [500, 420],
    2: [520, 360],
    3: [460, 470],
    4: [540, 330],
  };
  const handle = createRef<ReadoutLayerHandle>();
  const view = renderView(<ReadoutLayer own={own} rules={RULES} selected={[]} handle={handle} />, {
    wrapper: WithTestCatalog,
  });
  // Each unit's id rides in its anchor's x, so the projection knows which.
  const at = (u: number) => [u * 1000, 0, 0] as const;
  const project = (x: number) => screen[Math.round(x / 1000)] ?? null;
  const drawn = { own: new Map(own.map((u) => [u.id, at(u.id)])) };
  const card = (u: number) => view.container.querySelector<HTMLElement>(`[data-unit="${u}"]`)!;
  const leader = (u: number) => card(u).parentElement!.querySelector("path.ro-leader")!;
  const read = () =>
    new Map(
      own.map((u) => [
        u.id,
        {
          shown: card(u.id).style.display !== "none",
          at: card(u.id).style.transform,
          leader: leader(u.id).getAttribute("d"),
          zoom: card(u.id).dataset.zoom,
          z: Number(card(u.id).parentElement!.style.zIndex),
        },
      ]),
    );
  /** A leader's ring and its line up to the card's corner. */
  const rise = (d: string | null) => d!.slice(0, d!.lastIndexOf(" L "));
  const place = (hover: number | null, space: boolean) =>
    handle.current!.place(
      project,
      gameCamera.opening(),
      drawn,
      null,
      space,
      hover === null ? null : { x: -100, y: -100, unit: hover, enemy: null },
    );

  // Holding Space opens every card where it stands: each leader rises to
  // the same corner as compact.
  place(null, false);
  const compact = read();
  place(null, true);
  for (const [id, open] of read()) {
    expect(open.zoom, `card ${id} with Space`).toBe("default");
    expect(rise(open.leader), `card ${id}'s leader with Space`).toBe(rise(compact.get(id)!.leader));
  }
  for (const space of [false, true]) {
    place(null, space);
    const before = read();
    expect([...before.values()].every((c) => c.shown)).toBe(true);
    if (!space) expect([...before.values()].every((c) => c.zoom === "compressed")).toBe(true);
    for (const hovered of [1, 2, 3, 4]) {
      place(hovered, space);
      const open = read();
      for (const [id, was] of before) {
        const now = open.get(id)!;
        const state = `card ${id}, ${hovered} hovered, space ${space}`;
        expect(now.shown, state).toBe(true);
        if (id === hovered) {
          expect(now.zoom, state).toBe("default");
          // It opens away from its unit: its leader rises to the same corner,
          // and only its underline runs on along the wider card.
          expect(rise(now.leader), state).toBe(rise(was.leader));
        } else {
          expect(now.zoom, state).toBe(was.zoom);
          expect(now.at, state).toBe(was.at);
          expect(now.leader, state).toBe(was.leader);
          expect(open.get(hovered)!.z, `${hovered} over ${id}, space ${space}`).toBeGreaterThan(
            now.z,
          );
        }
      }
    }
    // The pointer leaves: everything is as it was.
    place(null, space);
    expect(read()).toEqual(before);
  }
});
