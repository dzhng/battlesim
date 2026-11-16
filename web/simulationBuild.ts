import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import {
  buildInputs,
  sharedBuildIdentity,
  sourceBuildIdentity,
  WIRE_SOURCE_DIRECTORY,
} from "../packages/multiplayer/buildIdentity.ts";
import type { Plugin } from "vite";

/** Publish the very files @wasm resolves, including the glue's panic reporter. */
export function simulationBuild(root: string, commit: string): Plugin {
  return {
    name: "simulation-build",
    config: () => ({ define: { __SHARED_BUILD__: JSON.stringify(sharedBuildIdentity(root)) } }),
    configureServer(server) {
      const documents = new Set(buildInputs(root).data);
      server.watcher.add([
        ...[...documents].map((path) => join(root, path)),
        join(root, WIRE_SOURCE_DIRECTORY),
      ]);
      const refresh = (file: string) => {
        const path = relative(root, file).replaceAll("\\", "/");
        if (documents.has(path) || path.startsWith(`${WIRE_SOURCE_DIRECTORY}/`))
          void server.restart();
      };
      // Defines are served by Vite's bootstrap, so HMR alone would leave stale hashes.
      for (const event of ["add", "change", "unlink"] as const) server.watcher.on(event, refresh);
    },
    generateBundle() {
      const wasm = join(root, "web/src/wasm");
      this.emitFile({
        type: "asset",
        fileName: "build.json",
        source: JSON.stringify({ commit, ...sourceBuildIdentity(root) }) + "\n",
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
