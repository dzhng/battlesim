// Props as appearances: which installed scenery bundle draws each prop the
// simulation places, fitted to its box. The simulation's box is the authority
// (sight, rounds and movement meet it); the appearance is authored to a
// declared box (`footprint_half_m`) and is fitted to each placed box here:
//
// - a prop kind is drawn by the appearances its catalog `appearance` binds
//   (the world layout's `propAppearance`), never by a list here;
// - an appearance is chosen per prop kind by the footprint nearest the box
//   (a tank's wreck against a truck's);
// - it is scaled per axis from its footprint to the box, except a module
//   (a wall, a fence, a sandbag line) that is repeated along the
//   box's long side instead of stretched.
//
// A building's parts are not drawn here: a building is its template's rows,
// standing or fallen (`buildingReferences.ts`).
//
// What a side draws is what it knows (`structureModels`): the map's props,
// less those a known prop replaces, plus every known prop. A replacement is
// atomic: the list that drops a wall carries its rubble, and a fall the side
// has not seen leaves the wall standing.

import type { Vec3 } from "math";
import { color } from "math/color";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { PROTOTYPE_KIT, PROTOTYPE_MODULE } from "@packages/scene-assets/src/prototypeSet";
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

interface Candidate {
  name: string;
  footprint: Vec3;
  bundle: StaticBundle;
}

/** The installed prop appearances, indexed by the simulation prop kind:
 *  each kind is drawn by the appearances of the scenery kind its catalog
 *  `appearance` names (`drawn_by`), as the world layout carries it.
 *
 *  A kind none is fitted to (street furniture before its models) takes the
 *  stand-in, where `standIns` gives one: the prototype kit's unit box,
 *  stretched to the prop's own box and tinted by its kind, through the same
 *  model instances. A body the simulation holds is then never invisible. A
 *  forest's trees are the scenery's, never a stand-in. */
export class PropAppearances {
  private readonly byKind = new Map<string, Candidate[]>();
  private readonly bindings: WorldLayout["propAppearance"];
  /** The kinds whose body stops no mover class: surfaces movers stand on. */
  private readonly walkedOn: Set<string>;
  /** Each stand-in tint, linear; null where no stand-in is drawn. */
  private readonly standIns: Map<string, Vec3> | null;

  constructor(
    installed: InstalledAppearances,
    layout: Pick<WorldLayout, "propAppearance" | "blockingPropKinds">,
    standIns?: StandInStyle,
  ) {
    const kit = installed.appearances.get(PROTOTYPE_KIT)?.bundle;
    const box = kit?.kind === "static" && kit.states.some((s) => s.name === PROTOTYPE_MODULE);
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
    for (const [name, entry] of installed.appearances) {
      if (entry.bundle.kind !== "static" || !entry.footprint) continue;
      const kinds = Object.keys(this.bindings).filter(
        (k) => this.bindings[k].drawn_by === entry.scenery,
      );
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
    const chosen = this.choose(box.kind, box.half);
    if (!chosen && this.standsIn(box.kind)) {
      const [hx, hy, hz] = box.half;
      out.push({
        ...placed(PROTOTYPE_KIT, box, 0, [2 * hx, 2 * hy, 2 * hz], 0, false, PROTOTYPE_MODULE),
        tint: this.standIns!.get(box.kind) ?? this.standIns!.get("default")!,
      });
      return out;
    }
    if (!chosen) return out;
    const [fx, fy, fz] = chosen.footprint;
    const [hx, hy, hz] = box.half;
    const ground = this.walkedOn.has(box.kind);
    if (!this.bindings[box.kind]?.modular) {
      out.push(placed(chosen.name, box, 0, [hx / fx, hy / fy, hz / fz], 0, ground));
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
      const chosen = this.choose(prop.kind, prop.half);
      if (chosen) out.add(chosen.name);
      else if (this.standsIn(prop.kind)) out.add(PROTOTYPE_KIT);
    }
    for (const [kind, list] of this.byKind)
      if (!this.bindings[kind]?.map_only) for (const c of list) out.add(c.name);
    // A battle can leave a body with no art anywhere (a burnt-out car).
    for (const kind of Object.keys(this.bindings))
      if (!this.bindings[kind].map_only && this.standsIn(kind)) out.add(PROTOTYPE_KIT);
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

/** The static map's props, in exported order. */
export function mapProps(exports: WorldExports, layout: WorldLayout): MapProp[] {
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const props = exports.props;
  const out: MapProp[] = [];
  for (let r = 0; r < props.length; r += layout.propStride)
    out.push({
      id: props[r + at.idLo] + props[r + at.idHi] * 2 ** layout.limbBits,
      kind: layout.propKinds[props[r + at.kind]],
      center: [props[r + at.x], props[r + at.y]],
      yaw: props[r + at.yaw],
      half: [props[r + at.hx], props[r + at.hy], props[r + at.hz]],
      baseZ: props[r + at.baseZ],
    });
  return out;
}

/** A map prop as a side knows it: standing as authored, or the remains the
 *  side has seen take its place. */
export interface KnownStanding {
  prop: MapProp;
  box: PropBox;
  fallen: boolean;
}

/**
 * What a side knows stands of the map props `among` holds: each one it has not
 * seen replaced, as authored, and the remains of each it has; nothing for one
 * it saw destroyed with nothing left. A fall the side has not seen leaves the
 * prop standing. Buildings are drawn from these and the camera keeps clear of them, so
 * what is drawn and what blocks the camera cannot part.
 */
export function knownStanding(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  among: { has(prop: number): boolean },
): KnownStanding[] {
  const replaced = new Map<number, KnownProp>();
  for (const k of known)
    if (k.authoredProp !== null && among.has(k.authoredProp)) replaced.set(k.authoredProp, k);
  const out: KnownStanding[] = [];
  for (const prop of props) {
    if (!among.has(prop.id)) continue;
    const remains = replaced.get(prop.id);
    if (!remains) out.push({ prop, box: prop, fallen: false });
    else if (!remains.destroyed) out.push({ prop, box: remains, fallen: true });
  }
  return out;
}

/**
 * What a side draws of the props: every map prop `keep` accepts, less those a
 * known prop replaces, plus every known prop. Known props arrive only with
 * knowledge, so a fall the side has not seen leaves the map's prop standing.
 */
export function structureModels(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  appearances: PropAppearances,
  keep: (prop: MapProp) => boolean = () => true,
): ModelInstance[] {
  const replaced = new Map<number, KnownProp>();
  for (const k of known) if (k.authoredProp !== null) replaced.set(k.authoredProp, k);
  const out: ModelInstance[] = [];
  for (const prop of props) if (keep(prop) && !replaced.has(prop.id)) appearances.fit(prop, out);
  for (const k of known) if (!k.destroyed) appearances.fit(k, out);
  return out;
}
