// @vitest-environment node
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { build } from "vite";
import { expect, test } from "vitest";
import { simulationBuild } from "../../simulationBuild";

const root = join(import.meta.dirname, "../../..");
test("the site build emits its checkout commit and exact served simulation bytes", async () => {
  await mkdir(join(root, "throwaway/multiplayer/slice09"), { recursive: true });
  const scratch = await mkdtemp(join(root, "throwaway/multiplayer/slice09/site-"));
  try {
    await writeFile(join(scratch, "entry.js"), "export const page = true;");
    const commit = execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).trim();
    const dist = join(scratch, "dist");
    await build({
      configFile: false,
      root: scratch,
      publicDir: false,
      logLevel: "silent",
      plugins: [simulationBuild(root, commit)],
      build: { outDir: dist, rollupOptions: { input: join(scratch, "entry.js") } },
    });
    const identity = JSON.parse(await readFile(join(dist, "build.json"), "utf8"));
    const wasm = await readFile(join(dist, "sim/game_wasm_bg.wasm"));
    expect(identity).toEqual({ commit, sim: createHash("sha256").update(wasm).digest("hex") });
    for (const name of ["game_wasm.js", "game_wasm_bg.wasm"]) {
      expect(
        (await readFile(join(dist, "sim", name))).equals(
          await readFile(join(root, "web/src/wasm", name)),
        ),
      ).toBe(true);
    }
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
});
