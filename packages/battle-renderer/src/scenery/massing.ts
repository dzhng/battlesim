// Massing: buildings with no art yet, drawn as their physical parts. Every
// part of a building whose regional family the style lists (the prototype
// template catalogue's) is one plain box at the simulation's own size, tinted
// by the building's category, through the static chunk path (`lod.ts`). It is
// a developer's stand-in for template art, and reads as one: no windows, no
// roofs, one flat colour a category.
//
// A map prop that nothing else draws (street furniture whose art is not
// fitted yet, a tree no forest stood) is a box the same way, tinted by its
// kind: a body the simulation holds is never invisible.
//
// What a side draws is what it knows, as for every structure: a part it has
// seen fall is drawn as its remains' box, or not at all when nothing was left.
import { color } from "math/color";
import { mulberry32, random } from "math/random";
import { pick } from "@packages/renderer-core/src/kindTable";
import { knownStanding, type KnownProp, type MapProp } from "../models/propAppearance";
import type { PublicBuildings } from "../worldMesh";
import { createPlacedInstances, INSTANCE_FLOATS, type PlacedInstances } from "./lod";

type Rgb = readonly [number, number, number];

/** `presentation.massing` in the fixture. */
export interface MassingStyle {
  /** The regional families drawn as massing. */
  families: readonly string[];
  /** A building category's wall colour, or an artless prop kind's (sRGB),
   *  with a `default`. */
  tints: Record<string, Rgb>;
  /** Each box's value strays this far from its tint, either way, so
   *  neighbours of one category part at their shared walls. */
  tint_jitter: number;
  /** A fallen part's remains (sRGB). */
  ruin: Rgb;
}

export function validateMassingStyle(style: MassingStyle): MassingStyle {
  const rgb = (c: unknown) =>
    Array.isArray(c) && c.length === 3 && c.every((v) => typeof v === "number" && v >= 0 && v <= 1);
  if (!Array.isArray(style.families) || style.families.some((f) => typeof f !== "string"))
    throw new Error("presentation.massing.families: a list of regional family names");
  if (!style.tints?.default) throw new Error("presentation.massing.tints needs a default");
  if (!(style.tint_jitter >= 0 && style.tint_jitter <= 0.5))
    throw new Error("presentation.massing.tint_jitter must be within [0, 0.5]");
  for (const [name, tint] of Object.entries({ ...style.tints, ruin: style.ruin }))
    if (!rgb(tint)) throw new Error(`presentation.massing: ${name} must be [r, g, b] in [0, 1]`);
  return style;
}

/** Each massing part's prop id and its building's category. */
export function massingParts(buildings: PublicBuildings, style: MassingStyle): Map<number, string> {
  const out = new Map<number, string>();
  for (const building of buildings.buildings) {
    if (!style.families.includes(building.regionalFamily)) continue;
    for (const part of building.parts) out.set(part.prop, building.category);
  }
  return out;
}

/** Each map prop no appearance or forest draws (`drawn` says which are),
 *  and its kind: what its box is tinted by. */
export function artlessProps(
  props: readonly MapProp[],
  drawn: (prop: MapProp) => boolean,
): Map<number, string> {
  const out = new Map<number, string>();
  for (const prop of props) if (!drawn(prop)) out.set(prop.id, prop.kind);
  return out;
}

const _massing_tint = color.create();

/** The boxes a side draws for the massing `parts`: every map part it has not
 *  seen replaced, and the remains of those it has. A part the side has only
 *  seen moved is still itself, where it was last seen. Each scales the
 *  layer's unit box (±1 across, 0 to 1 up), so a part's half extents and
 *  height are its scale; all are the one kind. */
export function massingInstances(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  parts: ReadonlyMap<number, string>,
  style: MassingStyle,
): PlacedInstances {
  const boxes = knownStanding(props, known, parts);
  const out = createPlacedInstances(boxes.length);
  boxes.forEach(({ prop, box, fallen }, i) => {
    const remains = fallen && box.kind !== prop.kind;
    const tint: Rgb = remains ? style.ruin : pick(style.tints, parts.get(prop.id)!);
    const [hx, hy, hz] = box.half;
    // The part's own value, from its id: the same standing, fallen and next battle.
    const seeded = mulberry32.create(prop.id);
    const value = 1 + random.float(() => mulberry32.sample(seeded), -1, 1) * style.tint_jitter;
    color.setFromSRGB(_massing_tint, [tint[0], tint[1], tint[2]]);
    color.multiplyScalar(_massing_tint, _massing_tint, value);
    out.heights[i] = 2 * hz;
    out.reaches[i] = Math.hypot(hx, hy);
    out.records.set(
      [
        box.center[0],
        box.center[1],
        box.baseZ,
        box.yaw,
        hx,
        hy,
        2 * hz,
        hz,
        _massing_tint[0],
        _massing_tint[1],
        _massing_tint[2],
        0,
      ],
      i * INSTANCE_FLOATS,
    );
  });
  return out;
}
