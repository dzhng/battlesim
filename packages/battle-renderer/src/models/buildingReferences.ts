// A map's buildings as the frame draws them from template art: compact
// references for the whole map (which template, where it was placed, whose
// it is), and what a side knows fell. Nothing here holds a module transform;
// `buildingPlacements.ts` expands the references near the camera.
//
// A building is drawn from its template's rows when the installed template
// art library has that template. One it does not have (the village's houses,
// a lab's fixture) is not a reference here, and keeps the fitted-appearance
// path (`propAppearance.ts`).
//
// Knowledge is the rule every structure follows (`knownStanding`): a building
// stands intact until the side has seen it fall.
import type { TemplateState } from "@packages/scene-assets/src/templateLibrary";
import { knownStanding, type KnownProp, type MapProp, type PropBox } from "./propAppearance";
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
  /** A fallen part's remains, where its template has no ruin art (sRGB). */
  ruin_tint: Rgb;
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
  for (const [name, tint] of Object.entries({
    ...style.prototype_tints,
    ruin_tint: style.ruin_tint,
  }))
    if (!rgb(tint)) throw new Error(`${at}: ${name} must be [r, g, b] in [0, 1]`);
  return style;
}

/** Floats per building frame: x, y, z, then yaw (radians about +Z). */
export const FRAME_FLOATS = 4;

/** The buildings of a map that template art draws. */
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

/** A building a side knows is no longer intact. */
export interface FallenBuilding {
  /** Which of `PlacedBuildings`. */
  building: number;
  /** The state its template is drawn in, where the library has rows for it. */
  state: Exclude<TemplateState, "intact">;
  /** Each of its parts as the side knows it: the remains it saw take the
   *  part's place, or the part as authored if it has not seen that one go.
   *  Without art for `state`, each is drawn as a box. */
  parts: readonly PropBox[];
}

/** The buildings a side draws from template art: the map's, and which of
 *  them it has seen fall. `placed` keeps its identity while the map and the
 *  library do; only `fallen` follows knowledge. */
export interface SideBuildings {
  placed: PlacedBuildings;
  fallen: readonly FallenBuilding[];
}

/** A map's art-drawn buildings, and their parts' props. */
export interface BuildingIndex {
  placed: PlacedBuildings;
  /** Each part's prop, to its building in `placed`. */
  partBuilding: Map<number, number>;
  /** Per building: its parts' props on the map. */
  parts: MapProp[][];
}

/** The buildings of `buildings` whose template `hasArt` accepts, as
 *  references, with the map props (`props`) that are their parts. */
export function indexBuildings(
  buildings: PublicBuildings,
  props: readonly MapProp[],
  hasArt: (templateId: string) => boolean,
): BuildingIndex {
  const drawn = buildings.buildings.filter((b) => hasArt(b.templateId));
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
 * The buildings of `index` a side knows have fallen, from the props it has
 * learned (`known`): every building with a part it has seen replaced or
 * destroyed, each part as it knows it. A fall the side has not seen leaves
 * the building intact.
 */
export function fallenBuildings(
  index: BuildingIndex,
  known: readonly KnownProp[],
): FallenBuilding[] {
  const touched = new Set<number>();
  for (const k of known) {
    const building = k.authoredProp === null ? undefined : index.partBuilding.get(k.authoredProp);
    if (building !== undefined) touched.add(building);
  }
  return [...touched]
    .sort((a, b) => a - b)
    .map((building) => {
      const parts = index.parts[building];
      const own = new Set(parts.map((p) => p.id));
      return {
        building,
        state: "ruin" as const,
        parts: knownStanding(parts, known, own).map((standing) => standing.box),
      };
    });
}
