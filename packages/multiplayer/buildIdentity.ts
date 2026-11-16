// Build-time only: the site and server share this owner of compatibility inputs.
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

export const WIRE_SOURCE_DIRECTORY = "packages/multiplayer/src";

/** Includes runtime documents and JSON embedded into the server's battle code. */
export function buildInputs(root: string) {
  const wire = join(root, WIRE_SOURCE_DIRECTORY);
  return {
    sim: "web/src/wasm/game_wasm_bg.wasm",
    data: [
      "fixtures/catalog.json",
      "fixtures/generated-battle.json",
      "fixtures/map-presets.json",
      "fixtures/presentation.json",
      "fixtures/prototype-building-templates.json",
      "fixtures/rules.json",
      "fixtures/skirmish-sizes.json",
    ],
    wire: readdirSync(wire, { recursive: true, withFileTypes: true })
      .filter((entry) => entry.isFile())
      .map((entry) => relative(root, join(entry.parentPath, entry.name)).replaceAll("\\", "/"))
      .sort(),
  };
}

/** Frame paths and byte lengths so file boundaries cannot alias each other. */
function hashFiles(root: string, files: readonly string[]) {
  const hash = createHash("sha256");
  for (const path of [...files].sort()) {
    const bytes = readFileSync(join(root, path));
    hash.update(JSON.stringify([path, bytes.length]) + "\n");
    hash.update(bytes);
  }
  return hash.digest("hex");
}

/** Fixed into the page by Vite, in development and production alike. */
export function sharedBuildIdentity(root: string) {
  const inputs = buildInputs(root);
  return { data: hashFiles(root, inputs.data), wire: hashFiles(root, inputs.wire) };
}

/** The page hashes the bytes it loads instead; a bundle reads its own copy. */
export function sourceBuildIdentity(root: string) {
  return {
    sim: createHash("sha256")
      .update(readFileSync(join(root, buildInputs(root).sim)))
      .digest("hex"),
    ...sharedBuildIdentity(root),
  };
}
