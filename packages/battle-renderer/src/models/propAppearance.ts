// Props as appearances: which installed scenery bundle draws each prop the
// simulation places, fitted to its box. The simulation's box is the authority
// (sight, rounds and movement meet it); the appearance is authored to a
// declared box (`footprint_half_m`) and is fitted to each placed box here:
//
// - a prop kind is drawn by the appearances its catalog `appearance` binds
//   (the world layout's `propAppearance`), never by a list here;
// - a kind's appearances are those of no region and those of the map's own
//   region (an appearance's `regional_family`), never another region's; where
//   the map's region has looks of its own for the kind, only those;
// - an appearance is chosen among them by the footprint nearest the box (a
//   long wall module against a short one); a wreck is the one exception: it
//   is its own unit's wreck (`wreckOf`, the vehicle appearance's `wreck`),
//   whatever box it lies on, so you can tell which unit died by looking;
// - it is scaled per axis from its footprint to the box, except a module
//   (a wall, a fence, a sandbag line) that is repeated along the
//   box's long side instead of stretched;
// - where the appearance has paints (a car's), each body wears one, chosen
//   by the authored prop it is, so a shoved car keeps its colour.
//
// A building's parts are not drawn here: a building is its template's rows,
// intact, a ruin or a gutted shell (`buildingReferences.ts`).
//
// What a side draws is what it knows (`structureBodies`): the map's props,
// less those a known prop replaces, plus every known prop. A replacement is
// atomic: the list that drops a wall carries its rubble, and a fall the side
// has not seen leaves the wall standing. The map's props are fitted once
// (`fitMapProps`); what the side learns hides the ones it replaced and fits
// only its own (`sideStructures`), so a change costs what it touched.

import type { Vec3 } from "math";
import { color } from "math/color";
import { mulberry32 } from "math/random";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { STAND_IN_KIT, STAND_IN_MODULE } from "@packages/scene-assets/src/standInKit";
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

/** What draws a prop kind: a prop type's `appearance` in the catalog. */
export interface PropAppearance {
  /** The appearances fitted to its box: those of this scenery kind. Or
   *  `building`, a part its building draws from its template's art, and
   *  `forest`, the trees a forest draws itself. */
  drawn_by: string;
  /** Drawn by repeating one module along the box's long side. */
  modular?: boolean;
  /** Only a map places one; a battle never leaves or drops one. */
  map_only?: boolean;
}

/** Whether `kind` is drawn by `by` (`forest`, `building`, a scenery kind). */
export function drawnBy(
  layout: Pick<WorldLayout, "propAppearance">,
  kind: string,
  by: string,
): boolean {
  return layout.propAppearance[kind]?.drawn_by === by;
}

/** `presentation.stand_ins` in the fixture: what a prop kind with no art
 *  fitted is drawn as until it has some. */
export interface StandInStyle {
  /** A prop kind's colour (sRGB), with a `default`. */
  tints: Record<string, readonly [number, number, number]>;
}

export function validateStandIns(style: StandInStyle): StandInStyle {
  const rgb = (c: unknown) =>
    Array.isArray(c) && c.length === 3 && c.every((v) => typeof v === "number" && v >= 0 && v <= 1);
  if (!style.tints?.default) throw new Error("presentation.stand_ins.tints needs a default");
  for (const [kind, tint] of Object.entries(style.tints))
    if (!rgb(tint))
      throw new Error(`presentation.stand_ins.tints: ${kind} must be [r, g, b] in [0, 1]`);
  return style;
}

/** The state a prop appearance draws. */
const STATE = "default";
/** The scenery kind that draws a unit's wreck: each unit its own. */
const WRECK = "wreck";

interface Candidate {
  name: string;
  footprint: Vec3;
  bundle: StaticBundle;
  /** The tints its paintable surfaces take, linear; null where it has none. */
  paints: Vec3[] | null;
}

/** The installed prop appearances, indexed by the simulation prop kind:
 *  each kind is drawn by the appearances of the scenery kind its catalog
 *  `appearance` names (`drawn_by`), as the world layout carries it.
 *
 *  A kind none is fitted to (a body the simulation places before its model
 *  is made) takes the stand-in, where `standIns` gives one: the stand-in
 *  kit's unit box, stretched to the prop's own box and tinted by its kind,
 *  through the same model instances. A body the simulation holds is then never invisible. A
 *  forest's trees are the scenery's, never a stand-in.
 *
 *  `family` is the map's regional family, null for a map of none: a kind
 *  draws its region's own looks where it has any, else the looks of no
 *  region. */
export class PropAppearances {
  private readonly byKind = new Map<string, Candidate[]>();
  /** Each unit type's own wreck, by unit type: what its vehicle appearance
   *  names (`wreck`), where that is installed. */
  private readonly wrecks = new Map<string, Candidate>();
  private readonly bindings: WorldLayout["propAppearance"];
  /** The kinds whose body stops no mover class: surfaces movers stand on. */
  private readonly walkedOn: Set<string>;
  /** Each stand-in tint, linear; null where no stand-in is drawn. */
  private readonly standIns: Map<string, Vec3> | null;

  constructor(
    installed: InstalledAppearances,
    layout: Pick<WorldLayout, "propAppearance" | "blockingPropKinds" | "unitAppearance">,
    family: string | null,
    standIns?: StandInStyle,
  ) {
    const kit = installed.appearances.get(STAND_IN_KIT)?.bundle;
    const box = kit?.kind === "static" && kit.states.some((s) => s.name === STAND_IN_MODULE);
    this.standIns =
      standIns && box
        ? new Map(
            Object.entries(standIns.tints).map(([kind, tint]) => [
              kind,
              [...color.fromSRGB([tint[0], tint[1], tint[2]])] as Vec3,
            ]),
          )
        : null;
    this.bindings = layout.propAppearance;
    const blocking = new Set(Object.values(layout.blockingPropKinds).flat());
    this.walkedOn = new Set(Object.keys(this.bindings).filter((k) => !blocking.has(k)));
    const candidate = (name: string): Candidate | null => {
      const entry = installed.appearances.get(name);
      if (entry?.bundle.kind !== "static" || !entry.footprint) return null;
      return {
        name,
        footprint: entry.footprint,
        bundle: entry.bundle,
        paints: entry.paints?.map((p) => [...color.fromSRGB(p)] as Vec3) ?? null,
      };
    };
    for (const [unit, model] of Object.entries(layout.unitAppearance)) {
      const wreck = installed.appearances.get(model)?.wreck;
      const drawn = wreck ? candidate(wreck) : null;
      if (drawn) this.wrecks.set(unit, drawn);
    }
    const own = new Map<string, Candidate[]>();
    for (const [name, entry] of installed.appearances) {
      if (entry.scenery === WRECK) continue;
      const drawn = candidate(name);
      if (!drawn) continue;
      if (entry.regionalFamily !== null && entry.regionalFamily !== family) continue;
      const lists = entry.regionalFamily === null ? this.byKind : own;
      const kinds = Object.keys(this.bindings).filter(
        (k) => this.bindings[k].drawn_by === entry.scenery,
      );
      for (const kind of kinds) {
        const list = lists.get(kind) ?? [];
        list.push(drawn);
        list.sort((a, b) => (a.name < b.name ? -1 : 1));
        lists.set(kind, list);
      }
    }
    for (const [kind, list] of own) this.byKind.set(kind, list);
  }

  /** The appearance drawing `box`: a wreck its unit's own, anything else the
   *  one whose footprint is nearest the box (least total log scale); or null. */
  choose(box: Pick<PropBox, "kind" | "half" | "wreckOf">): Candidate | null {
    const { kind, half } = box;
    if (this.bindings[kind]?.drawn_by === WRECK) return this.wrecks.get(box.wreckOf ?? "") ?? null;
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

  /** Whether `kind` is drawn as the stand-in box: it has a binding, no
   *  appearance is fitted to it, and something else does not draw it (a
   *  forest its trees, a building its parts, from its template's art). */
  private standsIn(kind: string): boolean {
    return (
      this.standIns !== null &&
      kind in this.bindings &&
      !this.drawsTree(kind) &&
      this.bindings[kind].drawn_by !== "building" &&
      !this.byKind.has(kind)
    );
  }

  /** The models drawing `box` as its kind, appended to `out`. A kind with
   *  no appearance draws its stand-in, or nothing where there is none. */
  fit(box: PropBox, out: ModelInstance[]): ModelInstance[] {
    const chosen = this.choose(box);
    if (!chosen && this.standsIn(box.kind)) {
      const [hx, hy, hz] = box.half;
      out.push({
        ...placed(STAND_IN_KIT, box, 0, [2 * hx, 2 * hy, 2 * hz], 0, false, STAND_IN_MODULE),
        tint: this.standIns!.get(box.kind) ?? this.standIns!.get("default")!,
      });
      return out;
    }
    if (!chosen) return out;
    const [fx, fy, fz] = chosen.footprint;
    const [hx, hy, hz] = box.half;
    const ground = this.walkedOn.has(box.kind);
    if (!this.bindings[box.kind]?.modular) {
      const model = placed(chosen.name, box, 0, [hx / fx, hy / fy, hz / fz], 0, ground);
      out.push(chosen.paints ? { ...model, tint: paintOf(chosen.paints, box) } : model);
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
      out.push(placed(chosen.name, box, along, scale, turn, ground));
    }
    return out;
  }

  /** Whether `kind` is a forest's tree: the scenery draws it, not a model. */
  drawsTree(kind: string): boolean {
    return this.bindings[kind]?.drawn_by === "forest";
  }

  /** Every appearance drawing `props` could need: the one each map prop
   *  takes, and every appearance of a kind not `map_only`, since a battle
   *  can leave or place those anywhere (a wreck, rubble, a dropped crate, a
   *  sandbag line). */
  drawnFor(props: readonly PropBox[]): Set<string> {
    const out = new Set<string>();
    for (const prop of props) {
      const chosen = this.choose(prop);
      if (chosen) out.add(chosen.name);
      else if (this.standsIn(prop.kind)) out.add(STAND_IN_KIT);
    }
    for (const [kind, list] of this.byKind)
      if (!this.bindings[kind]?.map_only) for (const c of list) out.add(c.name);
    // Any vehicle can die anywhere: every unit's own wreck.
    for (const c of this.wrecks.values()) out.add(c.name);
    // A battle can leave a body with no art anywhere (a burnt-out car).
    for (const kind of Object.keys(this.bindings))
      if (!this.bindings[kind].map_only && this.standsIn(kind)) out.add(STAND_IN_KIT);
    return out;
  }
}

/** A static model on `box`, shifted `along` its local +X after turning by
 *  `turn`, in `state` (a kit's module, for the stand-in box). */
function placed(
  appearance: string,
  box: PropBox,
  along: number,
  scale: Vec3,
  turn: number,
  ground: boolean,
  state = STATE,
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

/** The paint `box` wears of `paints`: drawn from the authored prop it is (a
 *  map prop's id, or the one a known body stands for), else from where it
 *  stands, so the same car is the same colour in every battle. */
function paintOf(paints: readonly Vec3[], box: PropBox | MapProp | KnownProp): Vec3 {
  const authored = "id" in box ? box.id : "authoredProp" in box ? box.authoredProp : null;
  const seed =
    authored ?? Math.round(box.center[0] * 10) * 73856093 + Math.round(box.center[1] * 10);
  return paints[Math.floor(mulberry32.sample(mulberry32.create(seed)) * paints.length)];
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
 * (`fallenBuildings`), so what is drawn and what blocks the camera cannot part.
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

/** A body a side draws, and the models drawing it: several for a repeated
 *  module, none for a kind with no art and no stand-in (or a building's
 *  part, which its building draws). */
export interface DrawnBody {
  body: PropBox;
  models: ModelInstance[];
}

/**
 * What a side draws of the props, body by body: every map prop `keep`
 * accepts, less those a known prop replaces, plus every known prop. Known
 * props arrive only with knowledge, so a fall the side has not seen leaves
 * the map's prop standing.
 */
export function structureBodies(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  appearances: PropAppearances,
  keep: (prop: MapProp) => boolean = () => true,
): DrawnBody[] {
  const side = sideStructures(fitMapProps(props, appearances, keep), known, appearances);
  return side.bodies();
}

/** The map props a side may draw, each fitted once: their bodies and all
 *  their models in map order, and each prop's models' range in that list. */
export interface FittedMapProps {
  bodies: readonly DrawnBody[];
  models: readonly ModelInstance[];
  /** By map prop id, its models' `[first, end)` in `models`. */
  ranges: ReadonlyMap<number, readonly [number, number]>;
}

/** Fit every map prop `keep` accepts, once for the battle. */
export function fitMapProps(
  props: readonly MapProp[],
  appearances: PropAppearances,
  keep: (prop: MapProp) => boolean = () => true,
): FittedMapProps {
  const bodies: DrawnBody[] = [];
  const models: ModelInstance[] = [];
  const ranges = new Map<number, readonly [number, number]>();
  for (const prop of props) {
    if (!keep(prop)) continue;
    const first = models.length;
    appearances.fit(prop, models);
    bodies.push({ body: prop, models: models.slice(first) });
    ranges.set(prop.id, [first, models.length]);
  }
  return { bodies, models, ranges };
}

/**
 * What a side draws of the props (`structureBodies`) as the map's fitted
 * models less `hidden`, then `known`: drawn in that order it is the same list,
 * but a change in what the side knows refits only its known props and hides
 * only the map props they replace. `map` keeps its identity across changes.
 */
export interface SideStructures {
  /** Every fitted map prop's models (`FittedMapProps.models`). */
  map: readonly ModelInstance[];
  /** Indices into `map` of the models of the props a known prop replaces, ascending. */
  hidden: readonly number[];
  /** The known props' models, after the map's. */
  known: readonly ModelInstance[];
}

/** A side that draws no props apart. */
export const NO_STRUCTURES: SideStructures = { map: [], hidden: [], known: [] };

/** `structures` as one list, in drawing order. */
export function drawnStructures(structures: SideStructures): ModelInstance[] {
  const out: ModelInstance[] = [];
  let next = 0;
  structures.map.forEach((model, i) => {
    if (structures.hidden[next] === i) next++;
    else out.push(model);
  });
  return out.concat(structures.known);
}

/** What a side knowing `known` draws of `fitted`: the props the known
 *  replace hidden, the known that stand fitted after them. `bodies` lists
 *  what is drawn body by body. */
export function sideStructures(
  fitted: FittedMapProps,
  known: readonly KnownProp[],
  appearances: PropAppearances,
): SideStructures & { bodies(): DrawnBody[] } {
  const replaced = new Set<number>();
  const hidden: number[] = [];
  for (const k of known) {
    if (k.authoredProp === null || replaced.has(k.authoredProp)) continue;
    replaced.add(k.authoredProp);
    const range = fitted.ranges.get(k.authoredProp);
    if (range) for (let i = range[0]; i < range[1]; i++) hidden.push(i);
  }
  hidden.sort((a, b) => a - b);
  const knownBodies: DrawnBody[] = known
    .filter((k) => !k.destroyed)
    .map((body) => ({ body, models: appearances.fit(body, []) }));
  return {
    map: fitted.models,
    hidden,
    known: knownBodies.flatMap((b) => b.models),
    bodies: () => [
      ...fitted.bodies.filter((b) => !replaced.has((b.body as MapProp).id)),
      ...knownBodies,
    ],
  };
}
