/** Browser tests own independent Vite caches; simultaneous servers must not
 * replace each other's optimized dependency files while pages import them. */
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

export async function browserTestServer() {
  const cacheDir = await mkdtemp(join(tmpdir(), "battle-browser-test-"));
  const server = await createServer({
    configFile: fileURLToPath(new URL("../../vite.config.ts", import.meta.url)),
    configLoader: "runner",
    cacheDir,
    server: { port: 0, host: "127.0.0.1" },
    plugins: [
      {
        name: "browser-test-setup",
        configureServer(server) {
          // Register before Vite's SPA fallback, which would otherwise serve
          // index.html here and boot the menu (including GPU warm-up).
          // Browser imports still need React's preamble and Vite's client.
          server.middlewares.use("/__test_setup__", (_request, response, next) => {
            void server
              .transformIndexHtml(
                "/__test_setup__",
                "<!doctype html><title>Browser test setup</title>",
              )
              .then((html) => {
                response.setHeader("Content-Type", "text/html");
                response.end(html);
              }, next);
          });
        },
      },
    ],
  }).catch(async (error) => {
    await rm(cacheDir, { recursive: true, force: true });
    throw error;
  });
  const close = async () => {
    try {
      await server.close();
    } finally {
      await rm(cacheDir, { recursive: true, force: true });
    }
  };
  try {
    await server.listen();
    return { url: server.resolvedUrls!.local[0], close };
  } catch (error) {
    await close();
    throw error;
  }
}
