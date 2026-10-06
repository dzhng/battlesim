// @vitest-environment node
// The panel workbench shows every panel the battle can draw: every row kind
// the derivation (`panelRows.ts`) can produce appears in its specimens, so a
// new state row, unit type, weapon row or reason can't slip past review.
import game from "@fixtures/game.json";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { expect, test, vi } from "vitest";
import { panelSpecimens } from "@apps/battle-lab/src/panelSpecimens";
import { REASON_MARK } from "../src/battle/present/infoPanel";
import { STATE_ROWS, type PanelRules } from "../src/battle/present/panelRows";

// The specimens are built from the stand-in types alone: a panel that reads
// right for them reads right for any type of the roster.
vi.mock("@packages/scene-assets/src/shippedUnits", async (original) => {
  const shipped = await original<typeof import("@packages/scene-assets/src/shippedUnits")>();
  const { UnitCatalog } = await import("@packages/scene-assets/src/units");
  const standIns = ["rifle", "recon", "at", "tank", "supply", "jeep"];
  return {
    ...shipped,
    UNITS: new UnitCatalog({
      ...shipped.UNITS.view,
      units: shipped.UNITS.view.units.filter((type) => standIns.includes(type.id)),
    }),
  };
});

const RULES = game as unknown as PanelRules;
const specimens = panelSpecimens(RULES);
const rows = specimens.flatMap((s) => s.panel.weapons);
const kinds = rows.flatMap((w) => w.kinds);

test("every state row the panels can show is a specimen", () => {
  const shown = new Set(specimens.flatMap((s) => s.panel.states.map((r) => r.state)));
  expect(Object.keys(STATE_ROWS).filter((k) => !shown.has(k as keyof typeof STATE_ROWS))).toEqual(
    [],
  );
});

test("every unit type is shown as own, identified enemy and last sighting", () => {
  const names = (owner: string, group?: string) =>
    new Set(
      specimens
        .filter((s) => s.owner === owner && (!group || s.group === group))
        .map((s) => s.panel.name),
    );
  for (const t of UNITS.view.units) {
    const name = t.name.toUpperCase();
    expect(names("own").has(name), `own ${t.id}`).toBe(true);
    expect(names("enemy").has(name), `enemy ${t.id}`).toBe(true);
    expect(names("contact", "last seen").has(name), `last seen ${t.id}`).toBe(true);
  }
});

test("every weapon row is heard alone, and a report with nothing named is shown", () => {
  const heard = specimens.filter((s) => s.group === "heard");
  const alone = new Set(heard.map((s) => s.label));
  for (const r of Object.keys(RULES.weapons)) expect(alone.has(r), r).toBe(true);
  expect(heard.some((s) => s.panel.weapons.length === 0)).toBe(true);
});

test("every weapon situation: unlimited, a count, empty, a cannon's loaded kind, each timer, guiding", () => {
  expect(kinds.some((k) => k.count === null)).toBe(true);
  expect(kinds.some((k) => typeof k.count === "number" && k.count > 0)).toBe(true);
  expect(kinds.some((k) => k.count === 0)).toBe(true);
  expect(rows.some((w) => w.kinds.length > 1 && w.kinds.some((k) => k.loaded))).toBe(true);
  expect(rows.some((w) => w.live?.aim != null)).toBe(true);
  expect(rows.some((w) => w.live?.reload != null)).toBe(true);
  expect(rows.some((w) => w.live?.aim != null && w.live.reload != null)).toBe(true);
  expect(rows.some((w) => w.live?.guiding)).toBe(true);
});

test("every reason a weapon can't fire shows its mark", () => {
  const shown = new Set(rows.flatMap((w) => (w.live ? [w.live.reason] : [])));
  const marked = Object.keys(REASON_MARK).filter((r) => REASON_MARK[r]);
  expect(marked.filter((r) => !shown.has(r))).toEqual([]);
});

test("selected and unselected, default and far zoom, each specimen once", () => {
  expect(specimens.some((s) => s.selected)).toBe(true);
  expect(specimens.some((s) => !s.selected)).toBe(true);
  expect(specimens.some((s) => s.zoom === "far")).toBe(true);
  expect(new Set(specimens.map((s) => s.id)).size).toBe(specimens.length);
});
