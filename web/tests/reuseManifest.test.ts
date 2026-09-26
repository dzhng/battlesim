// @vitest-environment node
// The reuse manifest is the only door from ~/dev/game into this repo:
// every copied file is listed with its provenance, nothing imports the
// sibling at runtime, and third-party inputs match their recorded hashes.
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { expect, test } from "vitest";

const ROOT = new URL("../../", import.meta.url).pathname;
const MANIFEST = join(ROOT, "specs/battle-look/assets/reuse-manifest.json");

interface Manifest {
  runtime_imports_of_sibling: string;
  files: { source: string; source_commit: string; destination: string; mode: string }[];
  third_party: { path: string; sha256: string; licence: string; accepted_by: string }[];
}
const manifest = JSON.parse(readFileSync(MANIFEST, "utf8")) as Manifest;

function sources(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "wasm" || name.startsWith(".")) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...sources(path));
    else if (/\.(ts|tsx|mjs|js|rs|py)$/.test(name)) out.push(path);
  }
  return out;
}

test("every copied file exists where the manifest says, with provenance", () => {
  expect(manifest.runtime_imports_of_sibling).toBe("none");
  for (const f of manifest.files) {
    expect(existsSync(join(ROOT, f.destination)), f.destination).toBe(true);
    expect(f.source_commit, f.source).toMatch(/^[0-9a-f]{7,40}$/);
    expect(["copy", "adapted", "technique"]).toContain(f.mode);
  }
});

test("nothing imports the sibling repo at runtime", () => {
  const offenders = ["packages", "apps", "web/src", "web/scenes", "crates"]
    .flatMap((d) => sources(join(ROOT, d)))
    .filter((path) =>
      /from\s+["'][^"']*\/dev\/game\/|["']\.\.\/(\.\.\/)+game\//.test(readFileSync(path, "utf8")),
    )
    .map((path) => relative(ROOT, path));
  expect(offenders).toEqual([]);
});

test("third-party inputs match their recorded hash and carry an accepted licence", () => {
  for (const t of manifest.third_party) {
    const bytes = readFileSync(join(ROOT, t.path));
    expect(createHash("sha256").update(bytes).digest("hex"), t.path).toBe(t.sha256);
    expect(["CC0-1.0", "MIT", "project-owned"]).toContain(t.licence);
    expect(t.accepted_by, t.path).toBeTruthy();
  }
});
