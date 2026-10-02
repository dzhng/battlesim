// @vitest-environment node
// Shipping assets, through the native reader and the same browser codec:
// actual map selection and the complete generated catalogue retain the
// aggregate DOWNLOAD gate, separately from decoded/resident budgets.
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { expect, test } from "vitest";
import { gzipTransport, kitDownloadBytes, unpackGzip } from "@packages/scene-assets/src/gzip.ts";
import { decodeBundle } from "@packages/scene-assets/src/codec.ts";
import { decodeTemplateLibrary, templateKits } from "@packages/scene-assets/src/templateLibrary.ts";
import {
  bundlePath,
  templateLibraryPath,
  SHARED_KIT_DOWNLOAD_MAX_BYTES,
  KIT_BUNDLE_MAX_BYTES,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema.ts";

const root = new URL("../../../", import.meta.url);
const read = (path: string) => new Uint8Array(readFileSync(new URL(path, root)));
const json = (path: string) => JSON.parse(new TextDecoder().decode(read(path)));

test("the native asset reader admits the real Market Town shared-art download within 50 MiB", () => {
  const run = spawnSync(
    process.execPath,
    ["web/asset.mjs", "download", "fixtures/maps/market-town/map.json"],
    { cwd: root, encoding: "utf8" },
  );
  expect(run.status, run.stderr).toBe(0);
  const report = JSON.parse(run.stdout);
  expect(report.ok).toBe(true);
  expect(report.bytes).toBeGreaterThan(0);
  expect(report.bytes).toBeLessThanOrEqual(SHARED_KIT_DOWNLOAD_MAX_BYTES);
});

test("the native reader refuses an oversized library before opening its payload or map", () => {
  const scratch = mkdtempSync(join(tmpdir(), "kit-download-"));
  try {
    mkdirSync(join(scratch, "web"));
    mkdirSync(join(scratch, "assets/runtime"), { recursive: true });
    copyFileSync(new URL("web/asset.mjs", root), join(scratch, "web/asset.mjs"));
    symlinkSync(fileURLToPath(new URL("packages", root)), join(scratch, "packages"));
    symlinkSync(
      fileURLToPath(new URL("web/node_modules", root)),
      join(scratch, "web/node_modules"),
    );
    const hash = "a".repeat(64);
    writeFileSync(
      join(scratch, "assets/runtime/catalog.json"),
      JSON.stringify({
        sides: {},
        skeletons: {},
        appearances: {},
        templates: { library: hash },
        gzip: {
          [hash]: { hash: "b".repeat(64), bytes: SHARED_KIT_DOWNLOAD_MAX_BYTES + 1, raw_bytes: 1 },
        },
      }),
    );
    const run = spawnSync(process.execPath, ["web/asset.mjs", "download", "absent-map.json"], {
      cwd: scratch,
      encoding: "utf8",
    });
    expect(run.status).not.toBe(0);
    expect(run.stderr).toContain("shared kit download");
    expect(run.stderr).not.toContain("ENOENT");
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test("every generated template's full art and the library fit the shared download gate with exact raw integrity", async () => {
  const catalog = json("assets/runtime/catalog.json") as RuntimeCatalog;
  const libraryHash = catalog.templates!.library;
  const wire = gzipTransport(catalog, libraryHash);
  const library = decodeTemplateLibrary(
    await unpackGzip(read(`assets/runtime/${templateLibraryPath(wire.hash)}`), wire, libraryHash),
  );
  const ids = json("fixtures/prototype-building-templates.json").map(
    (template: { id: string }) => template.id,
  );
  for (const name of templateKits(library, ids)) {
    const hash = catalog.appearances[name].bundle;
    const gzip = gzipTransport(catalog, hash, KIT_BUNDLE_MAX_BYTES);
    const bundle = decodeBundle(
      await unpackGzip(read(`assets/runtime/${bundlePath(gzip.hash)}`), gzip, hash),
    );
    expect(bundle.kind, name).toBe("static");
  }
  expect(kitDownloadBytes(catalog, templateKits(library, ids))).toBeLessThanOrEqual(
    SHARED_KIT_DOWNLOAD_MAX_BYTES,
  );
});
