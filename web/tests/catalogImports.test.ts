// @vitest-environment node
// The battle session owns its unit catalog: only the session factory
// imports the committed game catalog, so no module can read one page-wide
// catalog behind a session's back.
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { expect, test } from "vitest";

const ROOT = join(import.meta.dirname, "../..");
const FACTORY = "web/src/battle/catalog/sets.ts";
/** Shipped and tool sources; tests and generated output are not the app. */
const SOURCES = ["apps", "packages", "web/src", "web/scenes", "web/asset.mjs", "web/scene.mjs"];
const SKIP = new Set(["node_modules", "wasm", "dist"]);

function sources(path: string): string[] {
  const full = join(ROOT, path);
  if (/\.(ts|tsx|mjs|js)$/.test(path)) return [full];
  return readdirSync(full, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory()
      ? SKIP.has(entry.name)
        ? []
        : sources(join(path, entry.name))
      : /\.(ts|tsx|mjs|js)$/.test(entry.name)
        ? [join(full, entry.name)]
        : [],
  );
}

/** A static or dynamic import whose specifier names `fixtures/catalog.json`. */
const IMPORT = /(?:from\s*|import\s*\(\s*|import\s+)["'][^"']*fixtures\/catalog\.json["']/;

test("only the session factory imports the game catalog", () => {
  const importers = SOURCES.flatMap(sources)
    .filter((file) => IMPORT.test(readFileSync(file, "utf8")))
    .map((file) => relative(ROOT, file));
  expect(importers).toEqual([FACTORY]);
});
