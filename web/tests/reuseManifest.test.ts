// @vitest-environment node
// The reuse manifest is the only door from ~/dev/game into this repo:
// every copied file is listed with its provenance, nothing imports the
// sibling at runtime, and third-party inputs match their recorded hashes.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { expect, test } from "vitest";
import { contentSha256 } from "@packages/scene-assets/src/glb.ts";
import { ALLOWED_LICENCES } from "@packages/scene-assets/src/schema.ts";

const ROOT = new URL("../../", import.meta.url).pathname;
const MANIFEST = join(ROOT, "reuse-manifest.json");

interface Manifest {
  runtime_imports_of_sibling: string;
  files: { source: string; source_commit: string; destination: string; mode: string }[];
  third_party: {
    path: string;
    sha256: string;
    licence: string;
    accepted_by: string;
    source_url?: string;
  }[];
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

// An LFS-tracked input checked out as a pointer is hashed by the pointer's oid,
// which is its content's sha256, so the check needs no LFS pull.
test("third-party inputs match their recorded hash and carry an accepted licence", async () => {
  for (const t of manifest.third_party) {
    expect(ALLOWED_LICENCES).toContain(t.licence);
    expect(t.accepted_by, t.path).toBeTruthy();
    // External packs are not redistributed: the Blender scripts verify the
    // hash when they read the file from the local pack cache (`packs.py`).
    if (t.path.startsWith("packs/")) {
      expect(t.source_url, t.path).toMatch(/^https:\/\//);
      expect(existsSync(join(ROOT, t.path)), t.path).toBe(false);
      continue;
    }
    const bytes = new Uint8Array(readFileSync(join(ROOT, t.path)));
    expect(await contentSha256(bytes), t.path).toBe(t.sha256);
  }
});
