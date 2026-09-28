// @vitest-environment node
// The generated icons: every weapon row's icon, every role's symbol and every
// unit type's silhouette, rendered from its own baked model, exists under
// assets/icons/ exactly as the generator draws it. `bun run --cwd web asset
// -- icons` rewrites them.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { expect, test } from "vitest";
import village from "@fixtures/village.json";
import { iconFiles, unitIcons } from "@packages/scene-assets/src/icons";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import {
  runtimeLookup,
  silhouetteSvg,
  unitSolids,
  type Solid,
} from "@packages/scene-assets/src/silhouette";

const ICONS = new URL("../../assets/icons/", import.meta.url);
const RUNTIME = new URL("../../assets/runtime/", import.meta.url);

const lookup = runtimeLookup(
  JSON.parse(readFileSync(new URL("catalog.json", RUNTIME), "utf8")) as RuntimeCatalog,
  (path) => new Uint8Array(readFileSync(new URL(path, RUNTIME))),
);
const shipped = (id: string) => unitSolids(UNITS, id, lookup);

test("every weapon, role and unit type has its generated icon, current, and nothing else", () => {
  const files = iconFiles(village.weapons, UNITS, shipped);
  for (const w of Object.values(village.weapons))
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
