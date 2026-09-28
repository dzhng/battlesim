// ContactGlyph: uncertain evidence for one side, drawn over fog in display
// space. Every contact is an area red through its middle: a soft red fill,
// a red hatch and a red glow at the rim. A last sighting is a ghost, its fill
// fainter and ringed by a crisp pale outline; a firing report keeps its even
// red fill to the rim. Both fade toward expiry and are gone at it.
//
// A glyph is built from an approximate contact's own fields only (area,
// source, age): no class, exact position, heading or motion, so it can show
// nothing the side does not know. The hatch is anchored to the world, not to
// the contact, and has no direction of its own. The obstacles the side has
// learned are world geometry (`models/propAppearance.ts`).
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

/** `presentation.contacts`: how a contact glyph is drawn. */
export interface ContactGlyphStyle {
  /** Distance between hatch lines, metres, anchored to the world. */
  hatch_spacing_m: number;
  hatch_width_m: number;
  /** The hatch's heading, degrees counter-clockwise from world +X. */
  hatch_angle_deg: number;
  /** A last sighting's outline, metres wide inside its rim. */
  outline_width_m: number;
  /** A last sighting's outline (rgb): pale, so its edge reads over fog,
   *  grass and roofs alike. */
  outline_color: [number, number, number];
  /** A last sighting's hatch (rgb): a lighter red than the glow, so the
   *  lines read against the red fill under them. */
  ghost_hatch_color: [number, number, number];
  /** The glow and the fills, and a firing report's hatch (rgb). */
  glow_color: [number, number, number];
  hatch_alpha: number;
  /** The glow at the rim, and a firing report's fill. */
  glow_alpha: number;
  /** A last sighting's red fill across its disc, inside the outline. */
  ghost_fill_alpha: number;
  /** Metres above the ground. */
  lift_m: number;
}

export function validateContactGlyphStyle(s: ContactGlyphStyle): ContactGlyphStyle {
  const positive = (name: string, v: number) => {
    if (!(Number.isFinite(v) && v > 0))
      throw new Error(`presentation.contacts.${name} must be > 0, got ${v}`);
  };
  const unit = (name: string, v: number) => {
    if (!(v >= 0 && v <= 1)) throw new Error(`presentation.contacts.${name} must be in [0, 1]`);
  };
  positive("hatch_spacing_m", s.hatch_spacing_m);
  positive("hatch_width_m", s.hatch_width_m);
  positive("outline_width_m", s.outline_width_m);
  if (s.hatch_width_m >= s.hatch_spacing_m)
    throw new Error("presentation.contacts.hatch_width_m must be less than hatch_spacing_m");
  if (!Number.isFinite(s.hatch_angle_deg))
    throw new Error("presentation.contacts.hatch_angle_deg must be finite");
  unit("hatch_alpha", s.hatch_alpha);
  unit("glow_alpha", s.glow_alpha);
  unit("ghost_fill_alpha", s.ghost_fill_alpha);
  for (const key of ["outline_color", "ghost_hatch_color", "glow_color"] as const)
    s[key].forEach((c, i) => unit(`${key}[${i}]`, c));
  if (!(s.lift_m >= 0)) throw new Error("presentation.contacts.lift_m must be ≥ 0");
  return s;
}

/** What a glyph is drawn from: an approximate contact's area, source and age. */
export interface ContactShape {
  center: readonly [number, number];
  radius: number;
  /** Remaining life in (0, 1]; at 0 (expiry) the glyph is gone. */
  freshness: number;
  source: string;
}

/** A contact's remaining life at `tick`: 1 when fresh, 0 at expiry. */
export function contactFreshness(
  c: { evidenceTick: number; expiresTick: number },
  tick: number,
): number {
  return Math.max(
    0,
    Math.min(1, (c.expiresTick - tick) / Math.max(1, c.expiresTick - c.evidenceTick)),
  );
}

/** Quads per turn: the glow's soft bands, and the ghost's crisp outline. */
const SEGMENTS = 48;
const OUTLINE_SEGMENTS = 128;
/** Hatch lines are draped on the ground in pieces at most this long. */
const DRAPE_M = 12;
/** A firing report's even fill ends here, as a fraction of its radius; its
 *  rim fades out beyond (an area, with no centre to aim at). */
const FILL_EDGE = 0.85;
/** A last sighting's glow hugs its outline: from here inward it fades out,
 *  and it spills this far outside the rim. */
const GLOW_INNER = 0.72;
const GLOW_SPILL = 1.08;

const rgba = (c: readonly number[], a: number): Rgba => [c[0], c[1], c[2], a];

/** Hatch lines across the disc out to `reach`, anchored to the world's lines
 *  of the hatch's heading, each draped on the ground. */
function hatch(
  mesh: MeshBuilder,
  c: ContactShape,
  reach: number,
  s: ContactGlyphStyle,
  color: Rgba,
  z: SurfaceHeight,
) {
  const a = (s.hatch_angle_deg * Math.PI) / 180;
  const along = [Math.cos(a), Math.sin(a)];
  const across = [-along[1], along[0]];
  const half = s.hatch_width_m / 2;
  const centre = c.center[0] * across[0] + c.center[1] * across[1];
  const first = Math.ceil((centre - reach) / s.hatch_spacing_m);
  const last = Math.floor((centre + reach) / s.hatch_spacing_m);
  const at = (x: number, y: number): [number, number, number] => [x, y, z(x, y) + s.lift_m];
  for (let k = first; k <= last; k++) {
    const off = k * s.hatch_spacing_m - centre;
    const chord = reach * reach - (Math.abs(off) + half) ** 2;
    if (chord <= 0) continue;
    const h = Math.sqrt(chord);
    const pieces = Math.max(1, Math.ceil((2 * h) / DRAPE_M));
    const ox = c.center[0] + across[0] * off;
    const oy = c.center[1] + across[1] * off;
    for (let i = 0; i < pieces; i++) {
      const t0 = -h + (2 * h * i) / pieces;
      const t1 = -h + (2 * h * (i + 1)) / pieces;
      const x0 = ox + along[0] * t0,
        y0 = oy + along[1] * t0;
      const x1 = ox + along[0] * t1,
        y1 = oy + along[1] * t1;
      const wx = across[0] * half,
        wy = across[1] * half;
      mesh.quad(
        at(x0 - wx, y0 - wy),
        at(x1 - wx, y1 - wy),
        at(x1 + wx, y1 + wy),
        at(x0 + wx, y0 + wy),
        color,
      );
    }
  }
}

function glyph(mesh: MeshBuilder, c: ContactShape, s: ContactGlyphStyle, z: SurfaceHeight) {
  if (!(c.freshness > 0) || !(c.radius > 0)) return;
  const life = 0.35 + 0.65 * Math.min(1, c.freshness);
  const glow = s.glow_alpha * life;
  const band = (
    inner: number,
    outer: number,
    color: readonly number[],
    a0: number,
    a1: number,
    segments = SEGMENTS,
  ) =>
    groundAnnulus(mesh, c.center, inner, outer, {
      z,
      lift: s.lift_m,
      segments,
      colorIn: rgba(color, a0),
      colorOut: rgba(color, a1),
    });
  if (c.source === "last_seen") {
    // The hatch and fill stop where the outline starts, so none overlap.
    const inner = Math.max(0, c.radius - s.outline_width_m);
    const fill = s.ghost_fill_alpha * life;
    band(0, inner, s.glow_color, fill, fill);
    band(c.radius * GLOW_INNER, c.radius, s.glow_color, 0, glow);
    band(c.radius, c.radius * GLOW_SPILL, s.glow_color, glow, 0);
    const ghost = s.hatch_alpha * life;
    hatch(mesh, c, inner, s, rgba(s.ghost_hatch_color, ghost), z);
    band(inner, c.radius, s.outline_color, ghost, ghost, OUTLINE_SEGMENTS);
    return;
  }
  band(0, c.radius * FILL_EDGE, s.glow_color, glow, glow);
  band(c.radius * FILL_EDGE, c.radius, s.glow_color, glow, 0);
  hatch(mesh, c, c.radius, s, rgba(s.glow_color, s.hatch_alpha * life), z);
}

export function buildContactGlyphs(
  contacts: readonly ContactShape[],
  z: SurfaceHeight,
  style: ContactGlyphStyle,
): WorldMeshes {
  const translucent = new MeshBuilder();
  for (const c of contacts) glyph(translucent, c, style, z);
  return { opaque: new Float32Array(0), translucent: translucent.build() };
}
