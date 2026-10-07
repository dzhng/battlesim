// @vitest-environment node
// The generated icons: every weapon row's icon, every role's symbol and every
// unit type's silhouette, rendered from its own baked model, exists under
// assets/icons/ exactly as the generator draws it. `bun run --cwd web asset
// -- icons` rewrites them.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { expect, test } from "vitest";
import { iconFiles, unitIcons } from "@packages/scene-assets/src/icons";
import { inkBounds } from "@packages/scene-assets/src/inkBounds";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { UNITS, WEAPONS } from "@packages/scene-assets/src/shippedUnits";
import {
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
const shipped = (id: string) => unitSolids(UNITS, id, lookup);

test("every weapon, role and unit type has its generated icon, current, and nothing else", () => {
  const files = iconFiles(WEAPONS, UNITS, shipped);
  for (const w of Object.values(WEAPONS))
    expect(files.has(`weapons/${w.icon}.svg`), `weapon icon ${w.icon}`).toBe(true);
  for (const t of UNITS.view.units) {
    const { silhouette, role } = unitIcons(t);
    expect(files.has(silhouette) && files.has(role), t.id).toBe(true);
  }
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
  expect(() => iconFiles({}, UNITS, () => null)).toThrow(
    /unit type \w+: its model is not installed/,
  );
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
