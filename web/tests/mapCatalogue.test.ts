// @vitest-environment node
// The saved-map catalogue (`fixtures/maps/<id>/`) as JavaScript meets it:
// every folder's listing metadata is valid and says what the map itself
// says, every map resolves through the WebAssembly resolver to the identity
// its sources pin (the identity the native reader checks too), and a folder
// that does not resolve is refused by name.
import { cpSync, existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, expect, test } from "vitest";
import fixtures from "@apps/battle-lab/src/fixtures.json";
import {
  checkMapFolder,
  filterMaps,
  listMaps,
  validateMapMeta,
  type MapEntry,
  type MapMeta,
} from "@web/maps/catalogue";
import { openCatalogue, shipped } from "@web/maps/node";
import { MapResolveError, resolveSavedMap, type MapIdentity } from "@web/maps/resolve";
import { resolve_saved_map } from "@wasm/game_wasm.js";

const FIXTURES = join(import.meta.dirname, "../../fixtures");
const ids = shipped.ids();

test("the listing is the catalogue's folders, each with valid metadata that says what its map says", () => {
  expect(ids.length).toBeGreaterThan(0);
  expect(listMaps().map((m) => m.id)).toEqual(ids);
  const routes = fixtures.map((f) => f.id);
  for (const id of ids) {
    const meta = validateMapMeta(id, shipped.meta(id));
    const { definition, identity } = shipped.load(id);
    expect(
      checkMapFolder(id, meta, { definition, identity, encounters: shipped.encounters(id) }),
    ).toEqual([]);
    expect(existsSync(join(FIXTURES, "biomes", `${meta.biome}.json`)), `${id}'s biome`).toBe(true);
    for (const benchmark of meta.benchmarks)
      expect(routes, `${id}'s benchmark`).toContain(benchmark);
  }
});

test("every map resolves to the identity its sources pin, and its definition survives JavaScript", () => {
  for (const id of ids) {
    const { definition, identity } = shipped.load(id);
    const sources = JSON.parse(shipped.document(id, "SOURCES.json")) as { identity: MapIdentity };
    expect(identity, id).toEqual(sources.identity);
    // The definition a route hands the simulation is the one that was
    // resolved: parsed and printed by JavaScript, it is still the map its
    // sources pin.
    const again = resolveSavedMap(resolve_saved_map, id, {
      map: JSON.stringify(definition),
      sources: shipped.document(id, "SOURCES.json"),
      library: shipped.library(),
    });
    expect(again.identity, id).toEqual(identity);
    expect(again.definition, id).toEqual(definition);
  }
});

test("every saved encounter is listed by its map and lists its units", () => {
  let encounters = 0;
  for (const id of ids)
    for (const name of shipped.encounters(id)) {
      expect(shipped.encounter(id, name).units.length, `${id}/${name}`).toBeGreaterThan(0);
      encounters++;
    }
  expect(encounters).toBeGreaterThan(0);
});

const META: MapMeta = {
  category: "lab",
  status: "released",
  label: "A lab",
  character: "arena",
  biome: "summer",
  size_m: [640, 480],
  tags: ["road", "building"],
  source: "authored",
  seed: null,
  encounters: ["first"],
  benchmarks: [],
};

test("metadata a listing could not show is refused, naming the file and the field", () => {
  expect(validateMapMeta("a-lab", META)).toEqual(META);
  const refused = (change: Record<string, unknown>, id = "a-lab") => {
    const meta: Record<string, unknown> = { ...META, ...change };
    for (const [key, value] of Object.entries(change)) if (value === undefined) delete meta[key];
    try {
      validateMapMeta(id, meta);
    } catch (e) {
      return (e as Error).message;
    }
    return "accepted";
  };
  expect(refused({ category: "arcade" })).toMatch(/^fixtures\/maps\/a-lab\/meta\.json\.category: /);
  expect(refused({ status: undefined })).toBe("fixtures/maps/a-lab/meta.json.status: missing");
  expect(refused({ colour: "red" })).toBe("fixtures/maps/a-lab/meta.json.colour: unknown field");
  expect(refused({ size_m: [640, 0] })).toMatch(/meta\.json\.size_m: /);
  expect(refused({ tags: ["road", "road"] })).toMatch(/meta\.json\.tags: "road" is listed twice/);
  expect(refused({ tags: ["moat"] })).toMatch(/meta\.json\.tags: "moat"/);
  expect(refused({ encounters: ["../first"] })).toMatch(/meta\.json\.encounters: /);
  // A seed is a generated map's, as decimal text.
  expect(refused({ seed: "7" })).toMatch(/meta\.json\.seed: /);
  expect(refused({ source: "generated" })).toMatch(/meta\.json\.seed: /);
  expect(refused({ source: "generated", seed: 7 })).toMatch(/meta\.json\.seed: /);
  expect(refused({ source: "generated", seed: "7" })).toBe("accepted");
  expect(refused({}, "A Lab")).toMatch(/^fixtures\/maps\/A Lab\/meta\.json: /);
});

test("metadata that disagrees with its map is reported field by field", () => {
  const { definition, identity } = shipped.load("geometry");
  const meta = validateMapMeta("geometry", shipped.meta("geometry"));
  const folder = { definition, identity, encounters: shipped.encounters("geometry") };
  expect(checkMapFolder("geometry", meta, folder)).toEqual([]);
  const problems = checkMapFolder(
    "geometry",
    { ...meta, size_m: [1, 1], tags: [], encounters: ["absent"], source: "generated", seed: "7" },
    folder,
  );
  expect(problems.map((p) => p.split(":")[0])).toEqual([
    "fixtures/maps/geometry/meta.json.size_m",
    "fixtures/maps/geometry/meta.json.tags",
    "fixtures/maps/geometry/meta.json.encounters",
    "fixtures/maps/geometry/meta.json.source",
  ]);
  // A generated map's listing takes its seed from the generation it pins.
  const generation = {
    generator_version: "layout-4",
    preset_revision: "layout-presets-6",
    seed: "41",
    config_hash: "0".repeat(64),
    template_catalog_hash: "0".repeat(64),
    map_hash: "0".repeat(64),
  };
  const generated = { ...folder, identity: { kind: "generated", generation } as MapIdentity };
  expect(
    checkMapFolder("geometry", { ...meta, source: "generated", seed: "41" }, generated),
  ).toEqual([]);
  expect(
    checkMapFolder("geometry", { ...meta, source: "generated", seed: "7" }, generated).join(),
  ).toMatch(/meta\.json\.seed: "7", but the generation's seed is "41"/);
  expect(checkMapFolder("geometry", meta, generated).join()).toMatch(/meta\.json\.source/);
});

test("a listing filters by category, status and feature, in id order", () => {
  const entry = (id: string, change: Partial<MapMeta>): MapEntry => ({ id, ...META, ...change });
  const entries = [
    entry("b", { category: "playable" }),
    entry("c", { status: "draft" }),
    entry("a", { tags: ["river"] }),
  ];
  expect(filterMaps(entries).map((m) => m.id)).toEqual(["a", "b", "c"]);
  expect(filterMaps(entries, { category: "lab" }).map((m) => m.id)).toEqual(["a", "c"]);
  expect(filterMaps(entries, { category: "lab", status: "released" }).map((m) => m.id)).toEqual([
    "a",
  ]);
  expect(filterMaps(entries, { tag: "building" }).map((m) => m.id)).toEqual(["b", "c"]);
});

const scratch = mkdtempSync(join(tmpdir(), "battlegame-maps-"));
afterAll(() => rmSync(scratch, { recursive: true, force: true }));

test("a map that does not resolve is refused naming the document at fault, never replaced", () => {
  cpSync(join(FIXTURES, "maps/geometry"), join(scratch, "geometry"), { recursive: true });
  const library = join(FIXTURES, "building-templates.json");
  const refusal = (
    id: string,
    read: (c: ReturnType<typeof openCatalogue>) => unknown = (c) => c.load(id),
  ) => {
    try {
      read(openCatalogue(scratch, library));
    } catch (e) {
      if (!(e instanceof MapResolveError)) throw e;
      return { code: e.code, location: e.location };
    }
    return "resolved";
  };
  expect(refusal("geometry")).toBe("resolved");
  // An address is one folder name: a path is refused before anything is read.
  expect(refusal("../geometry")).toMatchObject({ code: "invalid_id" });
  expect(refusal("nowhere")).toEqual({ code: "missing_document", location: "nowhere/map.json" });
  expect(refusal("geometry", (c) => c.encounter("geometry", "absent"))).toEqual({
    code: "missing_document",
    location: "geometry/encounters/absent.json",
  });

  // A map edited without its sources no longer is the map they pin.
  const map = shipped.document("geometry", "map.json");
  writeFileSync(
    join(scratch, "geometry/map.json"),
    map.replace('"fog_cell_m": 8', '"fog_cell_m": 16'),
  );
  expect(refusal("geometry")).toEqual({
    code: "identity_mismatch",
    location: "geometry/SOURCES.json.identity.map_hash",
  });
  writeFileSync(join(scratch, "geometry/map.json"), map);

  rmSync(join(scratch, "geometry/SOURCES.json"));
  expect(refusal("geometry")).toEqual({
    code: "missing_document",
    location: "geometry/SOURCES.json",
  });
});
