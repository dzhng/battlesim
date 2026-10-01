// Massing: buildings with no art yet, drawn as their physical parts. Every
// part of a building whose regional family the style lists (the prototype
// template catalogue's) is one plain box at the simulation's own size, tinted
// by the building's category, through the static chunk path (`lod.ts`). It is
// a developer's stand-in for template art, and reads as one: no windows, no
// roofs, one flat colour a category.
//
// What a side draws is what it knows, as for every structure: a part it has
// seen fall is drawn as its remains' box, or not at all when nothing was left.
import { color } from "math/color";
import { pick } from "@packages/renderer-core/src/kindTable";
import type { KnownProp, MapProp } from "../models/propAppearance";
import type { PublicBuildings } from "../worldMesh";
import { createPlacedInstances, INSTANCE_FLOATS, type PlacedInstances } from "./lod";

type Rgb = readonly [number, number, number];

/** `presentation.massing` in the fixture. */
export interface MassingStyle {
  /** The regional families drawn as massing. */
  families: readonly string[];
  /** A building category's wall colour (sRGB), with a `default`. */
  tints: Record<string, Rgb>;
  /** A fallen part's remains (sRGB). */
  ruin: Rgb;
}

export function validateMassingStyle(style: MassingStyle): MassingStyle {
  const rgb = (c: unknown) =>
    Array.isArray(c) && c.length === 3 && c.every((v) => typeof v === "number" && v >= 0 && v <= 1);
  if (!Array.isArray(style.families) || style.families.some((f) => typeof f !== "string"))
    throw new Error("presentation.massing.families: a list of regional family names");
  if (!style.tints?.default) throw new Error("presentation.massing.tints needs a default");
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

const _massing_tint = color.create();

/** The boxes a side draws for the massing `parts`: every map part it has not
 *  seen replaced, and the remains of those it has. Each scales the layer's
 *  unit box (±1 across, 0 to 1 up), so a part's half extents and height are
 *  its scale; all are the one kind. */
export function massingInstances(
  props: readonly MapProp[],
  known: readonly KnownProp[],
  parts: ReadonlyMap<number, string>,
  style: MassingStyle,
): PlacedInstances {
  const replaced = new Map<number, KnownProp>();
  for (const k of known)
    if (k.authoredProp !== null && parts.has(k.authoredProp)) replaced.set(k.authoredProp, k);
  const boxes: { box: MapProp | KnownProp; tint: Rgb }[] = [];
  for (const prop of props) {
    const category = parts.get(prop.id);
    if (category === undefined) continue;
    const remains = replaced.get(prop.id);
    if (!remains) boxes.push({ box: prop, tint: pick(style.tints, category) });
    else if (!remains.destroyed) boxes.push({ box: remains, tint: style.ruin });
  }
  const out = createPlacedInstances(boxes.length);
  boxes.forEach(({ box, tint }, i) => {
    const [hx, hy, hz] = box.half;
    color.setFromSRGB(_massing_tint, [tint[0], tint[1], tint[2]]);
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
