// @vitest-environment node
// The Armory's yard (`fixtures/armory/yard.json`): its dressing is catalog
// props drawn by their own art, standing clear of every spot, and each unit
// takes the spot its body asks for and fits it.
import { readFile } from "node:fs/promises";
import { expect, test } from "vitest";
import { AppearanceLibrary, type Fetch } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import type { UnitType } from "@packages/scene-assets/src/units";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import { loadWasm } from "@web/battle/sim/module";
import { bodyOf, spotFor, yard, yardModels, yardScenery } from "@apps/battle-lab/src/armory/yard";

const runtime = new URL("../../assets/runtime/", import.meta.url);
const catalog = await resolveCatalogSet("game");
const { units } = catalog;
const props = units.view.props;

/** The runtime files read from disk as the page fetches them. */
const diskFetch: Fetch = async (url) => {
  const bytes = await readFile(new URL(url.replace(/^\//, ""), runtime)).catch(() => null);
  return {
    ok: bytes !== null,
    status: bytes ? 200 : 404,
    arrayBuffer: async () =>
      bytes!.buffer.slice(bytes!.byteOffset, bytes!.byteOffset + bytes!.length),
    json: async () => JSON.parse(bytes!.toString("utf8")) as unknown,
  };
};

/** The yard's art as the page loads it: its scenery names, and nothing else. */
async function yardArt() {
  const files = JSON.parse(
    await readFile(new URL("catalog.json", runtime), "utf8"),
  ) as RuntimeCatalog;
  const library = new AppearanceLibrary(diskFetch);
  return library.load("/", new Set(), null, yardScenery(yard, props, files));
}

/** How far point `p` lies outside a box of half sides `half` centred on `at`
 *  and turned by `yaw` (0 inside). */
function outside(p: [number, number], at: [number, number], yaw: number, half: [number, number]) {
  const [dx, dy] = [p[0] - at[0], p[1] - at[1]];
  const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
  const local = [c * dx + s * dy, -s * dx + c * dy];
  return Math.hypot(...local.map((v, i) => Math.max(0, Math.abs(v) - half[i])));
}

/** A fabricated unit type: a body and a mobility, nothing the yard reads besides. */
const card = (body: UnitType["body"], mobility: UnitType["mobility"]): UnitType =>
  ({ ...units.type(units.ids[0]), id: "fabricated", roles: ["recon"], body, mobility }) as UnitType;

test("every yard prop is a catalog prop kind with an appearance", async () => {
  const installed = await yardArt();
  const models = yardModels(yard, props, installed);
  expect(models).toHaveLength(yard.props.length);
  yard.props.forEach((p, i) => {
    expect(props[p.kind], p.kind).toBeDefined();
    const drawn = installed.appearances.get(models[i].appearance)!;
    expect(drawn.scenery).toBe(props[p.kind].appearance.drawn_by);
    expect(models[i]).toMatchObject({ x: p.at[0], y: p.at[1], z: 0, yaw: p.yaw });
  });
});

test("no yard prop stands inside any spot's clear disc", async () => {
  const installed = await yardArt();
  const models = yardModels(yard, props, installed);
  for (const [id, spot] of Object.entries(yard.spots))
    models.forEach((m, i) => {
      const [hx, hy] = installed.appearances.get(m.appearance)!.footprint!;
      const gap = outside(spot.at, [m.x, m.y], m.yaw, [hx, hy]);
      expect(gap, `${yard.props[i].kind} #${i} against ${id}`).toBeGreaterThan(spot.clear_m);
    });
});

test("every body has a spot", () => {
  for (const id of units.ids) {
    const spot = spotFor(yard, units.type(id));
    expect(spot, id).not.toBeNull();
    expect(yard.spots[spot!.id]).toBeDefined();
  }
  expect(spotFor(yard, null)).toBeNull();
});

test("every unit fits its spot's clear disc", async () => {
  const wasm = await loadWasm();
  const showground = new wasm.Showground(JSON.stringify(catalog.rules));
  try {
    for (const id of units.ids) {
      const type = units.type(id);
      const spot = spotFor(yard, type)!;
      const hull = units.hull(id);
      const reach = hull
        ? Math.hypot(hull.half_extents_m[0], hull.half_extents_m[1])
        : Math.max(
            ...[1, 7, 42, 1234].flatMap((seed) =>
              (
                JSON.parse(showground.squad(id, seed)) as { soldiers: { at: [number, number] }[] }
              ).soldiers.map((s) => Math.hypot(...s.at)),
            ),
          );
      expect(reach, `${id} at ${spot.id}`).toBeLessThanOrEqual(spot.clear_m);
    }
  } finally {
    showground.free();
  }
});

test("a recon squad stands at the tents and a recon vehicle in the motor pool", () => {
  const hull = units.hull(units.ids.find((id) => units.hull(id))!)!;
  const squad = card({ squad: { slots: ["rifleman"] } }, { foot: {} } as UnitType["mobility"]);
  const vehicle = card({ hull }, { wheeled: {} } as UnitType["mobility"]);
  const rotorcraft = card({ hull }, { air: { flight: "rotor" } } as UnitType["mobility"]);
  expect(bodyOf(squad)).toBe("squad");
  expect(bodyOf(vehicle)).toBe("ground");
  expect(bodyOf(rotorcraft)).toBe("aircraft");
  expect(spotFor(yard, squad)!.id).toBe(yard.bodies.squad);
  expect(spotFor(yard, vehicle)!.id).toBe(yard.bodies.ground);
  expect(spotFor(yard, rotorcraft)!.id).toBe(yard.bodies.aircraft);
  expect(new Set([yard.bodies.squad, yard.bodies.ground, yard.bodies.aircraft]).size).toBe(3);
});
