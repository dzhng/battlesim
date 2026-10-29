// WGSL owner: shared_bindings.

// @block shared_bindings_Count
struct Count {
  total: atomic<u32>,
  hidden: atomic<u32>,
}

// @block shared_bindings_FragmentIn
struct FragmentIn {
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

// @block shared_bindings_In
struct In {
  @builtin(vertex_index) vi: u32,
  @location(0) a: vec4f,
  @location(1) b: vec4f,
  @location(2) color: vec4f,
  @location(3) misc: vec4f,
};

// @block shared_bindings_LUMA_alternate
const LUMA: vec3f = vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023);

// @block shared_bindings_LUMA_primary
const LUMA = vec3f(0.2126, 0.7152, 0.0722);

// @block shared_bindings_NEAR_W
const NEAR_W = 0.05;

// @block shared_bindings_PI
const PI = 3.14159265;

// @block shared_bindings_ROTOR_SOFT_M
/** How deep a rotor fades into what it meets, metres: a roof just under it
 *  still takes it. */
const ROTOR_SOFT_M = 0.05;

// @block shared_bindings_Rays
struct Rays {
  origin: vec3f,
  dx: vec3f,
  dy: vec3f,
}

// @block shared_bindings_VertexIn
struct VertexIn {
  @location(0) position: vec3f,
  @location(1) uv: vec2f,
  @location(2) face: f32,
}

// @block shared_bindings_VertexOut
struct VertexOut {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

// @block shared_bindings__Input
struct _Input {
  @location(0) world: vec3f,
  @location(1) uv: vec2f,
  @location(2) @interpolate(flat) layer: u32,
  @location(3) @interpolate(flat) yaw: f32,
  @location(4) @interpolate(flat) scale: vec3f,
  @location(5) tint: vec3f,
  @location(6) anchor: vec3f,
}

// @block shared_bindings_albedo_alternate
@group(1) @binding(0) var albedo: texture_2d_array<f32>;

// @block shared_bindings_albedo_primary
@group(1) @binding(2) var albedo: texture_2d_array<f32>;

// @block shared_bindings_args
@group(0) @binding(4) var<storage, read_write> args: array<atomic<u32>, 12>;

// @block shared_bindings_atLeast
fn atLeast(colour: vec3f, least: f32) -> vec3f {
  let luminance = dot(colour, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023));
  return (colour * max(1f, (least / max(luminance, 1e-5f))));
}

// @block shared_bindings_atlas
@group(0) @binding(2) var atlas: texture_2d_array<f32>;

// @block shared_bindings_backdropFragment_Input
struct backdropFragment_Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block shared_bindings_bakeShade
fn bakeShade(uv: vec2f, face: f32) -> vec4f {
  return bakeAlgorithm(uv, face, source, linear);
}

// @block shared_bindings_cam
@group(0) @binding(0) var<uniform> cam: Camera;

// @block shared_bindings_cell
fn cell(tuv: vec2f, frame: f32, cols: f32) -> vec2f {
  let at = vec2f(frame % cols, floor(frame / cols));
  return (at + tuv) / cols;
}

// @block shared_bindings_classDistance
fn classDistance(inside: f32) -> f32 {
  return (clamp(round((128f - (inside * 8f))), 1f, 255f) / 255f);
}

// @block shared_bindings_counts_alternate
@group(2) @binding(5) var<storage, read> counts: array<u32>;

// @block shared_bindings_counts_extended
@group(2) @binding(0) var<storage, read> counts: array<vec2u>;

// @block shared_bindings_counts_layered
@group(3) @binding(5) var<storage, read> counts: array<u32>;

// @block shared_bindings_counts_masked
@group(2) @binding(0) var<storage, read_write> counts: array<Count>;

// @block shared_bindings_counts_primary
@group(1) @binding(5) var<storage, read> counts: array<u32>;

// @block shared_bindings_counts_variant
@group(0) @binding(4) var<storage, read_write> counts: array<atomic<u32>>;

// @block shared_bindings_cullHi
var<workgroup> cullHi: array<atomic<i32>, 2>;

// @block shared_bindings_cullHit
var<workgroup> cullHit: atomic<u32>;

// @block shared_bindings_cullLo
var<workgroup> cullLo: array<atomic<i32>, 2>;

// @block shared_bindings_dfg_alternate
@group(3) @binding(4) var dfg: texture_2d<f32>;

// @block shared_bindings_dfg_extended
@group(1) @binding(4) var dfg: texture_2d<f32>;

// @block shared_bindings_dfg_primary
@group(2) @binding(4) var dfg: texture_2d<f32>;

// @block shared_bindings_direction
fn direction(uv: vec2f) -> vec3f {
  let theta = (uv.x - 0.5) * 6.2831853071795862e+0f;
  let phi = (uv.y - 0.5) * 3.1415926535897931e+0f;
  return vec3f(cos(phi) * cos(theta), sin(phi), cos(phi) * sin(theta));
}

// @block shared_bindings_distanceColumns
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

// @block shared_bindings_distance_texture
@group(0) @binding(4) var distance_1: texture_2d<f32>;

// @block shared_bindings_equirectUv
fn equirectUv(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * 1.5915494309189535e-1f + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * 3.1830988618379069e-1f + 0.5);
}

// @block shared_bindings_filterShade
fn filterShade(uv: vec2f, face: f32) -> vec4f {
  return filterAlgorithm(uv, face, source, linear, params);
}

// @block shared_bindings_flags
@group(0) @binding(2) var<storage, read_write> flags: array<u32>;

// @block shared_bindings_foliage
@group(0) @binding(3) var<storage, read> foliage: array<vec4f>;

// @block shared_bindings_fragment_Input
struct fragment_Input {
  @location(0) uv: vec2f,
}

// @block shared_bindings_frames
@group(1) @binding(3) var<storage, read> frames: array<vec4f>;

// @block shared_bindings_fullScreenTriangle_Output
struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

// @block shared_bindings_halo
@group(0) @binding(2) var halo: texture_2d<f32>;

// @block shared_bindings_heights_alternate
@group(0) @binding(1) var<storage, read> heights: array<u32>;

// @block shared_bindings_heights_primary
@group(0) @binding(2) var<storage, read> heights: array<u32>;

// @block shared_bindings_items
@group(0) @binding(3) var<storage, read> items: array<u32>;

// @block shared_bindings_level0
@group(0) @binding(1) var level0: texture_2d<f32>;

// @block shared_bindings_level1
@group(0) @binding(2) var level1: texture_2d<f32>;

// @block shared_bindings_level2
@group(0) @binding(3) var level2: texture_2d<f32>;

// @block shared_bindings_level3
@group(0) @binding(4) var level3: texture_2d<f32>;

// @block shared_bindings_level4
@group(0) @binding(5) var level4: texture_2d<f32>;

// @block shared_bindings_lists_alternate
@group(3) @binding(4) var<storage, read> lists: array<u32>;

// @block shared_bindings_lists_extended
@group(0) @binding(3) var<storage, read_write> lists: array<u32>;

// @block shared_bindings_lists_masked
@group(2) @binding(4) var<storage, read> lists: array<u32>;

// @block shared_bindings_lists_primary
@group(1) @binding(4) var<storage, read> lists: array<u32>;

// @block shared_bindings_lit
@group(0) @binding(2) var lit: texture_2d<f32>;

// @block shared_bindings_lutView
@group(0) @binding(1) var lutView: texture_2d<f32>;

// @block shared_bindings_maps_alternate
@group(0) @binding(7) var<storage, read_write> maps: array<u32>;

// @block shared_bindings_maps_extended
@group(3) @binding(3) var<storage, read> maps: array<u32>;

// @block shared_bindings_maps_layered
@group(0) @binding(3) var<storage, read> maps: array<u32>;

// @block shared_bindings_maps_masked
@group(2) @binding(3) var<storage, read> maps: array<u32>;

// @block shared_bindings_maps_primary
@group(1) @binding(3) var<storage, read> maps: array<u32>;

// @block shared_bindings_mask_alternate
@group(0) @binding(3) var mask: texture_2d<f32>;

// @block shared_bindings_mask_primary
@group(0) @binding(1) var mask: texture_2d<f32>;

// @block shared_bindings_minimum
@group(2) @binding(1) var<uniform> minimum: f32;

// @block shared_bindings_normal
@group(1) @binding(1) var normal: texture_2d_array<f32>;

// @block shared_bindings_out_alternate
@group(0) @binding(1) var<storage, read_write> out: array<f32>;

// @block shared_bindings_out_primary
@group(1) @binding(1) var<storage, read_write> out: array<u32>;

// @block shared_bindings_outputSrgb
fn outputSrgb(linear: vec3f) -> vec3f {
  return select(pow(linear,vec3f(0.41666))*1.055-0.055,linear*12.92,linear<=vec3f(0.0031308));
}

// @block shared_bindings_params
@group(0) @binding(2) var<uniform> params: vec4f;

// @block shared_bindings_pastTrunk
fn pastTrunk(d: f32, radius: f32, k: f32) -> f32 {
  let m = abs(d);
  return sign(d) * (min(m, radius) + max(m - radius, 0.0) * k);
}

// @block shared_bindings_placedNormal
fn placedNormal(normal: vec3f, shape: vec4f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  let nx = (normal.x / shape.x);
  let ny = (normal.y / shape.y);
  return normalize(vec3f(((nx * c) - (ny * s)), ((nx * s) + (ny * c)), (normal.z / shape.z)));
}

// @block shared_bindings_placedPoint
fn placedPoint(local: vec3f, pose: vec4f) -> vec3f {
  let c = cos(pose.w);
  let s = sin(pose.w);
  return vec3f((((local.x * c) - (local.y * s)) + pose.x), (((local.x * s) + (local.y * c)) + pose.y), (local.z + pose.z));
}

// @block shared_bindings_placedVertex_Output
struct placedVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block shared_bindings_planDistance
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

// @block shared_bindings_polygonEdgeDistance
fn polygonEdgeDistance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
 let ab=b-a;let len2=dot(ab,ab);var t=0.0;if(len2>0.0){t=clamp(dot(p-a,ab)/len2,0.0,1.0);}return length(p-(a+ab*t));
}

// @block shared_bindings_polygonTriangleInside
fn polygonTriangleInside(xy: vec2f, a: vec2f, b: vec2f, c: vec2f) -> bool {
 let ab=b-a;let bc=c-b;let ca=a-c;
 let area=dot(vec2f(-ab.y,ab.x),c-a);
 let s0=dot(vec2f(-ab.y,ab.x),xy-a);let s1=dot(vec2f(-bc.y,bc.x),xy-b);let s2=dot(vec2f(-ca.y,ca.x),xy-c);
 return area!=0.0&&((min(s0,min(s1,s2))>=0.0)||(max(s0,max(s1,s2))<=0.0));
}

// @block shared_bindings_projScale
/** Clip-space units per metre at unit depth, across and up. */
fn projScale() -> vec2f {
  let m = cam.viewProj;
  return vec2f(length(vec3f(m[0].x, m[1].x, m[2].x)), length(vec3f(m[0].y, m[1].y, m[2].y)));
}

// @block shared_bindings_props
@group(0) @binding(2) var<storage, read> props: array<vec4f>;

// @block shared_bindings_radicalInverse
fn radicalInverse(i: u32) -> f32 {
  var b=(i<<16u)|(i>>16u);
  b=((b&0x55555555u)<<1u)|((b&0xAAAAAAAAu)>>1u);
  b=((b&0x33333333u)<<2u)|((b&0xCCCCCCCCu)>>2u);
  b=((b&0x0F0F0F0Fu)<<4u)|((b&0xF0F0F0F0u)>>4u);
  b=((b&0x00FF00FFu)<<8u)|((b&0xFF00FF00u)>>8u);
  return f32(b)*2.3283064365386963e-10;
}

// @block shared_bindings_rays
@group(0) @binding(3) var<uniform> rays: Rays;

// @block shared_bindings_readWord
fn readWord(at: u32) -> u32 {
  return heights[at];
}

// @block shared_bindings_rebuild
@group(0) @binding(5) var<storage, read> rebuild: array<u32>;

// @block shared_bindings_rectInside
fn rectInside(xy: vec2f, r: vec4f) -> f32 {
  return min(min((xy.x - r.x), (r.z - xy.x)), min((xy.y - r.y), (r.w - xy.y)));
}

// @block shared_bindings_rimAt
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

// @block shared_bindings_rows_alternate
@group(0) @binding(1) var rows: texture_2d<f32>;

// @block shared_bindings_rows_extended
@group(0) @binding(0) var<storage, read> rows: array<vec4f>;

// @block shared_bindings_rows_primary
@group(0) @binding(2) var rows: texture_2d<f32>;

// @block shared_bindings_scene
@group(0) @binding(1) var scene: texture_2d<f32>;

// @block shared_bindings_source_alternate
@group(0) @binding(0) var source: texture_2d<f32>;

// @block shared_bindings_source_primary
@group(0) @binding(1) var source: texture_2d<f32>;

// @block shared_bindings_srgbToLinear
fn srgbToLinear(c: vec3f) -> vec3f {
  return pow(max(c, vec3f()), vec3f(2.200000047683716));
}

// @block shared_bindings_step_uniform
@group(0) @binding(2) var<uniform> step_1: vec2f;

// @block shared_bindings_strokeCutInside
fn strokeCutInside(xy: vec2f, ends: vec4f, detail: vec4f, inside: f32) -> f32 {
 let cuts=u32(detail.z);
 if(cuts==0u){return inside;}
 let a=ends.xy;let ab=ends.zw-a;let len=length(ab);
 let along=dot(xy-a,ab)/len;
 let past=max(select(-1e9,-along,(cuts&1u)!=0u),select(-1e9,along-len,(cuts&2u)!=0u));
 if(past<=0.0){return min(inside,-past);}
 let rel=xy-a;
 let aside=max(abs(rel.x*ab.y-rel.y*ab.x)/len-detail.x,0.0);
 return -length(vec2f(aside,past));
}

// @block shared_bindings_tier
@group(1) @binding(1) var<uniform> tier: vec4u;

// @block shared_bindings_triangleRuleHeight
fn triangleRuleHeight(h00: f32, h10: f32, h01: f32, h11: f32, u: f32, v: f32) -> f32 {
  if (u >= v) { return h00 + u * (h10 - h00) + v * (h11 - h10); }
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}

// @block shared_bindings_turned
fn turned(xy: vec2f, turn: vec2f) -> vec2f {
  return vec2f(((xy.x * turn.x) + (xy.y * turn.y)), ((xy.y * turn.x) - (xy.x * turn.y)));
}

// @block shared_bindings_roughnessMip
fn roughnessMip(r: f32) -> f32 {
  if(r>=0.8){return (1.0-r)/0.2-2.0;}
  if(r>=0.4){return (0.8-r)*3.0/0.4-1.0;}
  if(r>=0.305){return (0.4-r)/0.095+2.0;}
  if(r>=0.21){return (0.305-r)/0.095+3.0;}
  return -2.0*log2(1.16*r);
}
