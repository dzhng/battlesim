import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import typegpu from "unplugin-typegpu/vite";
import react from "@vitejs/plugin-react";

// packages/* and apps/* are source-only directories outside this vite root.
// Their bare imports resolve to web/node_modules through explicit aliases, so
// dependencies stay owned by web/package.json.
const nodeModule = (path: string) =>
  fileURLToPath(new URL(`./node_modules/${path}`, import.meta.url));

const isolationHeaders = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
};

export default defineConfig({
  plugins: [react(), typegpu()],
  // Appearance bundles and their runtime catalog, served same-origin at the
  // site root and copied into production builds (packages/scene-assets).
  publicDir: fileURLToPath(new URL("../assets/runtime/", import.meta.url)),
  resolve: {
    alias: [
      { find: /^typegpu$/, replacement: nodeModule("typegpu/index.js") },
      { find: /^math$/, replacement: nodeModule("math/dist/index.js") },
      { find: /^math\/shapes$/, replacement: nodeModule("math/dist/shapes/index.js") },
      { find: /^math\/random$/, replacement: nodeModule("math/dist/random/index.js") },
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
