// Uncertain evidence for one side: every report has the same enemy-coloured
// fill, hatch and glow, with a pale outline. The area and opacity come from
// the shared contact presentation; no hidden position or unit type is read.
// A report lies on the ground, draped over it, unless it is aloft (a
// helicopter lost or heard well over the ground, D11): then the same recipe
// stands at the report's height as a sign facing the camera, stronger, with
// a dark keyline outside its rim so it holds over fog and roofs alike, and a
// thin stem down to the ground under it. One glyph, two ways to stand.
import { EMPTY_MESH, groundAnnulus, groundStrip, MeshBuilder, rgbA, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";
import type { StrokeWidth } from "./strokeWidth";

/** `presentation.contacts.air`: how an aloft report's sign differs. */
export interface AirSignStyle {
  fill_alpha: number;
  hatch_alpha: number;
  glow_alpha: number;
  /** Each hatch line, a stroke this many pixels wide. */
  hatch_px: number;
  /** The pale rim, a stroke this many pixels wide. */
  rim_px: number;
  /** The dark edge just outside the pale rim: a stroke this many pixels wide. */
  keyline_px: number;
  keyline_alpha: number;
  /** The sign spans the report's area, held between these radii on screen
   *  (sized by the marks' stroke rule, so smaller as the camera pulls out):
   *  never a speck far out, never hiding the ground it marks close in. */
  min_radius_px: number;
  max_radius_px: number;
  /** The stem from the ground under the sign up to its rim: a stroke this
   *  many pixels wide, in the rim's colour at this alpha. */
  stem_px: number;
  stem_alpha: number;
}

/** `presentation.contacts`: how a contact glyph is drawn. */
export interface ContactGlyphStyle {
  /** Seconds of visual fading after a report is removed. */
  fade_s: number;
  /** Distance between hatch lines, metres, anchored to the world. */
  hatch_spacing_m: number;
  hatch_width_m: number;
  /** The hatch's heading, degrees counter-clockwise from world +X (on an
   *  aloft sign, from the screen's right). */
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
  air: AirSignStyle;
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
  if (!s.air) throw new Error("presentation.contacts.air is required");
  for (const key of [
    "fill_alpha",
    "hatch_alpha",
    "glow_alpha",
    "keyline_alpha",
    "stem_alpha",
  ] as const)
    unit(`air.${key}`, s.air[key]);
  for (const key of [
    "hatch_px",
    "rim_px",
    "keyline_px",
    "min_radius_px",
    "max_radius_px",
    "stem_px",
  ] as const)
    positive(`air.${key}`, s.air[key]);
  if (s.air.max_radius_px < s.air.min_radius_px)
    throw new Error("presentation.contacts.air.max_radius_px must be ≥ min_radius_px");
  return s;
}

/** What a glyph is drawn from: an approximate contact's area, presentation
 *  opacity and whether it hangs in the air. A report not aloft lies on the
 *  ground at the centre's XY, whatever its height. */
export interface ContactShape {
  center: readonly [number, number, ...number[]];
  radius: number;
  /** Opacity in (0, 1]; at 0 the glyph is gone. */
  opacity: number;
  /** Hangs at the centre's height (`ContactView.aloft`). */
  aloft: boolean;
}

/** How an aloft sign faces the camera and is sized on screen: the screen's
 *  right and up in the world (`screenAxes`), and the marks' stroke rule at
 *  the camera's zoom. */
export interface ContactFacing {
  right: readonly [number, number, number];
  up: readonly [number, number, number];
  stroke: StrokeWidth;
}

type P2 = readonly [number, number];
type P3 = readonly [number, number, number];

/** Quads per turn: the glow's soft bands, and the crisp outline. */
const SEGMENTS = 48;
const OUTLINE_SEGMENTS = 128;
/** Hatch lines are draped on the ground in pieces at most this long. */
const DRAPE_M = 12;
/** A report's glow hugs its outline: from here inward it fades out,
 *  and it spills this far outside the rim. */
const GLOW_INNER = 0.72;
const GLOW_SPILL = 1.08;
/** An aloft sign's keyline is black: an edge of contrast, not a hue. */
const KEYLINE = [0, 0, 0] as const;
/** An aloft sign is lit as the ground glyph is, so both read as one red. */
const UP: P3 = [0, 0, 1];

/** Where a glyph's parts are drawn: bands round its centre and strips
 *  across it, both given in the glyph's own plane as offsets from its centre. */
interface GlyphPen {
  band(inner: number, outer: number, colorIn: Rgba, colorOut: Rgba, segments: number): void;
  strip(a: P2, b: P2, width: number, color: Rgba): void;
}

/** On the ground, draped over it a little lifted. */
function groundPen(mesh: MeshBuilder, at: P2, z: SurfaceHeight, lift: number): GlyphPen {
  return {
    band: (inner, outer, colorIn, colorOut, segments) =>
      groundAnnulus(mesh, at, inner, outer, { z, lift, segments, colorIn, colorOut }),
    strip: (a, b, width, color) =>
      groundStrip(mesh, [at[0] + a[0], at[1] + a[1]], [at[0] + b[0], at[1] + b[1]], width, color, {
        z,
        lift,
        step: DRAPE_M,
      }),
  };
}

/** In the plane facing the camera through `at`. */
function facingPen(mesh: MeshBuilder, at: P3, f: ContactFacing): GlyphPen {
  const place = (u: number, v: number): P3 => [
    at[0] + f.right[0] * u + f.up[0] * v,
    at[1] + f.right[1] * u + f.up[1] * v,
    at[2] + f.right[2] * u + f.up[2] * v,
  ];
  const quad = (p: readonly P3[], c: readonly Rgba[]) => {
    for (const i of [0, 1, 2, 0, 2, 3]) mesh.vertex(p[i], UP, c[i]);
  };
  return {
    band: (inner, outer, colorIn, colorOut, segments) => {
      const ring = (a: number, r: number) => place(Math.cos(a) * r, Math.sin(a) * r);
      for (let k = 0; k < segments; k++) {
        const [a0, a1] = [(k / segments) * Math.PI * 2, ((k + 1) / segments) * Math.PI * 2];
        quad(
          [ring(a0, inner), ring(a1, inner), ring(a1, outer), ring(a0, outer)],
          [colorIn, colorIn, colorOut, colorOut],
        );
      }
    },
    strip: (a, b, width, color) => {
      const length = Math.hypot(b[0] - a[0], b[1] - a[1]);
      if (length < 1e-6) return;
      const nu = (-(b[1] - a[1]) / length) * (width / 2),
        nv = ((b[0] - a[0]) / length) * (width / 2);
      quad(
        [
          place(a[0] - nu, a[1] - nv),
          place(b[0] - nu, b[1] - nv),
          place(b[0] + nu, b[1] + nv),
          place(a[0] + nu, a[1] + nv),
        ],
        [color, color, color, color],
      );
    },
  };
}

/** One glyph's numbers: its radius, its outline's width, the scale its
 *  hatch's spacing is drawn at and its lines' width (at most half the
 *  spacing), its alphas, the hatch's phase across its lines, and the aloft
 *  sign's keyline (none on the ground). Widths are in metres. */
interface Recipe {
  radius: number;
  outline: number;
  scale: number;
  hatchWidth: number;
  fill: number;
  hatch: number;
  glow: number;
  phase: number;
  keyline: { width: number; alpha: number } | null;
}

function draw(pen: GlyphPen, life: number, r: Recipe, s: ContactGlyphStyle) {
  const { radius } = r;
  const glow = r.glow * life;
  const red = (a: number) => rgbA(s.color, a);
  // The hatch and fill stop where the outline starts, so none overlap.
  const inner = Math.max(0, radius - r.outline);
  const fill = r.fill * life;
  pen.band(0, inner, red(fill), red(fill), SEGMENTS);
  pen.band(radius * GLOW_INNER, radius, red(0), red(glow), SEGMENTS);
  // The glow spills outside the rim, or outside the keyline when there is one.
  const edge = radius + (r.keyline?.width ?? 0);
  pen.band(edge, edge + radius * (GLOW_SPILL - 1), red(glow), red(0), SEGMENTS);
  hatch(pen, inner, r, s, red(r.hatch * life));
  const pale = rgbA(s.outline_color, life);
  pen.band(inner, radius, pale, pale, OUTLINE_SEGMENTS);
  if (r.keyline) {
    const dark = rgbA(KEYLINE, r.keyline.alpha * life);
    pen.band(radius, edge, dark, dark, OUTLINE_SEGMENTS);
  }
}

/** Hatch lines across the disc out to `reach`, at the hatch's heading, at
 *  multiples of the spacing from the phase's origin. */
function hatch(pen: GlyphPen, reach: number, r: Recipe, s: ContactGlyphStyle, color: Rgba) {
  const a = (s.hatch_angle_deg * Math.PI) / 180;
  const along = [Math.cos(a), Math.sin(a)];
  const across = [-along[1], along[0]];
  const spacing = s.hatch_spacing_m * r.scale;
  const width = Math.min(r.hatchWidth, spacing / 2);
  const first = Math.ceil((r.phase - reach) / spacing);
  const last = Math.floor((r.phase + reach) / spacing);
  for (let k = first; k <= last; k++) {
    const off = k * spacing - r.phase;
    const chord = reach * reach - (Math.abs(off) + width / 2) ** 2;
    if (chord <= 0) continue;
    const h = Math.sqrt(chord);
    const [ox, oy] = [across[0] * off, across[1] * off];
    pen.strip(
      [ox - along[0] * h, oy - along[1] * h],
      [ox + along[0] * h, oy + along[1] * h],
      width,
      color,
    );
  }
}

/** An aloft report's sign: the recipe facing the camera at the report's
 *  height, and its stem. It spans the report's area, held between its
 *  radii on screen; its hatch's spacing scales with it, and its hatch lines,
 *  rim and keyline are strokes. */
function aloft(
  mesh: MeshBuilder,
  c: ContactShape,
  life: number,
  s: ContactGlyphStyle,
  z: SurfaceHeight,
  facing: ContactFacing,
) {
  const { air } = s;
  const { stroke } = facing;
  const radius = Math.min(Math.max(c.radius, stroke(air.min_radius_px)), stroke(air.max_radius_px));
  const at: P3 = [c.center[0], c.center[1], c.center[2] ?? 0];
  draw(
    facingPen(mesh, at, facing),
    life,
    {
      radius,
      outline: Math.min(radius, facing.stroke(air.rim_px)),
      scale: radius / c.radius,
      hatchWidth: facing.stroke(air.hatch_px),
      fill: air.fill_alpha,
      hatch: air.hatch_alpha,
      glow: air.glow_alpha,
      phase: 0,
      keyline: { width: facing.stroke(air.keyline_px), alpha: air.keyline_alpha },
    },
    s,
  );
  // The stem stands from the ground under the centre to where it meets the
  // sign's rim on screen: a vertical drop of `radius` along the screen's up
  // is `radius / up.z` in height. Seen from straight above, or with the
  // sign over its own ground point, there is no stem to draw.
  const ground = z(at[0], at[1]);
  const top = facing.up[2] > 1e-3 ? at[2] - radius / facing.up[2] : -Infinity;
  if (top > ground)
    mesh.segment(
      [at[0], at[1], ground],
      [at[0], at[1], top],
      facing.stroke(air.stem_px) / 2,
      rgbA(s.outline_color, air.stem_alpha * life),
    );
}

function glyph(
  mesh: MeshBuilder,
  c: ContactShape,
  s: ContactGlyphStyle,
  z: SurfaceHeight,
  facing: ContactFacing,
) {
  if (!(c.opacity > 0) || !(c.radius > 0)) return;
  const life = Math.min(1, c.opacity);
  if (c.aloft) {
    aloft(mesh, c, life, s, z, facing);
    return;
  }
  // On the ground the hatch is anchored to the world's lines, so it never
  // swims as reports come and go.
  const a = (s.hatch_angle_deg * Math.PI) / 180;
  const phase = -c.center[0] * Math.sin(a) + c.center[1] * Math.cos(a);
  draw(
    groundPen(mesh, [c.center[0], c.center[1]], z, s.lift_m),
    life,
    {
      radius: c.radius,
      outline: s.outline_width_m,
      scale: 1,
      hatchWidth: s.hatch_width_m,
      fill: s.fill_alpha,
      hatch: s.hatch_alpha,
      glow: s.glow_alpha,
      phase,
      keyline: null,
    },
    s,
  );
}

/** Every report's glyph: on the ground, or aloft facing the camera as
 *  `facing` says. */
export function buildContactGlyphs(
  contacts: readonly ContactShape[],
  z: SurfaceHeight,
  style: ContactGlyphStyle,
  facing: ContactFacing,
): WorldMeshes {
  const translucent = new MeshBuilder();
  for (const c of contacts) glyph(translucent, c, style, z, facing);
  return { opaque: EMPTY_MESH, translucent: translucent.build() };
}
