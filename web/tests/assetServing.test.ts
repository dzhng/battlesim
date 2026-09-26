// @vitest-environment node
// A production build carries every runtime appearance file, served
// same-origin under the page's cross-origin isolation headers.
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { build, preview, type PreviewServer } from "vite";
import { afterAll, beforeAll, expect, test } from "vitest";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema.ts";
import { bundlePath } from "@packages/scene-assets/src/schema.ts";

const WEB = new URL("../", import.meta.url).pathname;
const RUNTIME = new URL("../../assets/runtime/", import.meta.url).pathname;
const work = mkdtempSync(join(tmpdir(), "asset-serving-"));
let server: PreviewServer;
let origin: string;

beforeAll(async () => {
  // A one-line entry keeps the build to the public directory's copy, which is what is under test.
  const entry = join(WEB, "asset-serving.html");
  writeFileSync(entry, "<!doctype html><title>assets</title>\n");
  try {
    await build({
      configFile: join(WEB, "vite.config.ts"),
      root: WEB,
      logLevel: "silent",
      build: { outDir: join(work, "dist"), emptyOutDir: true, rollupOptions: { input: entry } },
    });
  } finally {
    rmSync(entry);
  }
  server = await preview({
    configFile: join(WEB, "vite.config.ts"),
    root: WEB,
    logLevel: "silent",
    build: { outDir: join(work, "dist") },
    preview: { port: 0, host: "127.0.0.1" },
  });
  origin = server.resolvedUrls!.local[0].replace(/\/$/, "");
}, 120_000);

afterAll(async () => {
  await server?.close();
  rmSync(work, { recursive: true, force: true });
});

test("every runtime catalog file is in the build and served byte for byte, same-origin and isolated", async () => {
  const catalog = JSON.parse(readFileSync(join(RUNTIME, "catalog.json"), "utf8")) as RuntimeCatalog;
  const files = [
    "catalog.json",
    ...Object.values(catalog.skeletons).map(bundlePath),
    ...Object.values(catalog.appearances).map((a) => bundlePath(a.bundle)),
  ];
  for (const file of files) {
    const response = await fetch(`${origin}/${file}`);
    expect(response.status, file).toBe(200);
    expect(response.headers.get("cross-origin-embedder-policy")).toBe("require-corp");
    expect(
      Buffer.from(await response.arrayBuffer()).equals(readFileSync(join(RUNTIME, file))),
      file,
    ).toBe(true);
  }
});
