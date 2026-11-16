import { copyFile, mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

import { buildInputs, sourceBuildIdentity } from "../../packages/multiplayer/buildIdentity.ts";

/** Snapshot runtime data beside the bundle; startup never reads a live checkout. */
export async function bundleServer(
  root,
  outdir,
  commit,
  lobbyEntry = "apps/game-server/src/main.ts",
) {
  await mkdir(outdir, { recursive: true });
  const result = await Bun.build({
    entrypoints: [
      join(root, lobbyEntry),
      join(root, "apps/game-server/src/battle.ts"),
      join(root, "apps/game-server/src/battleHost/worker.ts"),
    ],
    outdir,
    target: "node",
    format: "esm",
    naming: "[name].mjs",
  });
  if (!result.success) throw new Error(result.logs.map(String).join("\n"));
  const inputs = buildInputs(root);
  for (const path of [inputs.sim, ...inputs.data]) {
    await mkdir(dirname(join(outdir, path)), { recursive: true });
    await copyFile(join(root, path), join(outdir, path));
  }
  const build = { commit, ...sourceBuildIdentity(root) };
  await writeFile(join(outdir, "build.json"), JSON.stringify(build) + "\n");
  return build;
}

// Release preparation runs the bundler from the site's archived source.
if (import.meta.main) {
  const [root, outdir, commit, entry] = process.argv.slice(2);
  await bundleServer(root, outdir, commit, entry);
}
