import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

/** Snapshot runtime data beside the bundle; startup never reads a live checkout. */
export async function bundleServer(root, outdir, commit) {
  await mkdir(outdir, { recursive: true });
  const result = await Bun.build({
    entrypoints: [
      join(root, "apps/game-server/src/main.ts"),
      join(root, "apps/game-server/src/battleHost/worker.ts"),
    ],
    outdir,
    target: "node",
    format: "esm",
    naming: "[name].mjs",
  });
  if (!result.success) throw new Error(result.logs.map(String).join("\n"));
  const wasm = "web/src/wasm/game_wasm_bg.wasm";
  for (const path of [
    wasm,
    "fixtures/catalog.json",
    "fixtures/rules.json",
    "fixtures/map-presets.json",
    "fixtures/prototype-building-templates.json",
  ]) {
    await mkdir(dirname(join(outdir, path)), { recursive: true });
    await copyFile(join(root, path), join(outdir, path));
  }
  const sim = createHash("sha256")
    .update(await readFile(join(outdir, wasm)))
    .digest("hex");
  await writeFile(join(outdir, "build.json"), JSON.stringify({ commit, sim }) + "\n");
  return { commit, sim };
}
