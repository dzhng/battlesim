// @vitest-environment node
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { expect, test, vi } from "vitest";
import { buildInputs } from "../../../packages/multiplayer/buildIdentity.ts";
import { simulationBuild } from "../../simulationBuild";
import { buildIdentity } from "../../../apps/battle-lab/src/buildIdentity";

const root = join(import.meta.dirname, "../../..");
test("the dev page, site publication and server bundle state the same identity and cover every bundled document", async () => {
  const folder = await mkdtemp(join(root, "throwaway/multiplayer/shared-build-"));
  try {
    const commit = execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).trim();
    const plugin = simulationBuild(root, commit);
    const emitted: { fileName: string; source: string | Uint8Array }[] = [];
    (plugin.generateBundle as (this: unknown) => void).call({
      emitFile: (file: (typeof emitted)[number]) => emitted.push(file),
    });
    const site = JSON.parse(String(emitted.find((file) => file.fileName === "build.json")!.source));
    expect(site.commit).toMatch(/^[a-f0-9]{40}$/);
    const wasm = emitted.find((file) => file.fileName === "sim/game_wasm_bg.wasm")!.source;
    const fetch = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(
        async () => new Response(typeof wasm === "string" ? wasm : new Uint8Array(wasm).buffer),
      );
    try {
      const page = await buildIdentity();
      const { commit: provenance, ...identity } = site;
      expect(provenance).toBe(commit);
      expect(page).toEqual(identity);
    } finally {
      fetch.mockRestore();
    }
    const probe = join(folder, "bundle.mjs");
    await writeFile(
      probe,
      `
      import { bundleServer } from ${JSON.stringify(join(root, "apps/game-server/bundle.mjs"))};
      const original = Bun.build;
      let inputs;
      Bun.build = async options => {
        const result = await original({ ...options, metafile: true });
        inputs = Object.keys(result.metafile.inputs);
        return result;
      };
      const build = await bundleServer(${JSON.stringify(root)}, ${JSON.stringify(join(folder, "server"))}, ${JSON.stringify(commit)});
      await Bun.write(${JSON.stringify(join(folder, "result.json"))}, JSON.stringify({ build, inputs }));
    `,
    );
    execFileSync("bun", [probe], { cwd: root, stdio: "pipe" });
    const { build, inputs } = JSON.parse(await readFile(join(folder, "result.json"), "utf8"));
    expect(build).toEqual(site);
    const covered = new Set(buildInputs(root).data);
    const embedded: string[] = [];
    for (const path of inputs as string[]) {
      const file = path.startsWith("/") ? relative(root, path) : path.replace(/^\.\//, "");
      if (file.endsWith(".json")) {
        embedded.push(file);
        expect(covered.has(file), file).toBe(true);
      }
    }
    expect(embedded.length).toBeGreaterThan(0);
    for (const path of await readdir(join(folder, "server"), { recursive: true })) {
      if (path.endsWith(".json") && path !== "build.json")
        expect(covered.has(path), path).toBe(true);
    }
    for (const file of buildInputs(root).data) {
      const bytes = await readFile(join(folder, "server", file));
      expect(bytes.equals(await readFile(join(root, file))), file).toBe(true);
    }
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});
