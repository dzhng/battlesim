// @vitest-environment node
// The generated icons: every weapon row's icon, every role's symbol, every
// unit type's silhouette, rendered from its own baked model, and every
// disabled card's, rendered from its source model (named by
// fixtures/units/model-manifest.json), exists under assets/icons/ exactly as
// the generator draws it. `bun run --cwd web asset -- icons` rewrites them.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { expect, test } from "vitest";
import { cardIcon, iconFiles, unitIcons } from "@packages/scene-assets/src/icons";
import { inkBounds } from "@packages/scene-assets/src/inkBounds";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import type { UnitCard, UnitCatalog, UnitType } from "@packages/scene-assets/src/units";
import { UNITS, WEAPONS } from "./catalog";
import {
  disabledLookup,
  runtimeLookup,
  silhouetteSvg,
  unitSolids,
  type Solid,
} from "@packages/scene-assets/src/silhouette";

const ICONS = new URL("../../assets/icons/", import.meta.url);
const RUNTIME = new URL("../../assets/runtime/", import.meta.url);

const lookup = await runtimeLookup(
  JSON.parse(readFileSync(new URL("catalog.json", RUNTIME), "utf8")) as RuntimeCatalog,
  async (path) => new Uint8Array(readFileSync(new URL(path, RUNTIME))),
);
const REPO = new URL("../../", import.meta.url);
const SKELETONS = JSON.parse(readFileSync(new URL("assets/catalog.json", REPO), "utf8")).skeletons;
const disabled = await disabledLookup(
  JSON.parse(readFileSync(new URL("fixtures/units/model-manifest.json", REPO), "utf8")),
  async (path) => new Uint8Array(readFileSync(new URL(path, REPO))),
  SKELETONS,
);
const shipped = (id: string) => (UNITS.has(id) ? unitSolids(UNITS, id, lookup) : disabled(id));
const APS = { capacity: 4, cooldown_s: 3, standoff_m: 8, service_s: 10, stock_per_charge: 20 };
const disabledCards = UNITS.cards.filter((c) => c.disabled_reason !== null && !UNITS.has(c.id));

test("every weapon, role and unit type has its generated icon, current, and nothing else", () => {
  const files = iconFiles(WEAPONS, UNITS, shipped);
  // Every weapon a card can show: a row's, an active protection's and a
  // disabled card's planned weapon's.
  const shown = [
    ...Object.values(WEAPONS).map((w) => w.icon),
    ...UNITS.view.units.flatMap((t) => t.capabilities.active_protection?.icon ?? []),
    ...UNITS.cards.flatMap((c) => c.planned_weapons.map((w) => w.icon)),
  ];
  for (const icon of shown)
    expect(files.has(`weapons/${icon}.svg`), `weapon icon ${icon}`).toBe(true);
  for (const t of UNITS.view.units) {
    const { silhouette, role } = unitIcons(t);
    expect(files.has(silhouette) && files.has(role), t.id).toBe(true);
  }
  expect(disabledCards.length).toBeGreaterThan(0);
  for (const c of disabledCards) expect(files.has(cardIcon(c.id)), c.id).toBe(true);
  const stale = [...files].filter(([path, svg]) => {
    const file = new URL(path, ICONS);
    return !existsSync(file) || readFileSync(file, "utf8") !== svg;
  });
  expect(
    stale.map(([path]) => path),
    "missing or stale: run `bun run --cwd web asset -- icons`",
  ).toEqual([]);
  const onDisk = (readdirSync(ICONS, { recursive: true }) as string[]).filter((p) =>
    p.endsWith(".svg"),
  );
  expect(onDisk.filter((p) => !files.has(p))).toEqual([]);
});

test("an icon the generator cannot draw is refused, naming it", () => {
  expect(() => iconFiles({ bayonet: { icon: "bayonet" } }, UNITS, shipped)).toThrow(/bayonet/);
  // An active protection's icon and a disabled card's planned weapon's are
  // drawn by the same generator, and refused the same way.
  const tank = UNITS.type("test_tank");
  const protectedTank = {
    ...tank,
    capabilities: { active_protection: { ...APS, name: "Shield", icon: "shield" } },
  };
  const card = { ...UNITS.cards[0], planned_weapons: [{ name: "Laser", icon: "laser" }] };
  const only = (units: UnitType[], cards: UnitCard[]) =>
    ({ view: { ...UNITS.view, units }, cards, has: () => true }) as unknown as UnitCatalog;
  expect(() => iconFiles({}, only([protectedTank], []), shipped)).toThrow(/"shield"/);
  expect(() => iconFiles({}, only([], [card]), shipped)).toThrow(/"laser"/);
  expect(() => iconFiles({}, UNITS, () => null)).toThrow(
    /unit type \w+: its model is not installed/,
  );
  // A disabled card has no unit type; its icon comes from its source model.
  expect(() => iconFiles({}, UNITS, (id) => (UNITS.has(id) ? shipped(id) : null))).toThrow(
    /disabled card \w+: its source model is not installed/,
  );
});

test("a disabled soldier card's silhouette is its soldier posed aiming, not its bind pose", () => {
  // The Javelin team's source is a skinned soldier on the launcher clips
  // (its manifest entry names the skeleton): posed at the clips' aim
  // reference he stands a man's height, the launcher out ahead of him.
  const solids = disabled("fgm_148_javelin_team");
  expect(solids?.length).toBeGreaterThan(0);
  let [x0, x1, y0, y1, z0, z1] = [Infinity, -Infinity, Infinity, -Infinity, Infinity, -Infinity];
  for (const s of solids!)
    for (let v = 0; v < s.positions.length; v += 3) {
      [x0, x1] = [Math.min(x0, s.positions[v]), Math.max(x1, s.positions[v])];
      [y0, y1] = [Math.min(y0, s.positions[v + 1]), Math.max(y1, s.positions[v + 1])];
      [z0, z1] = [Math.min(z0, s.positions[v + 2]), Math.max(z1, s.positions[v + 2])];
    }
  expect(z1 - z0).toBeGreaterThan(1.5);
  expect(z1 - z0).toBeLessThan(2.1);
  // A bind (T) pose spreads the arms across the view's depth; aiming does not.
  expect(y1 - y0).toBeLessThan(1.2);
  expect(x1 - x0).toBeGreaterThan(1.0);
});

test("a silhouette is the model's side view: its outline, front to the right", () => {
  // An L of two boxes: a long low hull and a tall block at its back (−X).
  const box = (x0: number, x1: number, z0: number, z1: number): Solid => ({
    positions: Float32Array.from(
      [
        [x0, 0, z0],
        [x1, 0, z0],
        [x1, 0, z1],
        [x0, 0, z1],
      ].flat(),
    ),
    indices: [0, 1, 2, 0, 2, 3],
  });
  const svg = silhouetteSvg([box(0, 4, 0, 1), box(0, 1, 1, 3)]);
  const [, w, h] = svg.match(/viewBox="0 0 (\d+) (\d+)"/)!.map(Number);
  expect(w).toBeGreaterThan(h);
  // One closed outline; SVG y runs down, so the tall block's top is the
  // smallest y, at the left (the back).
  const d = svg.match(/ d="([^"]+)"/)![1];
  expect(d.match(/M/g)).toHaveLength(1);
  const points = [...d.matchAll(/(-?[\d.]+) (-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);
  const top = Math.min(...points.map((p) => p[1]));
  const topXs = points.filter((p) => p[1] - top < 1).map((p) => p[0]);
  expect(Math.max(...topXs)).toBeLessThan(w / 3);
  // The same input gives the same bytes.
  expect(silhouetteSvg([box(0, 4, 0, 1), box(0, 1, 1, 3)])).toBe(svg);
});

test("an icon drawing's bounds follow its curves, not just its end points", () => {
  // A half circle bulging right of its chord, and a cubic bowing up.
  const half = inkBounds('<path d="M12 4a8 8 0 0 1 0 16"/>');
  expect(half.map((v) => Number(v.toFixed(2)))).toEqual([12, 4, 20, 20]);
  expect(inkBounds('<path d="M0 10C0 2 10 2 10 10"/>')[1]).toBeCloseTo(4, 2);
  expect(
    inkBounds('<rect x="3" y="8" width="18" height="12"/><circle cx="2" cy="2" r="1"/>'),
  ).toEqual([1, 1, 21, 20]);
});

test("every weapon, state and glyph icon is centred on its ink", () => {
  const files = iconFiles(WEAPONS, UNITS, shipped);
  const offCentre = [...files]
    .filter(([path]) => /^(weapons|states|glyphs)\//.test(path))
    .filter(([, svg]) => {
      const [vx, vy, w, h] = svg
        .match(/viewBox="([^"]+)"/)![1]
        .split(" ")
        .map(Number);
      const [x0, y0, x1, y1] = inkBounds(svg);
      return Math.hypot((x0 + x1) / 2 - (vx + w / 2), (y0 + y1) / 2 - (vy + h / 2)) > 0.01;
    })
    .map(([path]) => path);
  expect(offCentre).toEqual([]);
});
