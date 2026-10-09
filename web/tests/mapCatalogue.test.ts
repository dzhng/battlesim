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
  categoryMap,
  checkMapFolder,
  listMaps,
  validateMapMeta,
  type MapEntry,
  type MapMeta,
} from "@web/maps/catalogue";
import { openCatalogue, shipped } from "@web/maps/node";
import { MapResolveError, type MapIdentity } from "@web/maps/resolve";

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

test("every map resolves to the identity its sources pin, and the definition handed on is the resolved one", () => {
  for (const id of ids) {
    const { definition, identity, json } = shipped.load(id);
    const sources = JSON.parse(shipped.document(id, "SOURCES.json")) as { identity: MapIdentity };
    expect(identity, id).toEqual(sources.identity);
    // Preparation splices a scenario from the resolver's own text.
    expect(JSON.parse(json), id).toEqual(definition);
    // A lab hands the simulation the definition as JavaScript prints it. An
    // authored map survives that: it holds no number JavaScript prints as
    // another (a negative zero).
    if (identity.kind === "authored")
      expect(JSON.parse(JSON.stringify(definition)), id).toEqual(definition);
  }
});

test("a saved building is its template and frame, and resolves to the template's geometry there", () => {
  const saved = JSON.parse(shipped.document("geometry", "map.json")) as {
    buildings: Record<string, unknown>[];
  };
  expect(Object.keys(saved.buildings[0]).sort()).toEqual([
    "frame",
    "kind",
    "owner",
    "parts",
    "template_id",
  ]);
  const [building] = shipped.load("geometry").definition.buildings!;
  expect(building.geometry.template_id).toBe(saved.buildings[0].template_id);
  expect(building.geometry.frame).toEqual(saved.buildings[0].frame);
  expect(building.geometry.parts.map((part) => part.id)).toEqual(
    building.parts.map((reference) => reference.part),
  );
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
  category: "test",
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
  // A saved map is a test's or the menu's: no game, lab or benchmark map.
  for (const category of ["playable", "lab", "benchmark", undefined])
    expect(refused({ category })).toMatch(/^fixtures\/maps\/a-lab\/meta\.json\.category: /);
  expect(refused({ category: "menu" })).toBe("accepted");
  expect(refused({ character: "farmland" })).toMatch(/meta\.json\.character: /);
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

test("a map is taken only for what its category says it is for", () => {
  const entries = [
    { id: "town", ...META, category: "menu" },
    { id: "range", ...META, category: "test" },
  ] satisfies MapEntry[];
  expect(categoryMap(entries, "town", "menu").id).toBe("town");
  expect(categoryMap(entries, "range", "test").id).toBe("range");
  expect(() => categoryMap(entries, "range", "menu")).toThrow(
    'the saved map "range" is a test map, not a menu map',
  );
  expect(() => categoryMap(entries, "town", "test")).toThrow(/is a menu map, not a test map/);
  expect(() => categoryMap(entries, "nowhere", "test")).toThrow('no saved map "nowhere"');
});

test("every lab stands on a test map", () => {
  for (const fixture of fixtures)
    if (fixture.map) expect(categoryMap(listMaps(), fixture.map, "test").id).toBe(fixture.map);
});

const scratch = mkdtempSync(join(tmpdir(), "battlegame-maps-"));
afterAll(() => rmSync(scratch, { recursive: true, force: true }));

test("a map that does not resolve is refused naming the document at fault, never replaced", () => {
  cpSync(join(FIXTURES, "maps/geometry"), join(scratch, "geometry"), { recursive: true });
  const refusal = (
    id: string,
    read: (c: ReturnType<typeof openCatalogue>) => unknown = (c) => c.load(id),
  ) => {
    try {
      read(openCatalogue(scratch, FIXTURES));
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

  // A map names its physical library by file name: a path is refused before
  // it is read, and a library the catalogue lacks is a missing document.
  const sources = shipped.document("geometry", "SOURCES.json");
  for (const [library, code] of [
    ["../fixtures/building-templates.json", "invalid_sources"],
    ["absent-templates.json", "missing_document"],
  ]) {
    writeFileSync(
      join(scratch, "geometry/SOURCES.json"),
      sources.replace('"building-templates.json"', JSON.stringify(library)),
    );
    expect(refusal("geometry")).toEqual({
      code,
      location: "geometry/SOURCES.json.catalogue.library",
    });
  }

  rmSync(join(scratch, "geometry/SOURCES.json"));
  expect(refusal("geometry")).toEqual({
    code: "missing_document",
    location: "geometry/SOURCES.json",
  });
});
