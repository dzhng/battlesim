import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import type { Plugin } from "vite";

/** Publish the very files @wasm resolves, including the glue's panic reporter. */
export function simulationBuild(root: string, commit: string): Plugin {
  return {
    name: "simulation-build",
    generateBundle() {
      const wasm = join(root, "web/src/wasm");
      const sim = createHash("sha256")
        .update(readFileSync(join(wasm, "game_wasm_bg.wasm")))
        .digest("hex");
      this.emitFile({
        type: "asset",
        fileName: "build.json",
        source: JSON.stringify({ commit, sim }) + "\n",
      });
      const files = [
        "game_wasm.js",
        "game_wasm_bg.wasm",
        ...readdirSync(join(wasm, "snippets"), { recursive: true, withFileTypes: true })
          .filter((entry) => entry.isFile())
          .map((entry) => relative(wasm, join(entry.parentPath, entry.name))),
      ];
      for (const file of files)
        this.emitFile({
          type: "asset",
          fileName: `sim/${file}`,
          source: readFileSync(join(wasm, file)),
        });
    },
  };
}
