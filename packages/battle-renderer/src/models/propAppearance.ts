// Props as appearances: which installed static bundle, in which state, draws
// each prop the simulation places, fitted to its box. The simulation's box is
// the authority (sight, rounds and movement meet it); the appearance is
// authored to a declared box (`footprint_half_m`, slice 22) and is fitted to
// each placed box here:
//
// - an appearance is chosen per prop kind by the footprint nearest the box
//   (a tank's wreck against a truck's, one house plan against another);
// - it is scaled per axis from its footprint to the box, except a module
//   (a wall) that is repeated along the box's long side instead of stretched;
// - a building's ruin is its own building's appearance in the "ruin" state,
//   authored to the ruin rule's height, so only its plan is fitted.
//
// What a side draws is what it knows (`structureModels`): the map's props,
// less those a known prop replaces, plus every known prop. A replacement is
// atomic: the list that drops a building carries its ruin, and a collapse the
// side has not seen leaves the building standing.

import type { Vec3 } from "math";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { StaticBundle } from "@packages/scene-assets/src/schema";
import type { WorldExports, WorldLayout } from "../worldMesh";
import type { ModelInstance } from "./modelInstances";

/** A prop's box: centred on `center` at ground level `baseZ`, turned by `yaw`. */
export interface PropBox {
  kind: string;
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
}

/** A prop authored in the static map. */
export interface MapProp extends PropBox {
  id: number;
}

/** A prop the side has learned (a wreck, a ruin), and the map prop it stands
 *  in place of, if any. */
export interface KnownProp extends PropBox {
  replaces: number | null;
}

/** Which appearances draw a simulation prop kind: buildings by their unit,
 *  the rest by their scenery kind. Trunks are the forest's trees. */
const SCENERY_OF: Record<string, string> = {
  wall: "wall",
  crate: "crate",
  bridgedeck: "bridge_deck",
  wreck: "wreck",
  ruin: "ruin",
};
/** Kinds drawn by repeating their module along the box's long side. */
const MODULAR = new Set(["wall"]);
/** The state a building stands in, and the one its ruin takes. */
export const INTACT = "intact";
export const RUIN = "ruin";
/** The state every other prop appearance draws. */
const DEFAULT_STATE = "default";

interface Candidate {
  name: string;
  footprint: Vec3;
  bundle: StaticBundle;
}

/** The installed prop appearances, indexed by the simulation prop kind. */
export class PropAppearances {
  private readonly byKind = new Map<string, Candidate[]>();

  constructor(installed: InstalledAppearances) {
    for (const [name, entry] of installed.appearances) {
      if (entry.bundle.kind !== "static" || !entry.footprint) continue;
      const kinds =
        entry.unit === "building"
          ? ["building"]
          : Object.keys(SCENERY_OF).filter((k) => SCENERY_OF[k] === entry.scenery);
      for (const kind of kinds) {
        const list = this.byKind.get(kind) ?? [];
        list.push({ name, footprint: entry.footprint, bundle: entry.bundle });
        list.sort((a, b) => (a.name < b.name ? -1 : 1));
        this.byKind.set(kind, list);
      }
    }
  }

  /** The appearance whose footprint is nearest `box` (least total log scale), or null. */
  choose(kind: string, half: PropBox["half"]): Candidate | null {
    let best: Candidate | null = null;
    let bestMisfit = Infinity;
    for (const c of this.byKind.get(kind) ?? []) {
      let misfit = 0;
      for (let a = 0; a < 3; a++) misfit += Math.abs(Math.log(half[a] / c.footprint[a]));
      if (misfit < bestMisfit) {
        bestMisfit = misfit;
        best = c;
      }
    }
    return best;
  }

  /** The models drawing `box` as its kind (in `state`, when the appearance
   *  has it), appended to `out`. A kind with no appearance draws nothing. */
  fit(box: PropBox, out: ModelInstance[], state?: string): ModelInstance[] {
    const chosen = this.choose(box.kind, box.half);
    if (!chosen) return out;
    const name = state ?? (box.kind === "building" ? INTACT : DEFAULT_STATE);
    if (!chosen.bundle.states.some((s) => s.name === name)) return out;
    const [fx, fy, fz] = chosen.footprint;
    const [hx, hy, hz] = box.half;
    if (!MODULAR.has(box.kind)) {
      out.push(placed(chosen.name, name, box, 0, [hx / fx, hy / fy, hz / fz], 0));
      return out;
    }
    // A module runs along its own long axis; turn it to the box's long side.
    const turn = hx >= hy === fx >= fy ? 0 : Math.PI / 2;
    const long = turn === 0 ? hx : hy;
    const across = turn === 0 ? hy : hx;
    const count = Math.max(1, Math.round(long / fx));
    const step = (2 * long) / count;
    const scale: Vec3 = [long / (count * fx), across / fy, hz / fz];
    for (let k = 0; k < count; k++) {
      const along = -long + step * (k + 0.5);
      out.push(placed(chosen.name, name, box, along, scale, turn));
    }
    return out;
  }

  /** Every appearance drawing `props` could need: the one each map prop
   *  takes (a building's ruin is its own appearance), and every wreck and
   *  ruin, since a battle can leave either anywhere. */
  drawnFor(props: readonly PropBox[]): Set<string> {
    const out = new Set<string>();
    for (const prop of props) {
      const chosen = this.choose(prop.kind, prop.half);
      if (chosen) out.add(chosen.name);
    }
    for (const kind of ["wreck", "ruin"])
      for (const c of this.byKind.get(kind) ?? []) out.add(c.name);
    return out;
  }

  /** A building's ruin: that building's own appearance in its ruin state, on
   *  the building's plan, at the height it was authored to (the ruin rule). */
  ruinOf(building: PropBox, ruin: PropBox, out: ModelInstance[]): ModelInstance[] {
    const chosen = this.choose("building", building.half);
    if (!chosen || !chosen.bundle.states.some((s) => s.name === RUIN)) return this.fit(ruin, out);
    const [fx, fy] = chosen.footprint;
    out.push(placed(chosen.name, RUIN, ruin, 0, [ruin.half[0] / fx, ruin.half[1] / fy, 1], 0));
    return out;
  }
}

/** A static model on `box`, shifted `along` its local +X after turning by `turn`. */
function placed(
  appearance: string,
  state: string,
  box: PropBox,
  along: number,
  scale: Vec3,
  turn: number,
): ModelInstance {
  const yaw = box.yaw + turn;
  return {
    appearance,
    x: box.center[0] + along * Math.cos(yaw),
    y: box.center[1] + along * Math.sin(yaw),
    z: box.baseZ,
    yaw,
    scale,
    pose: { kind: "static", state },
  };
}

/** The static map's props, in exported order. */
export function mapProps(exports: WorldExports, layout: WorldLayout): MapProp[] {
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const props = exports.props;
  const out: MapProp[] = [];
  for (let r = 0; r < props.length; r += layout.propStride)
    out.push({
      id: props[r + at.id],
      kind: layout.propKinds[props[r + at.kind]],
      center: [props[r + at.x], props[r + at.y]],
      yaw: props[r + at.yaw],
      half: [props[r + at.hx], props[r + at.hy], props[r + at.hz]],
      baseZ: props[r + at.baseZ],
    });
  return out;
}

/**
 * What a side draws of the props: every map prop `keep` accepts, less those a
 * known prop replaces, plus every known prop — a building's ruin as that
 * building's own appearance. Known props arrive only with knowledge, so a
 * collapse the side has not seen leaves its building standing.
 */
export function structureModels(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  appearances: PropAppearances,
  keep: (prop: MapProp) => boolean = () => true,
): ModelInstance[] {
  const replaced = new Map<number, KnownProp>();
  for (const k of known) if (k.replaces !== null) replaced.set(k.replaces, k);
  const out: ModelInstance[] = [];
  const byId = new Map<number, MapProp>();
  for (const prop of props) {
    byId.set(prop.id, prop);
    if (keep(prop) && !replaced.has(prop.id)) appearances.fit(prop, out);
  }
  for (const k of known) {
    const building = k.replaces === null ? undefined : byId.get(k.replaces);
    if (k.kind === "ruin" && building?.kind === "building") appearances.ruinOf(building, k, out);
    else appearances.fit(k, out);
  }
  return out;
}
