// @vitest-environment node
// The generated icons: every weapon row's icon, every role's symbol and every
// unit type's silhouette exists under assets/icons/ exactly as the generator
// draws it. `bun run --cwd web asset -- icons` rewrites them.
import { existsSync, readFileSync } from "node:fs";
import { expect, test } from "vitest";
import village from "@fixtures/village.json";
import { iconFiles, unitIcons } from "@packages/scene-assets/src/icons";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";

const ICONS = new URL("../../assets/icons/", import.meta.url);

test("every weapon, role and unit type has its generated icon, current", () => {
  const files = iconFiles(village.weapons, UNITS);
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
});

test("an icon the generator cannot draw is refused, naming it", () => {
  expect(() => iconFiles({ bayonet: { icon: "bayonet" } }, UNITS)).toThrow(/bayonet/);
});
