// The Armory's yard: the static ground a unit on show stands on. It is a
// presentation document (`fixtures/armory/yard.json`) that the simulation
// never admits. It names the spots a unit can stand on, which spot each body
// takes, the props that dress the yard, each a catalog prop kind drawn by its
// own scenery art, and the hills the land rises in past it.
import yardJson from "@fixtures/armory/yard.json";
import { BackdropRelief } from "@wasm/renderer_browser";
import type { Rgba } from "@packages/battle-renderer/src/mesh";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { propsDrawnBy } from "@packages/scene-assets/src/scenery";
import type { CatalogView, UnitType } from "@packages/scene-assets/src/units";
import type { WorldLayers, WorldScenery } from "@packages/battle-renderer/src/scene";
import type { BackdropReliefInput } from "@packages/battle-renderer/src/terrain/terrainSurface";
import { worldScenery, worldSceneryNames } from "@packages/battle-renderer/src/worldMesh";
import type { LabModel } from "@web/battle/sim/presentationModels";
import { gameBiome } from "../gameBiome";
import { plainWorld, type GroundPad } from "../plainWorld";

export type SpotId = "apron" | "motor_pool" | "tents";
/** What a unit stands as, for the spot it takes. */
export type Body = "squad" | "aircraft" | "ground";

/** Where a spot's camera looks from at first: the orbit about `target`. */
export interface CameraPose {
  target: [number, number, number];
  distance: number;
  /** Elevation above the ground, radians. */
  pitch: number;
  /** Azimuth about +Z, radians. */
  yaw: number;
}

/** A place a unit stands: its centre, the way it faces (radians
 *  counter-clockwise from +X), the radius kept clear of dressing, and its
 *  camera. */
export interface SpotPlace {
  at: [number, number];
  yaw: number;
  clear_m: number;
  camera: CameraPose;
}

export interface Spot extends SpotPlace {
  id: SpotId;
}

/** A prop that dresses the yard: a catalog prop kind, where it stands and
 *  which way it faces. */
export interface YardProp {
  kind: string;
  at: [number, number];
  yaw: number;
}

export interface YardDocument {
  /** The yard's flat ground: how far it reaches each way, its colour, the
   *  hard standing paved over it (a later pad over an earlier one), and the
   *  relief of the land past it, which keeps clear of it. */
  ground: { half_m: number; tint: Rgba; pads: GroundPad[]; relief: BackdropReliefInput };
  spots: Record<SpotId, SpotPlace>;
  /** The spot each body takes. */
  bodies: Record<Body, SpotId>;
  props: YardProp[];
}

const finite = (v: unknown, n: number) =>
  Array.isArray(v) && v.length === n && v.every((x) => Number.isFinite(x));

/** `raw` as a yard document, or an error naming each field it gets wrong:
 *  its shape and that every body names a spot. The relief's features are
 *  the battle's own, admitted where they are read (`yardGround`, and the
 *  stage's ground). */
export function admitYard(raw: unknown): YardDocument {
  const doc = raw as YardDocument;
  const wrong: string[] = [];
  if (!(doc.ground?.half_m > 0) || !finite(doc.ground.tint, 4) || !Array.isArray(doc.ground.pads))
    wrong.push("ground: a positive half_m, an rgba tint and a list of pads");
  const relief = doc.ground?.relief;
  if (!(relief?.cell_m > 0) || !Array.isArray(relief.features))
    wrong.push("ground.relief: a positive cell_m and a list of features");
  (doc.ground?.pads ?? []).forEach((p, i) => {
    const sides = finite(p.half_m, 2) && p.half_m.every((h) => h > 0);
    if (!finite(p.at, 2) || !sides || !Number.isFinite(p.yaw) || !finite(p.tint, 4))
      wrong.push(`ground.pads[${i}]: an [x, y] at, positive half_m, a yaw and an rgba tint`);
  });
  for (const [id, s] of Object.entries(doc.spots ?? {}))
    if (!finite(s.at, 2) || !(s.clear_m > 0) || !finite(s.camera?.target, 3))
      wrong.push(`spots.${id}: an [x, y] at, a positive clear_m and a camera target`);
  for (const [body, id] of Object.entries(doc.bodies ?? {}))
    if (!doc.spots?.[id]) wrong.push(`bodies.${body}: no spot "${id}"`);
  (doc.props ?? []).forEach((p, i) => {
    if (typeof p.kind !== "string" || !finite(p.at, 2) || !Number.isFinite(p.yaw))
      wrong.push(`props[${i}]: a kind, an [x, y] at and a yaw`);
  });
  if (wrong.length) throw new Error(`fixtures/armory/yard.json: ${wrong.join("; ")}`);
  return doc;
}

/** The yard the Armory stands its units in. */
export const yard = admitYard(yardJson);

let world: WorldLayers | null = null;
/** The yard's flat ground and its pads as a stage's world, built once.
 *  Past it, the scene's presentation draws the battle's backdrop over the
 *  yard's relief: the biome's patchwork and its trees (`yardWorldScenery`). */
export function yardWorld(): WorldLayers {
  const { tint, half_m, pads, relief } = yard.ground;
  world ??= plainWorld(tint, half_m, pads, relief);
  return world;
}

let land: BackdropRelief | null = null;
/** The ground's height at `(x, y)`, metres: level over the yard, and past
 *  it the relief the backdrop draws and stands its trees on, read from the
 *  same surface. The one height the Armory's camera and its checks read;
 *  the presentation module must have loaded. */
export function yardGround(): (x: number, y: number) => number {
  const h = yard.ground.half_m;
  land ??= new BackdropRelief(
    JSON.stringify(yard.ground.relief),
    Float64Array.from([-h, -h, h, h]),
  );
  const relief = land;
  return (x, y) => relief.height(x, y);
}

/** What `unit` stands as: a squad, a hull with a flight model, or any other
 *  hull. */
export function bodyOf(unit: UnitType): Body {
  if ("squad" in unit.body) return "squad";
  return "air" in unit.mobility ? "aircraft" : "ground";
}

/** The spot `unit` stands on; null for none (a planned card). */
export function spotFor(doc: YardDocument, unit: UnitType | null): Spot | null {
  if (!unit) return null;
  const id = doc.bodies[bodyOf(unit)];
  return { id, ...doc.spots[id] };
}

/** Whether scenery kind `scenery` draws prop `kind` (`propsDrawnBy`). */
const draws = (props: CatalogView["props"], scenery: string | null | undefined, kind: string) =>
  scenery != null && propsDrawnBy(props, scenery).includes(kind);

/** The scenery appearances the yard draws, by name: what an art load takes
 *  for the yard. Those that draw its props, and the battle biome's trees,
 *  hedgerows and dressing, which the presentation stands past the yard's
 *  ground (`yardWorldScenery`). Regional looks are left to the loader, which
 *  fetches them only on request. */
export function yardScenery(
  doc: YardDocument,
  props: CatalogView["props"],
  catalog: Pick<RuntimeCatalog, "appearances">,
): Set<string> {
  const kinds = new Set(doc.props.map((p) => p.kind));
  const { trees, dressing } = worldSceneryNames(gameBiome);
  return new Set([
    ...Object.entries(catalog.appearances)
      .filter(([, e]) => e.unit === "scenery" && [...kinds].some((k) => draws(props, e.scenery, k)))
      .map(([name]) => name),
    ...trees,
    ...dressing,
  ]);
}

/** The yard's trees: the battle biome's scenery art, which the scene's
 *  presentation stands past the yard's ground as it stands a battle's
 *  backdrop past the map's edge. */
export function yardWorldScenery(installed: InstalledAppearances): WorldScenery {
  return worldScenery(gameBiome, installed);
}

/** The yard's props as static models, each drawn by the first installed
 *  appearance of no region whose scenery kind draws it, as authored. */
export function yardModels(
  doc: YardDocument,
  props: CatalogView["props"],
  installed: InstalledAppearances,
): LabModel[] {
  const drawers = [...installed.appearances]
    .flatMap(([name, a]) =>
      a.bundle.kind === "static" && a.regionalFamily === null
        ? [{ name, scenery: a.scenery, state: a.bundle.states[0].name }]
        : [],
    )
    .sort((a, b) => (a.name < b.name ? -1 : 1));
  return doc.props.map((p) => {
    const drawer = drawers.find((d) => draws(props, d.scenery, p.kind));
    if (!drawer)
      throw new Error(
        `yard prop ${p.kind}: no installed appearance draws it; load the yard's scenery (yardScenery)`,
      );
    return {
      appearance: drawer.name,
      x: p.at[0],
      y: p.at[1],
      z: 0,
      yaw: p.yaw,
      pose: { kind: "static", state: drawer.state },
    };
  });
}
