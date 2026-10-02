import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import typegpu from "unplugin-typegpu/vite";
import react from "@vitejs/plugin-react";
import { mechanicsPlugin } from "../apps/mechanics-editor/server";

// packages/* and apps/* are source-only directories outside this vite root.
// Their bare imports resolve to web/node_modules through explicit aliases, so
// dependencies stay owned by web/package.json.
const nodeModule = (path: string) =>
  fileURLToPath(new URL(`./node_modules/${path}`, import.meta.url));

const isolationHeaders = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
};

/**
 * The workbench's hot reload: when an art source (`assets/source/**`) or the
 * authored catalog changes, re-bake with the asset CLI and tell the page,
 * which reloads the runtime catalog through `AppearanceLibrary`.
 */
function assetWatch(): Plugin {
  const assets = fileURLToPath(new URL("../assets/", import.meta.url));
  const sources = `${assets}source/`;
  const catalog = `${assets}catalog.json`;
  return {
    name: "asset-watch",
    apply: "serve",
    configureServer(server) {
      server.watcher.add([sources, catalog]);
      let timer: ReturnType<typeof setTimeout> | undefined;
      let running: ReturnType<typeof spawn> | null = null;
      let dirty = false;
      const bake = () => {
        if (running || !dirty) return;
        dirty = false;
        const run = (running = spawn(process.execPath, ["asset.mjs", "bake"], {
          cwd: fileURLToPath(new URL(".", import.meta.url)),
        }));
        let output = "";
        run.stdout!.on("data", (d) => (output += d));
        run.stderr!.on("data", (d) => (output += d));
        run.on("error", (error) => (output += error.message));
        run.on("close", (code) => {
          running = null;
          if (dirty) bake();
          else
            server.ws.send({
              type: "custom",
              event: "assets:rebaked",
              data: { ok: code === 0, output },
            });
        });
      };
      const rebake = (file: string) => {
        if (!file.startsWith(sources) && file !== catalog) return;
        dirty = true;
        clearTimeout(timer);
        timer = setTimeout(bake, 150);
      };
      const events = ["add", "change", "unlink"] as const;
      for (const event of events) server.watcher.on(event, rebake);
      server.httpServer?.once("close", () => {
        for (const event of events) server.watcher.off(event, rebake);
        clearTimeout(timer);
        dirty = false;
        running?.kill();
      });
    },
  };
}

export default defineConfig({
  // Worktrees share dependencies; compiled caches stay local.
  cacheDir: fileURLToPath(new URL("../throwaway/vite-cache/", import.meta.url)),
  plugins: [
    react(),
    typegpu(),
    assetWatch(),
    mechanicsPlugin(fileURLToPath(new URL("..", import.meta.url))),
  ],
  // Appearance bundles and their runtime catalog, served same-origin at the
  // site root and copied into production builds (packages/scene-assets).
  publicDir: fileURLToPath(new URL("../assets/runtime/", import.meta.url)),
  resolve: {
    alias: [
      { find: /^typegpu$/, replacement: nodeModule("typegpu/index.js") },
      { find: /^math$/, replacement: nodeModule("math/dist/index.js") },
      { find: /^math\/shapes$/, replacement: nodeModule("math/dist/shapes/index.js") },
      { find: /^math\/random$/, replacement: nodeModule("math/dist/random/index.js") },
      { find: /^math\/color$/, replacement: nodeModule("math/dist/color/index.js") },
      { find: /^math\/time$/, replacement: nodeModule("math/dist/time/index.js") },
      { find: /^react$/, replacement: nodeModule("react/index.js") },
      { find: /^react\/jsx-runtime$/, replacement: nodeModule("react/jsx-runtime.js") },
      { find: /^react\/jsx-dev-runtime$/, replacement: nodeModule("react/jsx-dev-runtime.js") },
      { find: /^react-dom\/client$/, replacement: nodeModule("react-dom/client.js") },
      {
        find: /^@packages\//,
        replacement: fileURLToPath(new URL("../packages/", import.meta.url)),
      },
      { find: /^@apps\//, replacement: fileURLToPath(new URL("../apps/", import.meta.url)) },
      { find: /^@web\//, replacement: fileURLToPath(new URL("./src/", import.meta.url)) },
      { find: /^@wasm\//, replacement: fileURLToPath(new URL("./src/wasm/", import.meta.url)) },
      {
        find: /^@fixtures\//,
        replacement: fileURLToPath(new URL("../fixtures/", import.meta.url)),
      },
    ],
  },
  // Scan every lab route up front. Routes load lazily, so without this Vite
  // finds a route's dependencies only when a scene first opens it,
  // re-optimizes mid-run, and answers in-flight module requests with
  // "504 Outdated Optimize Dep".
  optimizeDeps: {
    entries: ["index.html", "../apps/battle-lab/src/**/*.tsx"],
  },
  server: {
    headers: isolationHeaders,
    // Scenes import ../packages and ../apps sources; git worktrees need the
    // explicit workspace root.
    fs: { allow: [fileURLToPath(new URL("..", import.meta.url))] },
  },
  preview: { headers: isolationHeaders },
});
