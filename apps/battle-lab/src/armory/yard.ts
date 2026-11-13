// The Armory's yard: the static ground a unit on show stands on. It is a
// presentation document (`fixtures/armory/yard.json`) that the simulation
// never admits. It names the spots a unit can stand on, which spot each body
// takes, and the props that dress the yard, each a catalog prop kind drawn
// by its own scenery art.
import yardJson from "@fixtures/armory/yard.json";
import type { Rgba } from "@packages/battle-renderer/src/mesh";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import { propsDrawnBy } from "@packages/scene-assets/src/scenery";
import type { CatalogView, UnitType } from "@packages/scene-assets/src/units";
import type { LabModel } from "@web/battle/sim/presentationModels";

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
  /** The yard's flat ground: how far it reaches each way, and its colour. */
  ground: { half_m: number; tint: Rgba };
  spots: Record<SpotId, SpotPlace>;
  /** The spot each body takes. */
  bodies: Record<Body, SpotId>;
  props: YardProp[];
}

const finite = (v: unknown, n: number) =>
  Array.isArray(v) && v.length === n && v.every((x) => Number.isFinite(x));

/** `raw` as a yard document, or an error naming each field it gets wrong:
 *  its shape and that every body names a spot. */
export function admitYard(raw: unknown): YardDocument {
  const doc = raw as YardDocument;
  const wrong: string[] = [];
  if (!(doc.ground?.half_m > 0) || !finite(doc.ground.tint, 4))
    wrong.push("ground: a positive half_m and an rgba tint");
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

/** The scenery appearances that draw the yard's props, by name: what an art
 *  load takes for the yard. Regional looks are left to the loader, which
 *  fetches them only on request. */
export function yardScenery(
  doc: YardDocument,
  props: CatalogView["props"],
  catalog: Pick<RuntimeCatalog, "appearances">,
): Set<string> {
  const kinds = new Set(doc.props.map((p) => p.kind));
  return new Set(
    Object.entries(catalog.appearances)
      .filter(([, e]) => e.unit === "scenery" && [...kinds].some((k) => draws(props, e.scenery, k)))
      .map(([name]) => name),
  );
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
