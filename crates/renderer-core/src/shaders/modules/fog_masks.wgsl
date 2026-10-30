// WGSL owner: fog_masks.

// @block fog_masks_FogLook
struct FogLook {
  tint: vec3f,
  dim: f32,
  cool: f32,
  saturation: f32,
  veil: f32,
  lineStrength: f32,
  lineSpacingPx: f32,
  lineWidthPx: f32,
  lineAngle: f32,
  lineFloor: f32,
  edgeSoftnessPx: f32,
  rimColor: vec3f,
  rimAlpha: f32,
  rimWidthPx: f32,
  reachPx: f32,
}

// @block fog_masks_FogMaskView
struct FogMaskView {
  mask: u32,
}

// @block fog_masks_composeFog
fn composeFog(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let lit = textureLoad(lit, p, 0);
  let m = textureLoad(mask, p, 0).xy;
  if (view.mask == 1u) {
    // Unseen (more than half the pixel's samples) black; untouched (none)
    // white; partly unseen grey.
    let v = select(select(0.35, 16.0, m.x <= 0.0), 0.0, m.x > 0.5);
    return vec4f(vec3f(v), 1.0);
  }
  if (view.mask == 2u) {
    // Mostly ground (more than half the pixel's samples) white, else black.
    let ground = textureLoad(mask, p, 0).z > 0.5;
    return vec4f(vec3f(select(0.0, 16.0, ground)), 1.0);
  }
  if (view.mask == 3u) {
    // The terrain's bytes where every sample of the pixel is ground; black
    // where anything else covers any of it.
    let ground = textureLoad(mask, p, 0).z >= 0.999;
    return vec4f(select(vec3f(0.0), lit.xyz, ground), 1.0);
  }
  if (m.x <= 0.0) { return lit; }
  let s = look;
  var unseen = m.x;
  if (s.edgeSoftnessPx > 0.0) {
    let toSeen = textureLoad(distance_1, p, 0).y * 16.0;
    unseen *= smoothstep(0.0, 1.0, (toSeen - 0.5) / s.edgeSoftnessPx);
  }
  return vec4f(mix(lit.xyz, fogLook(lit.xyz, pixel, s), unseen), lit.w);
}

// @block fog_masks_view
@group(0) @binding(1) var<uniform> view: FogMaskView;
// @block fog_masks_fogLook
fn fogLook(lit: vec3f, pixel: vec2f, s: FogLook) -> vec3f {
  let lum = dot(lit, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B));
  let kept = mix(vec3f(lum), lit, s.saturation);
  let c = mix(kept, lum * s.tint, s.cool) * s.dim;
  // Distance to the nearest line, in pixels, across the lines' direction.
  let across = vec2f(sin(s.lineAngle), cos(s.lineAngle));
  let t = dot(pixel, across) / s.lineSpacingPx;
  let dist = abs(fract(t + 0.5) - 0.5) * s.lineSpacingPx;
  let cover = clamp(s.lineWidthPx * 0.5 + 0.5 - dist, 0.0, 1.0);
  return c * (1.0 + s.lineStrength * cover) + s.tint * (s.veil + s.lineFloor * cover);
}

// @block fog_masks_look
@group(0) @binding(0) var<uniform> look: FogLook;

// @block fog_masks_distanceColumns
fn distanceColumns(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  if (textureLoad(mask, p, 0).z <= 0.5) { return vec4f(1.0, 1.0, 0.0, 1.0); }
  let height = i32(textureDimensions(rows).y);
  let reach = i32(look.reachPx);
  var best = vec2f(16.0 * 16.0);
  for (var k = -reach; k <= reach; k++) {
    let y = p.y + k;
    if (y < 0 || y >= height) { continue; }
    let row = textureLoad(rows, vec2i(p.x, y), 0).xy * 16.0;
    best = min(best, row * row + vec2f(f32(k * k)));
  }
  return vec4f(min(sqrt(best), vec2f(16.0)) / 16.0, 0.0, 1.0);
}

// @block fog_masks_distance_texture
@group(0) @binding(4) var distance_1: texture_2d<f32>;

// @block fog_masks_mask_composeLayout
@group(0) @binding(3) var mask: texture_2d<f32>;

// @block fog_masks_mask_rowsLayout
@group(0) @binding(1) var mask: texture_2d<f32>;

// @block fog_masks_rimAt
fn rimAt(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let s = look;
  let m = textureLoad(mask, p, 0).xy;
  let seen = m.y;
  if (seen <= 0.0 || m.x > 0.5 || s.rimAlpha <= 0.0 || s.rimWidthPx <= 0.0) {
    return vec4f(0.0);
  }
  let toUnseen = textureLoad(distance_1, p, 0).x * 16.0;
  let cover = clamp(s.rimWidthPx + 1.0 - toUnseen, 0.0, 1.0);
  return vec4f(s.rimColor, s.rimAlpha * cover * seen);
}

// @block fog_masks_rows_columnsLayout
@group(0) @binding(2) var rows: texture_2d<f32>;

// @block fog_masks_distanceRows
fn distanceRows(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let width = i32(textureDimensions(mask).x);
  let reach = i32(look.reachPx);
  var best = vec2f(16.0);
  for (var k = -reach; k <= reach; k++) {
    let x = p.x + k;
    if (x < 0 || x >= width) { continue; }
    let side = groundSide(textureLoad(mask, vec2i(x, p.y), 0));
    best = min(best, select(vec2f(16.0), vec2f(f32(abs(k))), side > vec2f(0.5)));
  }
  return vec4f(best / 16.0, 0.0, 1.0);
}

// @block fog_masks_groundSide
fn groundSide(m: vec4f) -> vec2f {
  if (m.z <= 0.5) { return vec2f(0.0); }
  return vec2f(f32(m.x > 0.5 && m.y <= 0.0), f32(m.y > 0.5 && m.x <= 0.0));
}
