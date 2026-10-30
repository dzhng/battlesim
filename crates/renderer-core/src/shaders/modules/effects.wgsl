// WGSL owner: effects.

// @block effects_OverlayGlow
struct OverlayGlow {
  radiusPx: f32,
  sigmaPx: f32,
  strength: f32,
  pad: f32,
}

// @block effects_compositeOverlay
fn compositeOverlay(pixel: vec2f) -> vec4f {
  let o = textureLoad(overlay, vec2i(pixel), 0);
  let g = glow;
  if (g.strength <= 0.0) { return o; }
  let top = vec2i(textureDimensions(halo)) - 1;
  let at = pixel * 0.5 - 0.5;
  let p0 = vec2i(floor(at));
  let f = at - floor(at);
  let a = textureLoad(halo, clamp(p0, vec2i(0), top), 0);
  let b = textureLoad(halo, clamp(p0 + vec2i(1, 0), vec2i(0), top), 0);
  let c = textureLoad(halo, clamp(p0 + vec2i(0, 1), vec2i(0), top), 0);
  let e = textureLoad(halo, clamp(p0 + vec2i(1, 1), vec2i(0), top), 0);
  let blur = mix(mix(a, b, f.x), mix(c, e, f.x), f.y);
  let gain = min(g.strength, 1.0 / max(blur.w, 1e-4));
  return o + (1.0 - o.w) * blur * gain;
}

// @block effects_glow
@group(0) @binding(0) var<uniform> glow: OverlayGlow;

// @block effects_glowColumns
fn glowColumns(pixel: vec2f) -> vec4f {
  let g = glow;
  let size = vec2i(textureDimensions(rows));
  let p = vec2i(pixel);
  let reach = i32(ceil(g.radiusPx * 0.5));
  var sum = vec4f(0.0);
  var total = 0.0;
  for (var k = -reach; k <= reach; k++) {
    let y = f32(k) * 2.0;
    let w = exp(-(y * y) / (2.0 * g.sigmaPx * g.sigmaPx));
    sum += w * textureLoad(rows, clamp(p + vec2i(0, k), vec2i(0), size - 1), 0);
    total += w;
  }
  return sum / total;
}

// @block effects_glowRows
fn glowRows(pixel: vec2f) -> vec4f {
  let g = glow;
  let size = vec2i(textureDimensions(overlay));
  let base = vec2i(pixel) * 2;
  let reach = i32(ceil(g.radiusPx * 0.5));
  var sum = vec4f(0.0);
  var total = 0.0;
  for (var k = -reach; k <= reach; k++) {
    let x = f32(k) * 2.0;
    let w = exp(-(x * x) / (2.0 * g.sigmaPx * g.sigmaPx));
    for (var dy = 0; dy < 2; dy++) {
      for (var dx = 0; dx < 2; dx++) {
        let p = clamp(base + vec2i(k * 2 + dx, dy), vec2i(0), size - 1);
        sum += w * textureLoad(overlay, p, 0);
      }
    }
    total += 4.0 * w;
  }
  return sum / total;
}

// @block effects_highlightGlow
fn highlightGlow(highlight: f32) -> vec3f {
  return (vec3f(0.949999988079071, 0.800000011920929, 0.20000000298023224) * (highlight * 0.7f));
}

// @block effects_overlay
@group(0) @binding(1) var overlay: texture_2d<f32>;

// @block effects_paintColour
fn paintColour(paint: vec4f) -> vec3f {
  let c = pow(max(paint.xyz / max(paint.w, 1e-4), vec3f(0.0)), vec3f(2.2));
  let grey = dot(c, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B));
  return max(mix(vec3f(grey), c, paintStyle.saturation), vec3f(0.0));
}

// @block effects_paintGlow
fn paintGlow(paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  return paintColour(paint) * paintStyle.emissive * paint.w;
}

// @block effects_paint
@group(2) @binding(7) var paint: texture_2d<f32>;

// @block effects_paintedAlbedo
fn paintedAlbedo(albedo: vec3f, paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return albedo; }
  let colour = min(paintColour(paint), vec3f(1.0));
  return mix(albedo, colour * paintStyle.albedo, paint.w);
}
// @block effects_GroundPaintStyle
struct GroundPaintStyle {
  albedo: f32,
  emissive: f32,
  fogKeep: f32,
  grassGlow: f32,
  grassFalloffM: f32,
  saturation: f32,
  pad1: f32,
  pad2: f32,
}

// @block effects_SpriteLight
/** The world's light on a lit sprite: the sun in the sprite's own frame
 *  (the camera's right, up, and toward the eye), and the sky's diffuse
 *  light (its PMREM at full roughness, times the fill, as world materials
 *  take it) on its top and on its side. */
struct SpriteLight {
  sun: vec3f,
  skyTop: vec3f,
  skySide: vec3f,
};

// @block effects_paintedSeen
fn paintedSeen(seen: f32, paint: vec4f) -> f32 {
  return mix(seen, mix(1.0, seen, paintStyle.fogKeep), paint.w);
}

// @block effects_smokeLight
/** The world's light on smoke at `o` on a lit sprite (-1..1, screen-
 *  aligned): a soft ball's (or, across a ribbon, a tube's) normal against the
 *  sun, wrapped (light scatters through smoke), and the sun scattered on
 *  toward the eye, strongest looking into it (Henyey-Greenstein, relative to
 *  isotropic): backlit dust and smoke glow rather than going dark. Plus the
 *  sky's light, more from above. */
fn smokeLight(f: SpriteVertexOutput, o: vec2f) -> vec3f {
  let n = normalize(vec3f(o, sqrt(max(0.0, 1.0 - dot(o, o))) + 0.3));
  let wrap = clamp((dot(n, f.sun) + 0.3) / 1.3, 0.0, 1.0);
  let g = 0.4;
  let forward = (1.0 - g * g) / pow(1.0 + g * g + 2.0 * g * f.sun.z, 1.5);
  let sky = mix(f.skySide, f.skyTop, clamp(n.y * 0.5 + 0.5, 0.0, 1.0));
  return light.sun.radiance.rgb * ((0.6 * wrap + 0.4 * forward) / PI) + sky + f.castLit;
}

// @block effects_grassPaintGlow
fn grassPaintGlow(paint: vec4f, height: f32) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  let s = paintStyle;
  let colour = paintColour(paint);
  return colour * s.grassGlow * paint.w * exp(-max(height, 0.0) / max(s.grassFalloffM, 1e-3));
}

// @block effects_LUMA
const LUMA = vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B);

// @block effects_PI
const PI = 3.14159265;

// @block effects_ROTOR_SOFT_M
/** How deep a rotor fades into what it meets, metres: a roof just under it
 *  still takes it. */
const ROTOR_SOFT_M = 0.05;

// @block effects_atlas
@group(0) @binding(2) var atlas: texture_2d_array<f32>;

// @block effects_cell
fn cell(tuv: vec2f, frame: f32, cols: f32) -> vec2f {
  let at = vec2f(frame % cols, floor(frame / cols));
  return (at + tuv) / cols;
}

// @block effects_halo
@group(0) @binding(2) var halo: texture_2d<f32>;

// @block effects_planDistance
/** How far (m) a point `p` (along the nose, across) lies outside an
 *  aircraft's plan seen from above, `half` its half length and half span;
 *  negative inside. A fighter's arrowhead in proportion to the plan: a broad,
 *  strongly swept near-delta wing spanning the plan's width with its root
 *  along most of the rear half, a short tapered nose ahead of it, and small
 *  tailplanes merging into the wing's trailing corners. */
fn planDistance(p: vec2f, half: vec2f) -> f32 {
  // Along the nose in half lengths (+1 its tip, −1 its tail).
  let x = p.x / half.x;
  let y = abs(p.y);
  // The fuselage: as wide as `bodyHalf` behind the wing's root, tapering
  // to a point at the nose.
  let bodyHalf = max(0.1 * half.x, 0.5);
  let width = bodyHalf * clamp((1.0 - x) / 0.7, 0.0, 1.0);
  let body = max(y - width, (abs(x) - 1.0) * half.x);
  // A wing's leading and trailing edges (half lengths) a share `t` of its
  // reach out from the root.
  let t = y / half.y;
  let wing = max(y - half.y, max(-0.8 + 0.1 * t - x, x - (0.25 - 0.8 * t)) * half.x);
  let tailHalf = 0.5 * half.y;
  let u = y / tailHalf;
  let tail = max(y - tailHalf, max(-1.0 - x, x - (-0.55 - 0.35 * u)) * half.x);
  return min(body, min(wing, tail));
}

// @block effects_rows_glowColumnsLayout
@group(0) @binding(1) var rows: texture_2d<f32>;

// @block effects_linearSampler_effects
@group(0) @binding(3) var linearSampler: sampler;

// @block effects_sceneDepth
@group(0) @binding(1) var sceneDepth: texture_depth_multisampled_2d;

// @block effects_spriteLight
fn spriteLight() -> SpriteLight {
  // The camera's right and up are the view-projection's first two rows.
  let m = cam.viewProj;
  let right = normalize(vec3f(m[0].x, m[1].x, m[2].x));
  let up = normalize(vec3f(m[0].y, m[1].y, m[2].y));
  let toward = cross(right, up);
  let sun = light.sun.direction.xyz;
  var l: SpriteLight;
  l.sun = vec3f(dot(sun, right), dot(sun, up), dot(sun, toward));
  // The PMREM wants y flipped.
  let maxMip = light.sun.settings.x;
  let side = normalize(vec3f(toward.x, toward.y, 0.0) + vec3f(0.0, 0.0, 1e-3));
  l.skyTop = samplePmrem(pmrem, linearSampler, vec3f(0.0, 0.0, 1.0), 1.0, maxMip) * light.sun.fill.xyz;
  l.skySide = samplePmrem(pmrem, linearSampler, vec3f(side.x, -side.y, side.z), 1.0, maxMip)
    * light.sun.fill.xyz;
  return l;
}

// @block effects_groundPaint
fn groundPaint(ground: vec3f) -> vec4f {
  if (layer.painted == 0u) { return vec4f(0.0); }
  let c = cam.viewProj * vec4f(ground, 1.0);
  if (c.w <= 0.0) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(paint));
  let ndc = c.xy / c.w;
  let px = vec2i(floor(vec2f(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * vec2f(size)));
  if (any(px < vec2i(0)) || any(px >= size)) { return vec4f(0.0); }
  let p = textureLoad(paint, px, 0);
  return vec4f(p.xyz * 2, p.w);
}

// @block effects_groundPaintAtPixel
fn groundPaintAtPixel(pixel: vec2f) -> vec4f {
  if (layer.painted == 0u) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(paint));
  let p = textureLoad(paint, clamp(vec2i(pixel), vec2i(0), size - 1), 0);
  return vec4f(p.xyz * 2, p.w);
}

// @block effects_paintStyle
@group(2) @binding(6) var<uniform> paintStyle: GroundPaintStyle;
