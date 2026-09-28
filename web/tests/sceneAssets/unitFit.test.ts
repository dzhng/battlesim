// @vitest-environment node
// Each unit type's model fits its own resolved numbers. The worked example
// (test only, never shipped): an M1 family off the shipped tank, resolved by
// the simulation's one resolver, where the M1A2 has its own model, inherits
// the tank's turret and gun geometry, and lists a part whose hardware its
// model must draw. And the generated check: every shipped type's appearance
// exists and its installed model fits the type.
import { readFileSync } from "node:fs";
import type { Vec3 } from "math";
import { beforeAll, expect, test } from "vitest";
import { initSync, resolve_catalog } from "@wasm/game_wasm.js";
import { decodeBundle } from "@packages/scene-assets/src/codec.ts";
import { lfsPointerOid } from "@packages/scene-assets/src/glb.ts";
import {
  bundlePath,
  type ArticulatedBundle,
  type Catalog,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema.ts";
import { UnitCatalog, type CatalogView } from "@packages/scene-assets/src/units.ts";
import {
  typeAppearanceFindings,
  typeFindings,
  validateAppearance,
} from "@packages/scene-assets/src/validate.ts";
import { AUTHORITY, TOLERANCES, tankGlb } from "./synthetic";

const read = (path: string) => readFileSync(new URL(path, import.meta.url));
const json = (path: string) => JSON.parse(read(path).toString("utf8"));
const shipped = json("../../../fixtures/unit-catalog.json") as CatalogView;

let family: UnitCatalog;
beforeAll(() => {
  initSync({ module: read("../../src/wasm/game_wasm_bg.wasm") });
  const documents = [
    ...shipped.documents,
    json("../../../crates/sim/tests/fixtures/m1-family.json"),
  ];
  family = new UnitCatalog(JSON.parse(resolve_catalog(JSON.stringify(documents))) as CatalogView);
});

/** The M1A2's findings for a synthetic tank drawn with `options`. */
async function m1a2(options: Parameters<typeof tankGlb>[0]) {
  const result = await validateAppearance(
    {
      name: "m1a2",
      entry: { unit: "vehicle", source: "m1a2.glb", basis_yaw_deg: 0 },
      files: { "m1a2.glb": tankGlb({ mounts: family.type("m1a2").mounts, ...options }) },
    },
    { authority: { ...AUTHORITY, units: family }, tolerances: TOLERANCES },
  );
  return result.findings;
}

test("the M1A2 inherits the tank's mount geometry and its part's hardware requirement", () => {
  const [tank, m1a2] = [family.type("tank"), family.type("m1a2")];
  expect(m1a2.appearance).toBe("m1a2");
  expect(m1a2.mounts.map((m) => [m.pivot_m, m.muzzle_m])).toEqual(
    tank.mounts.map((m) => [m.pivot_m, m.muzzle_m]),
  );
  expect(m1a2.parts).toEqual(["era"]);
  expect(family.view.parts.era.nodes).toEqual(["era_*"]);
});

test("its own model fits when it draws the turret and gun where the inherited mounts say", async () => {
  expect(await m1a2({ era: true })).toEqual([]);
});

test("a model whose turret sits elsewhere does not fit the inherited mounts", async () => {
  // The whole turret 0.8 m further back: its gun and roof HMG with it.
  const back = family.type("m1a2").mounts.map((m) => ({
    ...m,
    pivot_m: [m.pivot_m[0] - 0.8, m.pivot_m[1], m.pivot_m[2]] as Vec3,
  }));
  const moved = (await m1a2({ era: true, mounts: back })).map((f) => f.code);
  expect(moved).toContain("fit.vehicle_muzzle");
  expect(moved).toContain("fit.muzzle_arc");
  // The turret's pivot alone off its row: right at rest, wrong as it turns.
  const pivot = (await m1a2({ era: true, turretX: -0.8 })).map((f) => f.code);
  expect(pivot).toContain("fit.muzzle_arc");
});

test("a model without the part's hardware does not fit a type listing the part", async () => {
  const findings = await m1a2({});
  expect(findings.map((f) => f.code)).toEqual(["fit.part_nodes"]);
  expect(findings[0].message).toMatch(/\(as m1a2\).*part era.*"era_\*"/);
});

test("every shipped unit type draws appearances the catalog has, and its model fits it", () => {
  const units = new UnitCatalog(shipped);
  const catalog = json("../../../assets/catalog.json") as Catalog;
  expect(typeAppearanceFindings(catalog.appearances, units)).toEqual([]);
  const runtime = json("../../../assets/runtime/catalog.json") as RuntimeCatalog;
  const hulls = units.ids.filter((id) => units.hull(id));
  expect(hulls.length).toBeGreaterThan(0);
  for (const id of hulls) {
    const name = units.type(id).appearance!;
    const bytes = new Uint8Array(
      read(`../../../assets/runtime/${bundlePath(runtime.appearances[name].bundle)}`),
    );
    const pointer = lfsPointerOid(bytes);
    expect(
      pointer,
      `assets/runtime is LFS pointers: git lfs pull --include="assets/runtime/**"`,
    ).toBeNull();
    const bundle = decodeBundle(bytes) as ArticulatedBundle;
    const tolerances = { ...catalog.tolerances, ...catalog.appearances[name].tolerances };
    expect(typeFindings(name, bundle.nodes, units, id, tolerances), id).toEqual([]);
  }
});
