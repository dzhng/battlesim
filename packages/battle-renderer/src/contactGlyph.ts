// Uncertain evidence for one side: every report has the same enemy-coloured
// fill, hatch and glow, with a pale outline. The area and opacity come from
// the shared contact presentation; no hidden position or unit type is read.
import { EMPTY_MESH, groundAnnulus, groundStrip, MeshBuilder, rgbA, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

/** `presentation.contacts`: how a contact glyph is drawn. */
export interface ContactGlyphStyle {
  /** Seconds of visual fading after a report is removed. */
  fade_s: number;
  /** Distance between hatch lines, metres, anchored to the world. */
  hatch_spacing_m: number;
  hatch_width_m: number;
  /** The hatch's heading, degrees counter-clockwise from world +X. */
  hatch_angle_deg: number;
  /** Every report's outline, metres wide inside its rim. */
  outline_width_m: number;
  /** Every report's outline (rgb): pale, so its edge reads over fog,
   *  grass and roofs alike. */
  outline_color: [number, number, number];
  /** The glow, the fills and the hatch (rgb): the HUD's enemy colour. */
  color: readonly [number, number, number];
  hatch_alpha: number;
  /** The glow at the rim. */
  glow_alpha: number;
  /** Every report's red fill across its disc, inside the outline. */
  fill_alpha: number;
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
  positive("fade_s", s.fade_s);
  positive("hatch_spacing_m", s.hatch_spacing_m);
  positive("hatch_width_m", s.hatch_width_m);
  positive("outline_width_m", s.outline_width_m);
  if (s.hatch_width_m >= s.hatch_spacing_m)
    throw new Error("presentation.contacts.hatch_width_m must be less than hatch_spacing_m");
  if (!Number.isFinite(s.hatch_angle_deg))
    throw new Error("presentation.contacts.hatch_angle_deg must be finite");
  unit("hatch_alpha", s.hatch_alpha);
  unit("glow_alpha", s.glow_alpha);
  unit("fill_alpha", s.fill_alpha);
  for (const key of ["outline_color", "color"] as const)
    s[key].forEach((c, i) => unit(`${key}[${i}]`, c));
  if (!(s.lift_m >= 0)) throw new Error("presentation.contacts.lift_m must be ≥ 0");
  return s;
}

/** What a glyph is drawn from: an approximate contact's area and presentation opacity. */
export interface ContactShape {
  center: readonly [number, number];
  radius: number;
  /** Opacity in (0, 1]; at 0 the glyph is gone. */
  opacity: number;
}

/** Quads per turn: the glow's soft bands, and the crisp outline. */
const SEGMENTS = 48;
const OUTLINE_SEGMENTS = 128;
/** Hatch lines are draped on the ground in pieces at most this long. */
const DRAPE_M = 12;
/** A report's glow hugs its outline: from here inward it fades out,
 *  and it spills this far outside the rim. */
const GLOW_INNER = 0.72;
const GLOW_SPILL = 1.08;

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
  for (let k = first; k <= last; k++) {
    const off = k * s.hatch_spacing_m - centre;
    const chord = reach * reach - (Math.abs(off) + half) ** 2;
    if (chord <= 0) continue;
    const h = Math.sqrt(chord);
    const [ox, oy] = [c.center[0] + across[0] * off, c.center[1] + across[1] * off];
    groundStrip(
      mesh,
      [ox - along[0] * h, oy - along[1] * h],
      [ox + along[0] * h, oy + along[1] * h],
      s.hatch_width_m,
      color,
      { z, lift: s.lift_m, step: DRAPE_M },
    );
  }
}

function glyph(mesh: MeshBuilder, c: ContactShape, s: ContactGlyphStyle, z: SurfaceHeight) {
  if (!(c.opacity > 0) || !(c.radius > 0)) return;
  const life = Math.min(1, c.opacity);
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
      colorIn: rgbA(color, a0),
      colorOut: rgbA(color, a1),
    });
  // The hatch and fill stop where the outline starts, so none overlap.
  const inner = Math.max(0, c.radius - s.outline_width_m);
  const fill = s.fill_alpha * life;
  band(0, inner, s.color, fill, fill);
  band(c.radius * GLOW_INNER, c.radius, s.color, 0, glow);
  band(c.radius, c.radius * GLOW_SPILL, s.color, glow, 0);
  const ink = s.hatch_alpha * life;
  hatch(mesh, c, inner, s, rgbA(s.color, ink), z);
  band(inner, c.radius, s.outline_color, ink, ink, OUTLINE_SEGMENTS);
}

export function buildContactGlyphs(
  contacts: readonly ContactShape[],
  z: SurfaceHeight,
  style: ContactGlyphStyle,
): WorldMeshes {
  const translucent = new MeshBuilder();
  for (const c of contacts) glyph(translucent, c, style, z);
  return { opaque: EMPTY_MESH, translucent: translucent.build() };
}
