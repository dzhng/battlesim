// A map's buildings as the frame draws them from template art: compact
// references for the whole map (which template, where it was placed, whose
// it is), which building each part prop is, and the template art they are
// drawn from.
// Nothing here holds a module transform: Rust resolves and walks them
// (`crates/presentation`'s static world), and `buildingLayer.ts` draws what
// it chose.
//
// Every building of every map is drawn this way, from its template's rows in
// the installed template art library: a generated town's, a test map's
// houses, a lab's one box. Nothing else draws a building, and a template the
// library lacks is refused by name when the scene is built.
//
// Knowledge is the rule every structure follows: a building stands intact
// until the side has seen it destroyed, and is then drawn in the state the
// simulation published of it, never one worked out here. The presentation
// reads that from the side's known props (`static_world/fallen.rs`).
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { templateKits, type TemplateArtLibrary } from "@packages/scene-assets/src/templateLibrary";
import type { KnownProp, MapProp } from "./propAppearance";
import type { PublicBuildings } from "../worldMesh";

/** `presentation.buildings` in the fixture. */
export interface BuildingStyle {
  /** The detail tiers, by how many device pixels a metre of wall covers:
   *  above `[0]` tier 0, above `[1]` tier 1, above `[2]` tier 2, else tier 3.
   *  A window is the same size on a house and on a tower, so the tier follows
   *  distance, not a building's height. */
  lod_px_per_m: [number, number, number];
  /** Square chunks buildings are bucketed in, metres: a chunk takes one tier. */
  chunk_m: number;
  /** Module instances the residency pool holds at most (`static_world/walk.rs`). */
  pool_records: number;
  /** Each building's tint strays this far in value, either way, so identical
   *  neighbours part. */
  tint_jitter: number;
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

/** The buildings a scene's presentation draws: the map's, which building
 *  each part prop is, and the prop kinds a gutted building's parts become
 *  (the catalog's building rows: `buildingRemains`), which the side's known
 *  props are read against. A lab no battle publishes into hands its own
 *  stand-in for those props (`known`); in a battle the publication's decide. */
export interface SceneBuildings {
  placed: PlacedBuildings;
  partBuilding: ReadonlyMap<number, number>;
  shells: ReadonlySet<string>;
  known?: readonly KnownPart[];
}

/** What the rule reads of a known prop: the map prop whose place it took,
 *  its kind, and whether it was destroyed with nothing left. */
export type KnownPart = Pick<KnownProp, "authoredProp" | "kind" | "destroyed">;

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

/** The template art a scene draws from. */
export interface BuildingArt {
  library: TemplateArtLibrary;
  /** The library's file, which Rust resolves buildings from. */
  bytes: Uint8Array;
  /** Per library module: its mesh's bounds in its own frame, or null for a
   *  module of a kit that is not installed (a map installs the kits its
   *  templates' rows place: `templateKits`). */
  bounds: readonly (Bounds | null)[];
}

/** The template art `installed` holds, each module's bounds from its
 *  installed kit's bundle; null without a library. */
export function buildingArt(installed: InstalledAppearances | null): BuildingArt | null {
  const templates = installed?.templates;
  if (!templates) return null;
  const bounds = templates.modules.map(({ kit, state }) => {
    const bundle = state === null ? null : installed.appearances.get(kit)?.bundle;
    return bundle?.kind === "static" ? (bundle.states[state!]?.bounds ?? null) : null;
  });
  return { library: templates.library, bytes: templates.bytes, bounds };
}

/** Whether every kit `placed` needs is installed in `art`. The buildings and
 *  the kits that draw them arrive apart; a scene is built once both have. */
export function artCovers(placed: PlacedBuildings, art: BuildingArt): boolean {
  const { kits, modules } = art.library;
  const needed = templateKits(art.library, placed.templates);
  return modules.every((m, i) => art.bounds[i] !== null || !needed.has(kits[m.kit].appearance));
}
