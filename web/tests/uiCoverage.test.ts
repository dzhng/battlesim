// @vitest-environment node
// Every piece of interface the player sees has an approved picture: each
// component the battle and menus draw is drawn by a page whose scene matches
// it against its baselines (`web/scenes/_baseline.mjs`), or is named below
// with why it cannot be. A new component fails here until it is in one.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { expect, test } from "vitest";

const ROOT = join(import.meta.dirname, "../..");

/** The pages whose scenes pin pictures of what they draw. */
const PINNED_PAGES = [
  "apps/battle-lab/src/routes/panels.tsx", // scenes/panels.mjs
  "apps/battle-lab/src/routes/ui.tsx", // scenes/ui.mjs
  "apps/battle-lab/src/routes/cursor.tsx", // scenes/cursor.mjs
  "apps/battle-lab/src/MainMenu.tsx", // scenes/ui.mjs, its every page
];

/** What the player sees: the battle's presentation and the app's screens. */
const PLAYER_FACING = [
  ...readdirSync(join(ROOT, "web/src/battle/present"))
    .filter((f) => f.endsWith(".tsx"))
    .map((f) => `web/src/battle/present/${f}`),
  "apps/battle-lab/src/PauseMenu.tsx",
  "apps/battle-lab/src/LoadingScreen.tsx",
  "apps/battle-lab/src/battleStatus.tsx",
  "apps/battle-lab/src/SoundControls.tsx",
];

/** Drawn by the game but not pinned by a picture, each with why. */
const UNPINNED: Record<string, string> = {
  ReadoutLayer:
    "callouts placed over the live 3D battlefield; their panels are pinned by the panel workbench, their placement checked by scenes/readouts.mjs",
  SelectionCard: "drawn only by the readouts lab, not by the battle or menus",
};

const ALIASES: [string, string][] = [
  ["@web/", "web/src/"],
  ["@apps/", "apps/"],
  ["@packages/", "packages/"],
];

/** The source file an import names, or null for a package or data. */
function resolve(from: string, spec: string): string | null {
  const alias = ALIASES.find(([a]) => spec.startsWith(a));
  const base = alias
    ? join(ROOT, alias[1], spec.slice(alias[0].length))
    : spec.startsWith(".")
      ? join(dirname(from), spec)
      : null;
  if (!base) return null;
  for (const path of [base, `${base}.tsx`, `${base}.ts`, join(base, "index.ts")])
    if (/\.tsx?$/.test(path) && existsSync(path)) return path;
  return null;
}

/** Every source file the pinned pages draw through, their imports' imports included. */
function drawnThrough(): Map<string, string> {
  const seen = new Map<string, string>();
  const queue = PINNED_PAGES.map((p) => join(ROOT, p));
  while (queue.length) {
    const file = queue.pop()!;
    if (seen.has(file)) continue;
    const text = readFileSync(file, "utf8");
    seen.set(file, text);
    for (const [, spec] of text.matchAll(/(?:from|import)\s*\(?\s*["']([^"']+)["']/g)) {
      const next = resolve(file, spec);
      if (next) queue.push(next);
    }
  }
  return seen;
}

test("every component the player sees is drawn by a page with approved pictures", () => {
  const sources = drawnThrough();
  const pages = new Set(PINNED_PAGES.map((p) => join(ROOT, p)));
  const unpinned: string[] = [];
  const named = new Set<string>();
  for (const path of PLAYER_FACING) {
    const text = readFileSync(join(ROOT, path), "utf8");
    for (const [, name] of text.matchAll(/^export function ([A-Z]\w*)/gm)) {
      named.add(name);
      if (pages.has(join(ROOT, path))) continue;
      const drawn = [...sources].some(
        ([file, source]) => file !== join(ROOT, path) && new RegExp(`<${name}[\\s/>]`).test(source),
      );
      // A component drawn in its own file (ArmyDeck draws its cards) counts
      // when that file is itself drawn through a pinned page.
      const drawnAtHome = sources.has(join(ROOT, path)) && new RegExp(`<${name}[\\s/>]`).test(text);
      if (!drawn && !drawnAtHome && !(name in UNPINNED))
        unpinned.push(`${name} (${relative(ROOT, join(ROOT, path))})`);
    }
  }
  expect(unpinned).toEqual([]);
  // An exemption names a component that still exists.
  expect(Object.keys(UNPINNED).filter((name) => !named.has(name))).toEqual([]);
});
