// @vitest-environment node
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { build } from "vite";
import { expect, test, vi } from "vitest";
import { sharedBuildIdentity } from "../../../packages/multiplayer/buildIdentity.ts";
import { buildIdentity as devIdentity } from "../../../apps/battle-lab/src/buildIdentity";
import { simulationBuild } from "../../simulationBuild";

const root = join(import.meta.dirname, "../../..");
test("the site build emits its checkout commit and exact served simulation bytes", async () => {
  await mkdir(join(root, "throwaway/multiplayer/slice09"), { recursive: true });
  const scratch = await mkdtemp(join(root, "throwaway/multiplayer/slice09/site-"));
  try {
    await writeFile(
      join(scratch, "entry.js"),
      `export { buildIdentity } from ${JSON.stringify(join(root, "apps/battle-lab/src/buildIdentity.ts"))};`,
    );
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
      resolve: { alias: { "@wasm": join(root, "web/src/wasm") } },
      plugins: [simulationBuild(root, commit)],
      build: {
        outDir: dist,
        rollupOptions: {
          input: join(scratch, "entry.js"),
          preserveEntrySignatures: "strict",
          output: { entryFileNames: "page.mjs" },
        },
      },
    });
    const identity = JSON.parse(await readFile(join(dist, "build.json"), "utf8"));
    const wasm = await readFile(join(dist, "sim/game_wasm_bg.wasm"));
    expect(identity).toEqual({
      commit,
      sim: createHash("sha256").update(wasm).digest("hex"),
      ...sharedBuildIdentity(root),
    });
    const productionPage = await import(pathToFileURL(join(dist, "page.mjs")).href);
    vi.spyOn(globalThis, "fetch").mockImplementation(
      async () => new Response(new Uint8Array(wasm).buffer),
    );
    const pageBuild = await productionPage.buildIdentity();
    expect(pageBuild).toEqual(await devIdentity());
    expect({ commit, ...pageBuild }).toEqual(identity);
    for (const name of ["game_wasm.js", "game_wasm_bg.wasm"]) {
      expect(
        (await readFile(join(dist, "sim", name))).equals(
          await readFile(join(root, "web/src/wasm", name)),
        ),
      ).toBe(true);
    }
  } finally {
    vi.restoreAllMocks();
    await rm(scratch, { recursive: true, force: true });
  }
});
