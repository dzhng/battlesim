// Props as appearances: which installed static bundle, in which state, draws
// each prop the simulation places, fitted to its box. The simulation's box is
// the authority (sight, rounds and movement meet it); the appearance is
// authored to a declared box (`footprint_half_m`) and is fitted to
// each placed box here:
//
// - a prop kind is drawn by the appearances its catalog `appearance` binds
//   (the world layout's `propAppearance`), never by a list here;
// - an appearance is chosen per prop kind by the footprint nearest the box
//   (a tank's wreck against a truck's, one house plan against another);
// - it is scaled per axis from its footprint to the box, except a module
//   (a wall, a fence, a sandbag line) that is repeated along the
//   box's long side instead of stretched;
// - remains with a `remains_state` (a building's ruin) are the replaced
//   body's own appearance in that state, authored to the ruin rule's
//   height, so only its plan is fitted.
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
 *  in place of, if any. A `destroyed` entry draws nothing: the side saw that
 *  map prop destroyed with nothing in its place. */
export interface KnownProp extends PropBox {
  replaces: number | null;
  destroyed?: boolean;
}

/** What draws a prop kind: a prop type's `appearance` in the catalog. */
export interface PropAppearance {
  /** The appearances fitted to its box: those of this scenery kind, or
   *  `building` for the building appearances; `forest` for the trees a
   *  forest draws itself. */
  drawn_by: string;
  /** Drawn by repeating one module along the box's long side. */
  modular?: boolean;
  /** Only a map places one; a battle never leaves or drops one. */
  map_only?: boolean;
  /** Remains drawn as the body they stand in place of, in this state. */
  remains_state?: string;
}

/** Whether `kind` is drawn by `drawnBy` (`forest`, `building`, a scenery kind). */
export function drawnBy(
  layout: Pick<WorldLayout, "propAppearance">,
  kind: string,
  by: string,
): boolean {
  return layout.propAppearance[kind]?.drawn_by === by;
}

/** The state a building appearance stands in. */
export const INTACT = "intact";
/** The state every other prop appearance draws. */
const DEFAULT_STATE = "default";

interface Candidate {
  name: string;
  footprint: Vec3;
  bundle: StaticBundle;
}

/** The installed prop appearances, indexed by the simulation prop kind:
 *  each kind is drawn by the appearances its catalog `appearance` names
 *  (`drawn_by`: `building` for the building appearances, else a scenery
 *  kind), as the world layout carries it. */
export class PropAppearances {
  private readonly byKind = new Map<string, Candidate[]>();
  private readonly bindings: WorldLayout["propAppearance"];
  /** The kinds whose body stops no mover class: surfaces movers stand on. */
  private readonly walkedOn: Set<string>;

  constructor(
    installed: InstalledAppearances,
    layout: Pick<WorldLayout, "propAppearance" | "blockingPropKinds">,
  ) {
    this.bindings = layout.propAppearance;
    const blocking = new Set(Object.values(layout.blockingPropKinds).flat());
    this.walkedOn = new Set(Object.keys(this.bindings).filter((k) => !blocking.has(k)));
    for (const [name, entry] of installed.appearances) {
      if (entry.bundle.kind !== "static" || !entry.footprint) continue;
      const by = entry.unit === "building" ? "building" : entry.scenery;
      const kinds = Object.keys(this.bindings).filter((k) => this.bindings[k].drawn_by === by);
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
    const name =
      state ?? (this.bindings[box.kind]?.drawn_by === "building" ? INTACT : DEFAULT_STATE);
    if (!chosen.bundle.states.some((s) => s.name === name)) return out;
    const [fx, fy, fz] = chosen.footprint;
    const [hx, hy, hz] = box.half;
    const ground = this.walkedOn.has(box.kind);
    if (!this.bindings[box.kind]?.modular) {
      out.push(placed(chosen.name, name, box, 0, [hx / fx, hy / fy, hz / fz], 0, ground));
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
      out.push(placed(chosen.name, name, box, along, scale, turn, ground));
    }
    return out;
  }

  /** Whether `kind` is a forest's tree: the scenery draws it, not a model. */
  drawsTree(kind: string): boolean {
    return this.bindings[kind]?.drawn_by === "forest";
  }

  /** Every appearance drawing `props` could need: the one each map prop
   *  takes (a building's ruin is its own appearance), and every appearance of
   *  a kind not `map_only`, since a battle can leave or place those anywhere
   *  (a wreck, a ruin, a dropped crate, a sandbag line). */
  drawnFor(props: readonly PropBox[]): Set<string> {
    const out = new Set<string>();
    for (const prop of props) {
      const chosen = this.choose(prop.kind, prop.half);
      if (chosen) out.add(chosen.name);
    }
    for (const [kind, list] of this.byKind)
      if (!this.bindings[kind]?.map_only) for (const c of list) out.add(c.name);
    return out;
  }

  /** Remains drawn as the body they replace (`remains_state`, a building's
   *  ruin): that body's own appearance in that state, on its plan, at the
   *  height it was authored to (the ruin rule). Else the remains' own. */
  remainsOf(replaced: PropBox, remains: PropBox, out: ModelInstance[]): ModelInstance[] {
    const state = this.bindings[remains.kind]?.remains_state;
    const chosen = state ? this.choose(replaced.kind, replaced.half) : null;
    if (!state || !chosen || !chosen.bundle.states.some((s) => s.name === state))
      return this.fit(remains, out);
    const [fx, fy] = chosen.footprint;
    out.push(
      placed(
        chosen.name,
        state,
        remains,
        0,
        [remains.half[0] / fx, remains.half[1] / fy, 1],
        0,
        this.walkedOn.has(remains.kind),
      ),
    );
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
  ground: boolean,
): ModelInstance {
  const yaw = box.yaw + turn;
  return {
    appearance,
    x: box.center[0] + along * Math.cos(yaw),
    y: box.center[1] + along * Math.sin(yaw),
    z: box.baseZ,
    yaw,
    scale,
    pose: ground ? { kind: "static", state, ground } : { kind: "static", state },
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
    if (k.destroyed) continue;
    const replaced = k.replaces === null ? undefined : byId.get(k.replaces);
    if (replaced) appearances.remainsOf(replaced, k, out);
    else appearances.fit(k, out);
  }
  return out;
}
