// @vitest-environment node
// The Armory's yard (`fixtures/armory/yard.json`): its dressing is catalog
// props drawn by their own art, standing clear of every spot, its ground is
// paved where its pads lie and level, the land past it rises into hills each
// spot's camera sees, and each unit takes the spot its body asks for and fits
// it.
import { readFile } from "node:fs/promises";
import { expect, test } from "vitest";
import { AppearanceLibrary, type Fetch } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import type { UnitType } from "@packages/scene-assets/src/units";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import { loadWasm } from "@web/battle/sim/module";
import { loadPresentation } from "@web/battle/sim/presentation";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import {
  admitYard,
  bodyOf,
  spotFor,
  yard,
  yardGround,
  yardModels,
  yardScenery,
  yardWorld,
  yardWorldScenery,
} from "@apps/battle-lab/src/armory/yard";
import { gameBiome } from "@apps/battle-lab/src/gameBiome";

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

/** The colour of the topmost ground triangle over `p`, or null for none. */
function groundTint(mesh: Float32Array, p: [number, number]): number[] | null {
  let top: { z: number; tint: number[] } | null = null;
  for (let v = 0; v < mesh.length; v += 3 * VERTEX_FLOATS) {
    const at = (k: number) => mesh.subarray(v + k * VERTEX_FLOATS, v + k * VERTEX_FLOATS + 3);
    const [a, b, c] = [at(0), at(1), at(2)];
    const side = (u: Float32Array, w: Float32Array) =>
      (w[0] - u[0]) * (p[1] - u[1]) - (w[1] - u[1]) * (p[0] - u[0]);
    const s = [side(a, b), side(b, c), side(c, a)];
    if (!(s.every((x) => x >= 0) || s.every((x) => x <= 0))) continue;
    if (!top || a[2] > top.z) top = { z: a[2], tint: Array.from(mesh.subarray(v + 6, v + 10)) };
  }
  return top?.tint ?? null;
}

const expectTint = (got: number[] | null, want: readonly number[], what: string) => {
  expect(got, what).not.toBeNull();
  got!.forEach((x, i) => expect(x, `${what}[${i}]`).toBeCloseTo(want[i], 5));
};

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

test("the yard's art draws the trees the battle's biome grows past its ground", async () => {
  const scenery = yardWorldScenery(await yardArt());
  for (const s of gameBiome.trees.species)
    expect(scenery.appearances.has(s.appearance), s.appearance).toBe(true);
  expect(scenery.appearances.has(gameBiome.trees.hedgerows.appearance)).toBe(true);
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

test("the yard's ground is paved where its pads lie, and plain beyond them", () => {
  const ground = yardWorld().terrain.mesh;
  expect(yard.ground.pads.length).toBeGreaterThan(0);
  for (const [i, pad] of yard.ground.pads.entries()) {
    const [c, s] = [Math.cos(pad.yaw), Math.sin(pad.yaw)];
    // The centre and just inside each corner, turned with the pad.
    for (const [u, v] of [
      [0, 0],
      [0.95, 0.95],
      [-0.95, 0.95],
      [0.95, -0.95],
      [-0.95, -0.95],
    ]) {
      const [lx, ly] = [u * pad.half_m[0], v * pad.half_m[1]];
      const p: [number, number] = [pad.at[0] + c * lx - s * ly, pad.at[1] + s * lx + c * ly];
      const over = yard.ground.pads
        .slice(i + 1)
        .some((q) => outside(p, q.at, q.yaw, q.half_m) === 0);
      if (!over) expectTint(groundTint(ground, p), pad.tint, `pad ${i} at ${p}`);
    }
  }
  const edge = yard.ground.half_m - 1;
  expectTint(groundTint(ground, [-edge, -edge]), yard.ground.tint, "the ground beyond the pads");
});

test("a unit stands on the yard's pads, not sunk into them", () => {
  // The scenes hold a unit to within 2 cm of its ground, and the ground's
  // height is 0 under the pads as well.
  const ground = yardWorld().terrain.mesh;
  let top = 0;
  for (let v = 0; v < ground.length; v += VERTEX_FLOATS) top = Math.max(top, ground[v + 2]);
  expect(top).toBeGreaterThan(0);
  expect(top).toBeLessThan(0.02);
});

test("the yard stands level: every spot's clear disc and every pad is at height 0", async () => {
  await loadPresentation();
  const ground = yardGround();
  for (const [id, spot] of Object.entries(yard.spots))
    for (const r of [0, spot.clear_m / 2, spot.clear_m])
      for (let k = 0; k < 16; k++) {
        const a = (k / 16) * 2 * Math.PI;
        const [x, y] = [spot.at[0] + r * Math.cos(a), spot.at[1] + r * Math.sin(a)];
        expect(ground(x, y), `${id} at ${x}, ${y}`).toBe(0);
      }
  for (const [i, pad] of yard.ground.pads.entries()) {
    const [c, s] = [Math.cos(pad.yaw), Math.sin(pad.yaw)];
    for (const [u, v] of [
      [0, 0],
      [1, 1],
      [-1, 1],
      [1, -1],
      [-1, -1],
    ]) {
      const [lx, ly] = [u * pad.half_m[0], v * pad.half_m[1]];
      const [x, y] = [pad.at[0] + c * lx - s * ly, pad.at[1] + s * lx + c * ly];
      expect(ground(x, y), `pad ${i} at ${x}, ${y}`).toBe(0);
    }
  }
});

test("past the yard's ground the land rises into hills on each spot's skyline", async () => {
  await loadPresentation();
  const ground = yardGround();
  const edge = yard.ground.half_m;
  // Level out to the yard's edge, all round.
  for (let x = -edge; x <= edge; x += edge / 8)
    for (const y of [-edge, edge]) {
      expect(ground(x, y), `${x}, ${y}`).toBe(0);
      expect(ground(y, x), `${y}, ${x}`).toBe(0);
    }
  // Seen from each spot's opening eye, looking the way it looks, the land
  // past the yard climbs above the eye somewhere: a hill on the skyline.
  for (const [id, spot] of Object.entries(yard.spots)) {
    const { target, distance, pitch, yaw } = spot.camera;
    const eye = [
      target[0] + distance * Math.cos(pitch) * Math.cos(yaw),
      target[1] + distance * Math.cos(pitch) * Math.sin(yaw),
      target[2] + distance * Math.sin(pitch),
    ];
    const look = [-Math.cos(yaw), -Math.sin(yaw)];
    let above = -Infinity;
    for (let d = edge; d < 20 * edge; d += 5) {
      const [x, y] = [eye[0] + look[0] * d, eye[1] + look[1] * d];
      above = Math.max(above, ground(x, y) - eye[2]);
    }
    expect(above, id).toBeGreaterThan(0);
  }
});

test("a yard that leaves a body without a spot is refused", () => {
  const { aircraft: _, ...bodies } = yard.bodies;
  expect(() => admitYard({ ...yard, bodies })).toThrow(/bodies\.aircraft/);
});

test("every unit fits its spot's clear disc", async () => {
  const wasm = await loadWasm();
  const showground = new wasm.Showground(JSON.stringify(catalog.rules));
  try {
    for (const id of units.ids) {
      const type = units.type(id);
      const spot = spotFor(yard, type);
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
  expect(spotFor(yard, squad).id).toBe(yard.bodies.squad);
  expect(spotFor(yard, vehicle).id).toBe(yard.bodies.ground);
  expect(spotFor(yard, rotorcraft).id).toBe(yard.bodies.aircraft);
  expect(new Set([yard.bodies.squad, yard.bodies.ground, yard.bodies.aircraft]).size).toBe(3);
});
