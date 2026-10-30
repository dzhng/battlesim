// Props as the page holds them: the map's prop boxes and what a side knows of
// them, read for what only the page does with them (the camera's obstacles,
// fog's occluders), and what the page reports of the installed art a prop may
// be drawn as. Which appearance draws each prop, fitted to its box, which a
// side draws and at which tier are the presentation's
// (`crates/presentation/src/static_world/props.rs`): the page draws the
// frame's batches.
import { presentationSettings } from "@web/presentationSettings";
import { STAND_IN_KIT, STAND_IN_MODULE } from "@packages/scene-assets/src/standInKit";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { WorldExports, WorldLayout } from "../worldMesh";

/** A prop's box: centred on `center` at ground level `baseZ`, turned by `yaw`. */
export interface PropBox {
  kind: string;
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
  /** A wreck's unit type: it is drawn as that unit's own wreck. Absent or
   *  null for any other prop. */
  wreckOf?: string | null;
}

/** A prop authored in the static map. */
export interface MapProp extends PropBox {
  id: number;
}

/** A prop the side has learned (a wreck, a ruin), and the map prop it stands
 *  in place of, if any. A `destroyed` entry draws nothing: the side saw that
 *  map prop destroyed with nothing in its place. */
export interface KnownProp extends PropBox {
  /** Immutable public-map source across all remains chains; null for dynamic bodies. */
  authoredProp: number | null;
  replaces: number | null;
  destroyed?: boolean;
}

/** The static map's props, in exported order. */
export function mapProps(exports: WorldExports, layout: WorldLayout): MapProp[] {
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const props = exports.props;
  const out: MapProp[] = [];
  for (let r = 0; r < props.length; r += layout.propStride) {
    const wreckOf = props[r + at.wreckOf];
    out.push({
      id: props[r + at.idLo] + props[r + at.idHi] * 2 ** layout.limbBits,
      kind: layout.propKinds[props[r + at.kind]],
      center: [props[r + at.x], props[r + at.y]],
      yaw: props[r + at.yaw],
      half: [props[r + at.hx], props[r + at.hy], props[r + at.hz]],
      baseZ: props[r + at.baseZ],
      wreckOf: wreckOf >= 0 ? layout.unitKinds[wreckOf] : null,
    });
  }
  return out;
}

/**
 * The boxes a side knows stand where the map props `among` holds were placed:
 * each one it has not seen replaced, as authored, and the remains of each it
 * has; nothing for one it saw destroyed with nothing left. A fall the side has
 * not seen leaves the prop standing. The camera keeps clear of a building's
 * parts by these, the same knowledge its drawn state follows
 * (`static_world/fallen.rs`), so what is drawn and what blocks the camera cannot part.
 */
export function knownStanding(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  among: { has(prop: number): boolean },
): PropBox[] {
  const replaced = new Map<number, KnownProp>();
  for (const k of known)
    if (k.authoredProp !== null && among.has(k.authoredProp)) replaced.set(k.authoredProp, k);
  const out: PropBox[] = [];
  for (const prop of props) {
    if (!among.has(prop.id)) continue;
    const remains = replaced.get(prop.id);
    if (!remains) out.push(prop);
    else if (!remains.destroyed) out.push(remains);
  }
  return out;
}

/** What the presentation fits the props from (`PropInput`), as JSON: every
 *  installed static appearance with a footprint (its scenery kind, region,
 *  footprint, paints and its first state's bounds), each vehicle appearance's
 *  own wreck, the stand-in kit's bounds where it is installed with its module,
 *  the map's regional family and the models' tiers. */
export function propArt(
  installed: InstalledAppearances,
  family: string | null,
  dynamic = true,
): string {
  const appearances = [];
  const wrecks: Record<string, string> = {};
  for (const [name, a] of installed.appearances) {
    if (a.wreck) wrecks[name] = a.wreck;
    if (a.bundle.kind !== "static" || !a.footprint) continue;
    appearances.push({
      name,
      scenery: a.scenery,
      family: a.regionalFamily,
      footprint: a.footprint,
      paints: a.paints,
      bounds: a.bundle.states[0].bounds,
    });
  }
  const kit = installed.appearances.get(STAND_IN_KIT)?.bundle;
  const standIn =
    kit?.kind === "static" && kit.states.some((s) => s.name === STAND_IN_MODULE)
      ? kit.states[0].bounds
      : null;
  return JSON.stringify({
    appearances,
    wrecks,
    stand_in: standIn,
    family,
    detail: presentationSettings.models,
    dynamic,
  });
}
