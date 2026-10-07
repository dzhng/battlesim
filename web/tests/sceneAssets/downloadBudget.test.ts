// @vitest-environment node
// Shipping assets, through the native reader and the same browser codec:
// actual map selection and the complete generated catalogue, with each
// family's regional looks, retain the aggregate DOWNLOAD gate, separately
// from decoded/resident budgets.
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
import { downloadBytes, readBundle, readGzip } from "@packages/scene-assets/src/gzip.ts";
import { decodeTemplateLibrary, templateKits } from "@packages/scene-assets/src/templateLibrary.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import {
  familyLooks,
  onRequestOf,
  templateLibraryPath,
  CATALOG_LOAD_MAX_BYTES,
  MAP_DOWNLOAD_MAX_BYTES,
  KIT_BUNDLE_MAX_BYTES,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema.ts";

const root = new URL("../../../", import.meta.url);
const read = (path: string) => new Uint8Array(readFileSync(new URL(path, root)));
const json = (path: string) => JSON.parse(new TextDecoder().decode(read(path)));
const readRuntime = async (path: string) => read(`assets/runtime/${path}`);

test("the native asset reader admits the real Market Town shared-art download within the map limit", () => {
  const run = spawnSync(
    process.execPath,
    ["web/asset.mjs", "download", "fixtures/maps/market-town/map.json"],
    { cwd: root, encoding: "utf8" },
  );
  expect(run.status, run.stderr).toBe(0);
  const report = JSON.parse(run.stdout);
  expect(report.ok).toBe(true);
  expect(report.bytes).toBeGreaterThan(0);
  expect(report.bytes).toBeLessThanOrEqual(MAP_DOWNLOAD_MAX_BYTES);
});

test("the native reader refuses an oversized catalog load before opening its payload or map", () => {
  const scratch = mkdtempSync(join(tmpdir(), "kit-download-"));
  try {
    mkdirSync(join(scratch, "web"));
    mkdirSync(join(scratch, "assets/runtime"), { recursive: true });
    copyFileSync(new URL("web/asset.mjs", root), join(scratch, "web/asset.mjs"));
    symlinkSync(fileURLToPath(new URL("packages", root)), join(scratch, "packages"));
    // The CLI's session catalog factory (`web/src/battle/catalog/node.ts`).
    symlinkSync(fileURLToPath(new URL("web/src", root)), join(scratch, "web/src"));
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
          [hash]: { hash: "b".repeat(64), bytes: CATALOG_LOAD_MAX_BYTES + 1, raw_bytes: 1 },
        },
      }),
    );
    const run = spawnSync(process.execPath, ["web/asset.mjs", "download", "absent-map.json"], {
      cwd: scratch,
      encoding: "utf8",
    });
    expect(run.status).not.toBe(0);
    expect(run.stderr).toContain("catalog load");
    expect(run.stderr).not.toContain("ENOENT");
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

// A map is built in one regional family (M08), so each family's full art is
// what one map can fetch at most: the kits of all its templates and all its
// regional looks, with the library. The looks of no region load with the
// catalog for every map and are not this gate's.
async function familyDownloads(catalog: RuntimeCatalog) {
  const library = decodeTemplateLibrary(
    await readGzip(
      catalog,
      "template library",
      catalog.templates!.library,
      templateLibraryPath,
      readRuntime,
    ),
  );
  const templates: { id: string; regional_family: string }[] = json(
    "fixtures/prototype-building-templates.json",
  );
  const families: string[] = json("fixtures/map-presets.json").parcels.regional_families;
  const download = (family: string) => [
    ...templateKits(
      library,
      templates.filter((t) => t.regional_family === family).map((t) => t.id),
    ),
    ...familyLooks(onRequestOf(catalog), family),
  ];
  return { library, templates, families, download };
}

test("every regional family's full art and the library fit the shared download gate with exact raw integrity", async () => {
  const catalog = json("assets/runtime/catalog.json") as RuntimeCatalog;
  const { library, templates, families, download } = await familyDownloads(catalog);
  for (const name of templateKits(
    library,
    templates.map((template) => template.id),
  )) {
    const hash = catalog.appearances[name].bundle;
    const bundle = await readBundle(catalog, hash, readRuntime, undefined, KIT_BUNDLE_MAX_BYTES);
    expect(bundle.kind, name).toBe("static");
  }
  for (const family of families)
    expect(downloadBytes(catalog, download(family)), family).toBeLessThanOrEqual(
      MAP_DOWNLOAD_MAX_BYTES,
    );
});

/** The shipped catalog with one more regional look of `family`, `bytes` on the wire. */
function withLook(catalog: RuntimeCatalog, family: string, bytes: number): RuntimeCatalog {
  const raw = "c".repeat(64);
  return {
    ...catalog,
    appearances: {
      ...catalog.appearances,
      bench_heavy: {
        unit: "scenery",
        kind: "static",
        bundle: raw,
        scenery: "bench",
        footprint_half_m: [0.9, 0.3, 0.42],
        regional_family: family,
      },
    },
    gzip: { ...catalog.gzip, [raw]: { hash: "d".repeat(64), bytes, raw_bytes: 1 } },
  };
}

test("a family's regional looks count against its gate, and no other family's", async () => {
  const shipped = json("assets/runtime/catalog.json") as RuntimeCatalog;
  const { families } = await familyDownloads(shipped);
  const [heavy, ...others] = families;
  const catalog = withLook(shipped, heavy, MAP_DOWNLOAD_MAX_BYTES);
  const { download } = await familyDownloads(catalog);
  expect(download(heavy)).toContain("bench_heavy");
  expect(downloadBytes(catalog, download(heavy))).toBeGreaterThan(MAP_DOWNLOAD_MAX_BYTES);
  for (const family of others) {
    expect(download(family)).not.toContain("bench_heavy");
    expect(downloadBytes(catalog, download(family)), family).toBeLessThanOrEqual(
      MAP_DOWNLOAD_MAX_BYTES,
    );
  }
});

test("the loader refuses a family's oversized looks before fetching any", async () => {
  const look = withLook(
    { sides: { blue: [1, 1, 1], red: [1, 1, 1] }, skeletons: {}, appearances: {} },
    "paris",
    MAP_DOWNLOAD_MAX_BYTES + 1,
  );
  const fetched: string[] = [];
  const inner = memoryFetch(
    new Map([["catalog.json", new TextEncoder().encode(JSON.stringify(look))]]),
    "/",
  );
  const loader = new AppearanceLibrary((url) => {
    fetched.push(url);
    return inner(url);
  });
  const catalog = await loader.load("/");
  await expect(loader.withAppearances(familyLooks(catalog.onRequest, "paris"))).rejects.toThrow(
    /map download .* over/,
  );
  expect(fetched).toEqual(["/catalog.json"]);
  expect(loader.installed).toBe(catalog);
});

// Court and garden pieces load on every map (a region's bench and bins on its
// own maps), so each is held to a fixed share of the download: at most 1 MiB
// of raw art, every look of every piece.
const PIECE_MAX_RAW_BYTES = 2 ** 20;

test("every court and garden piece's art is at most 1 MiB raw, in every region's look", () => {
  const catalog = json("assets/runtime/catalog.json") as RuntimeCatalog;
  type Rows = { props: Record<string, { appearance: { drawn_by: string } }> };
  const rows = ["courts", "gardens"].flatMap((file) =>
    Object.values((json(`fixtures/props/city/${file}.json`) as Rows).props),
  );
  const sceneries = new Set(rows.map((row) => row.appearance.drawn_by));
  const looks = Object.entries(catalog.appearances).filter(
    ([, entry]) => entry.scenery !== undefined && sceneries.has(entry.scenery),
  );
  // Every piece has art: a missing look would pass a size check by absence.
  expect(new Set(looks.map(([, entry]) => entry.scenery))).toEqual(sceneries);
  // A bundle's raw art is its content and its textures (each with a header
  // line the bundle does not carry, so a few bytes over: on the safe side).
  const raw = (hash: string) =>
    catalog.gzip![hash].raw_bytes +
    (catalog.textures?.[hash] ?? []).reduce((n, id) => n + catalog.gzip![id].raw_bytes, 0);
  for (const [name, entry] of looks)
    expect(raw(entry.bundle), name).toBeLessThanOrEqual(PIECE_MAX_RAW_BYTES);
});
