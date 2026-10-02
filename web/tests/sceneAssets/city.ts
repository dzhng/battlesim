// A tiny city set for the scene-assets tests: a kit of two modules, physical
// descriptors the contract admits, and a set that dresses them. Generated in
// code, with knobs that break exactly one rule each.

import type { Vec3 } from "math";
import type { BakeContext } from "@packages/scene-assets/src/bake.ts";
import type { Catalog } from "@packages/scene-assets/src/schema.ts";
import type {
  TemplateDescriptor,
  TemplatePart,
  TemplateSetSource,
} from "@packages/scene-assets/src/templateSource.ts";
import { physicalTemplates } from "@web/maps/node";
import { UnitCatalog } from "@packages/scene-assets/src/units.ts";
import { AUTHORITY, GltfBuilder, TOLERANCES, gBox } from "./synthetic";

export const KIT = "city_kit_test";
export const KIT_SOURCE = "assets/source/city/test/kit.glb";
export const SET_SOURCE = "assets/source/city/test/templates.json";

/** A module's geometry in its own frame: boxes, each in one tier or (null) in all. */
export interface ModuleSpec {
  name: string;
  /** Where the module's empty sits in the file; its geometry goes with it. */
  at?: Vec3;
  boxes: { tier: number | null; min: Vec3; max: Vec3 }[];
}

/** One box in every tier. */
export const solid = (name: string, min: Vec3, max: Vec3, at?: Vec3): ModuleSpec => ({
  name,
  at,
  boxes: [{ tier: null, min, max }],
});

/** The test kit's modules: `shell`, a unit box standing on its origin, and
 *  `sill`, a ledge reaching 0.4 m out from a wall plane at y = 0, laid out
 *  beside it in the file. */
export const MODULES: ModuleSpec[] = [
  solid("shell", [-0.5, -0.5, 0], [0.5, 0.5, 1]),
  solid("sill", [-0.5, -0.4, 0], [0.5, 0, 0.2], [40, 0, 0]),
];

/** A kit GLB: each module a root-level empty, its meshes under it. */
export function kitGlb(modules: ModuleSpec[] = MODULES, looseMesh = false): Uint8Array {
  const b = new GltfBuilder();
  const roots = modules.map((module) => {
    const at = module.at ?? [0, 0, 0];
    const children = module.boxes.map((box, i) =>
      b.node({
        name: `${module.name}_${i}${box.tier === null ? "" : `_LOD${box.tier}`}`,
        mesh: gBox(b, box.min, box.max),
      }),
    );
    // Engine (x, y, z) is glTF (x, z, -y).
    return b.node({ name: module.name, t: [at[0], at[2], -at[1]], children });
  });
  if (looseMesh) roots.push(b.node({ name: "stray", mesh: gBox(b, [0, 0, 0], [1, 1, 1]) }));
  b.roots(...roots);
  return b.glb();
}

/** A facade edge covering one whole face of `part`. */
const face = (part: TemplatePart, name: string, facade: string, half: number) => ({
  id: `${part.id}-${name}`,
  part: part.id,
  facade,
  span_m: [-half, half],
  exposed: true,
  bays: { pitch_m: 3, phase_m: 1.5 },
});

/** A complete physical template of free-standing boxes (`parts`: centre and
 *  half extents, on the ground), with a door on the first part's south face. */
export function descriptor(
  id: string,
  parts: { id: string; center: [number, number]; half: Vec3; yaw?: number }[] = [
    { id: "body", center: [0, 0], half: [6, 4, 3] },
  ],
): TemplateDescriptor {
  const boxes: TemplatePart[] = parts.map((p) => ({
    id: p.id,
    center: p.center,
    yaw: p.yaw ?? 0,
    half_extents: p.half,
    base_z: 0,
  }));
  return {
    id,
    category: "detached_home",
    regional_family: "test",
    parts: boxes,
    floor_heights_m: [0],
    entrances: [{ id: "door-0", edge: `${boxes[0].id}-south`, offset_m: 0 }],
    edges: boxes.flatMap((p) => [
      face(p, "east", "positive_x", p.half_extents[1]),
      face(p, "north", "positive_y", p.half_extents[0]),
      face(p, "west", "negative_x", p.half_extents[1]),
      face(p, "south", "negative_y", p.half_extents[0]),
    ]),
    joins: [],
  };
}

/** A source row: module index, position, yaw, scale, every tier, a tint. */
export const row = (
  module: number,
  at: Vec3,
  scale: Vec3 = [1, 1, 1],
  yaw = 0,
  tint: Vec3 = [255, 255, 255],
  tiers = 15,
): number[] => [module, ...at, yaw, ...scale, tiers, ...tint];

/** The shell stretched to a part: the part itself, as art. */
export const shellRow = (part: TemplatePart, tint: Vec3 = [200, 180, 160]): number[] =>
  row(
    0,
    [part.center[0], part.center[1], part.base_z],
    [2 * part.half_extents[0], 2 * part.half_extents[1], 2 * part.half_extents[2]],
    part.yaw,
    tint,
  );

export const HOUSE = descriptor("test-house");
export const YARD = descriptor("test-yard", [
  { id: "house", center: [-10, 0], half: [5, 4, 3] },
  { id: "barn", center: [10, 0], half: [4, 6, 2], yaw: 0.5 },
]);

/** A set that dresses HOUSE (its shell, and a sill on its south wall) and
 *  YARD (a shell per part), within a 0.5 m side and 1 m top fit. */
export function testSet(edit: (set: TemplateSetSource) => void = () => {}): TemplateSetSource {
  const set: TemplateSetSource = {
    set: "test",
    kit: KIT,
    fit: { side_m: 0.5, top_m: 1 },
    source: { script: "city.ts" },
    modules: ["shell", "sill"],
    templates: [
      {
        status: "release",
        descriptor: structuredClone(HOUSE),
        states: { intact: [shellRow(HOUSE.parts[0]), row(1, [2, -4, 1.2])] },
      },
      {
        status: "prototype",
        descriptor: structuredClone(YARD),
        states: { intact: YARD.parts.map((part) => shellRow(part)) },
      },
    ],
  };
  edit(set);
  return set;
}

export const setBytes = (set: unknown): Uint8Array =>
  new TextEncoder().encode(typeof set === "string" ? set : JSON.stringify(set));

/** A catalog of the test kit and set and nothing else. */
export function cityCatalog(): Catalog {
  return {
    tolerances: TOLERANCES,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    skeletons: {},
    appearances: { [KIT]: { unit: "kit", source: KIT_SOURCE, basis_yaw_deg: 0 } },
    city_sets: { test: { templates: SET_SOURCE, kit: KIT } },
  };
}

export function citySources(
  set: unknown = testSet(),
  kit: Uint8Array = kitGlb(),
): Record<string, Uint8Array> {
  return { [KIT_SOURCE]: kit, [SET_SOURCE]: setBytes(set) };
}

/** No unit types: a city catalog draws none. */
const NO_UNITS = new UnitCatalog({
  documents: [],
  props: {},
  roles: {},
  parts: {},
  soldiers: {},
  units: [],
});

/** The bake's context for a physical catalogue of `rows` (the set's own by default). */
export const cityContext = (rows: unknown[] = [HOUSE, YARD]): BakeContext => ({
  authority: { ...AUTHORITY, units: NO_UNITS },
  templates: { catalogue: rows, physical: physicalTemplates() },
});
