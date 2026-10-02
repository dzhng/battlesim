// A map's buildings as the frame draws them from template art: compact
// references for the whole map (which template, where it was placed, whose
// it is), and what a side knows fell. Nothing here holds a module transform;
// `buildingPlacements.ts` expands the references near the camera.
//
// Every building of every map is drawn this way, from its template's rows in
// the installed template art library: a generated town's, the village's
// houses, a lab's one box. Nothing else draws a building, and a template the
// library lacks is refused by name when the scene is built.
//
// Knowledge is the rule every structure follows: a building stands intact
// until the side has seen it destroyed, and is then drawn in the state the
// simulation published of it (`fallenBuildings`), never one worked out here.
import type { TemplateState } from "@packages/scene-assets/src/templateLibrary";
import type { KnownProp, MapProp } from "./propAppearance";
import type { PublicBuildings } from "../worldMesh";

type Rgb = readonly [number, number, number];

/** `presentation.buildings` in the fixture. */
export interface BuildingStyle {
  /** The detail tiers, by how many device pixels a metre of wall covers:
   *  above `[0]` tier 0, above `[1]` tier 1, above `[2]` tier 2, else tier 3.
   *  A window is the same size on a house and on a tower, so the tier follows
   *  distance, not a building's height. */
  lod_px_per_m: [number, number, number];
  /** Square chunks buildings are bucketed in, metres: a chunk takes one tier. */
  chunk_m: number;
  /** Module instances the residency pool holds at most (`placementPool.ts`). */
  pool_records: number;
  /** Rows one view change may expand into the pool; chunks past it wait a
   *  frame at the coarsest tier. */
  expand_rows: number;
  /** Each building's tint strays this far in value, either way, so identical
   *  neighbours part. */
  tint_jitter: number;
  /** A building category's wall colour in the prototype set (sRGB), with a
   *  `default`: read by the set's generator (`asset prototypes`), not here. */
  prototype_tints: Record<string, Rgb>;
}

export function validateBuildingStyle(style: BuildingStyle): BuildingStyle {
  const at = "presentation.buildings";
  const rgb = (c: unknown) =>
    Array.isArray(c) && c.length === 3 && c.every((v) => typeof v === "number" && v >= 0 && v <= 1);
  const [t0, t1, t2] = style.lod_px_per_m ?? [];
  if (!(t0 > t1 && t1 > t2 && t2 > 0))
    throw new Error(`${at}.lod_px_per_m must fall and stay above zero`);
  if (!(style.chunk_m >= 16)) throw new Error(`${at}.chunk_m must be 16 m or more`);
  if (!(Number.isInteger(style.pool_records) && style.pool_records >= 1024))
    throw new Error(`${at}.pool_records must be a whole number, 1024 or more`);
  if (!(Number.isInteger(style.expand_rows) && style.expand_rows >= 1))
    throw new Error(`${at}.expand_rows must be a whole number, 1 or more`);
  if (!(style.tint_jitter >= 0 && style.tint_jitter <= 0.5))
    throw new Error(`${at}.tint_jitter must be within [0, 0.5]`);
  if (!style.prototype_tints?.default) throw new Error(`${at}.prototype_tints needs a default`);
  for (const [name, tint] of Object.entries(style.prototype_tints))
    if (!rgb(tint)) throw new Error(`${at}: ${name} must be [r, g, b] in [0, 1]`);
  return style;
}

/** Floats per building frame: x, y, z, then yaw (radians about +Z). */
export const FRAME_FLOATS = 4;

/** A map's buildings, as references to their templates. */
export interface PlacedBuildings {
  /** The template ids `template` names. */
  templates: string[];
  /** Per building: its template, an index into `templates`. */
  template: Uint16Array;
  /** Per building: the frame it was placed at, `FRAME_FLOATS` each. */
  frames: Float64Array;
  /** Per building: its owner prop, the building's identity on the map. */
  owners: Uint32Array;
}

/** The states a destroyed building is known in: collapsed to remains, or
 *  standing as a burnt shell. */
export type DamageState = Exclude<TemplateState, "intact">;

/** A building a side knows is no longer intact. */
export interface FallenBuilding {
  /** Which of `PlacedBuildings`. */
  building: number;
  /** The state its template is drawn in. */
  state: DamageState;
}

/** The buildings a side draws: the map's, and which of them it has seen
 *  fall. `placed` keeps its identity while the map and the
 *  library do; only `fallen` follows knowledge. */
export interface SideBuildings {
  placed: PlacedBuildings;
  fallen: readonly FallenBuilding[];
}

/** A map's buildings, and their parts' props. */
export interface BuildingIndex {
  placed: PlacedBuildings;
  /** Each part's prop, to its building in `placed`. */
  partBuilding: Map<number, number>;
  /** Per building: its parts' props on the map. */
  parts: MapProp[][];
}

/** The map's `buildings` as references, with the map props (`props`) that
 *  are their parts. */
export function indexBuildings(
  buildings: PublicBuildings,
  props: readonly MapProp[],
): BuildingIndex {
  const drawn = buildings.buildings;
  const templates = [...new Set(drawn.map((b) => b.templateId))].sort();
  const templateAt = new Map(templates.map((id, i) => [id, i]));
  const byId = new Map(props.map((p) => [p.id, p]));
  const placed: PlacedBuildings = {
    templates,
    template: new Uint16Array(drawn.length),
    frames: new Float64Array(drawn.length * FRAME_FLOATS),
    owners: new Uint32Array(drawn.length),
  };
  const partBuilding = new Map<number, number>();
  const parts: MapProp[][] = [];
  drawn.forEach((b, i) => {
    placed.template[i] = templateAt.get(b.templateId)!;
    placed.frames.set([...b.frame.translation, b.frame.yaw], i * FRAME_FLOATS);
    placed.owners[i] = b.owner;
    parts.push(
      b.parts.flatMap(({ prop }) => {
        partBuilding.set(prop, i);
        const part = byId.get(prop);
        return part ? [part] : [];
      }),
    );
  });
  return { placed, partBuilding, parts };
}

/**
 * The buildings of `index` a side knows are destroyed, and how, from the props
 * it has learned (`known`). What the simulation published decides the state:
 * a part's place taken by a prop of a type in `shells` (the types a gutted
 * building's parts become, from the catalog's building rows: `buildingRemains`)
 * is a building that stands `gutted`; taken by anything else, or by nothing,
 * it is a `ruin`. A destruction the side has not seen leaves the building
 * intact, however long ago it was.
 *
 * The simulation destroys a building whole, by one rule, and a side that sees
 * any part of it learns every part, so one known part says what the building
 * is. Were a side ever to know parts in both states, the building is a ruin:
 * a shell is drawn only where nothing known says a part came down.
 */
export function fallenBuildings(
  index: BuildingIndex,
  known: readonly KnownProp[],
  shells: { has(kind: string): boolean },
): FallenBuilding[] {
  const states = new Map<number, DamageState>();
  for (const k of known) {
    const building = k.authoredProp === null ? undefined : index.partBuilding.get(k.authoredProp);
    if (building === undefined) continue;
    const stands = !k.destroyed && shells.has(k.kind);
    states.set(building, stands && states.get(building) !== "ruin" ? "gutted" : "ruin");
  }
  return [...states].sort(([a], [b]) => a - b).map(([building, state]) => ({ building, state }));
}
