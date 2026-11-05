// WGSL owner: readouts.

// @block readouts_atlas_sampled
@group(0) @binding(0) var readoutAtlas: texture_2d<f32>;

// @block readouts_sampler_sampled
@group(0) @binding(1) var readoutSampler: sampler;

// @block readouts_ReadoutQuad
// One laid-out card primitive (`readouts.rs`'s `ReadoutQuad`).
struct ReadoutQuad {
  @location(0) ndc: vec4f,
  @location(1) px: vec4f,
  @location(2) shape: vec4f,
  @location(3) uv: vec4f,
  @location(4) ink: vec4f,
  @location(5) a: vec4f,
  @location(6) b: vec4f,
  @location(7) params: vec4f,
  @location(8) dash: vec4f,
}

// @block readouts_ReadoutVarying
struct ReadoutVarying {
  @builtin(position) position: vec4f,
  @location(0) pixel: vec2f,
  @location(1) uv: vec2f,
  @location(2) @interpolate(flat) shape: vec4f,
  @location(3) @interpolate(flat) ink: vec4f,
  @location(4) @interpolate(flat) a: vec4f,
  @location(5) @interpolate(flat) b: vec4f,
  @location(6) @interpolate(flat) params: vec4f,
  @location(7) @interpolate(flat) dash: vec4f,
}

// @block readouts_readoutVertex
@vertex
fn readoutVertex(quad: ReadoutQuad, @builtin(vertex_index) corner: u32) -> ReadoutVarying {
  var corners = array<vec2f, 6>(
    vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0),
    vec2f(0.0, 1.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0),
  );
  let k = corners[corner];
  var out: ReadoutVarying;
  out.position = vec4f(mix(quad.ndc.xy, quad.ndc.zw, k), 0.0, 1.0);
  out.pixel = mix(quad.px.xy, quad.px.zw, k);
  out.uv = mix(quad.uv.xy, quad.uv.zw, k);
  out.shape = quad.shape;
  out.ink = quad.ink;
  out.a = quad.a;
  out.b = quad.b;
  out.params = quad.params;
  out.dash = quad.dash;
  return out;
}

// @block readouts_readoutErf
// The error function (Abramowitz and Stegun 7.1.26, within 1.5e-7).
fn readoutErf(x: f32) -> f32 {
  let a = abs(x);
  let t = 1.0 / (1.0 + 0.3275911 * a);
  let poly = ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t;
  return sign(x) * (1.0 - poly * exp(-a * a));
}

// @block readouts_readoutBlurSpan
// The share of a Gaussian of `sigma` round `x` that falls in [lo, hi]: a
// blurred edge's coverage along one axis.
fn readoutBlurSpan(x: f32, lo: f32, hi: f32, sigma: f32) -> f32 {
  let s = max(sigma, 1e-3) * 1.41421356;
  return 0.5 * (readoutErf((hi - x) / s) - readoutErf((lo - x) / s));
}

// @block readouts_readoutBoxCoverage
// How much of the pixel at `p` a rectangle covers, its edges antialiased.
fn readoutBoxCoverage(p: vec2f, r: vec4f) -> f32 {
  let x = clamp(min(p.x - r.x, r.z - p.x) + 0.5, 0.0, 1.0);
  let y = clamp(min(p.y - r.y, r.w - p.y) + 0.5, 0.0, 1.0);
  return x * y;
}

// @block readouts_readoutOver
// `top` (straight colour at coverage `alpha`) over premultiplied `under`.
fn readoutOver(top: vec4f, alpha: f32, under: vec4f) -> vec4f {
  let a = clamp(top.a * alpha, 0.0, 1.0);
  return vec4f(top.rgb * a, a) + (1.0 - a) * under;
}

// @block readouts_readoutFragment
@fragment
fn readoutFragment(v: ReadoutVarying) -> @location(0) vec4f {
  let kind = i32(round(v.params.x));
  let p = v.pixel;
  if (kind == 1) {
    // A premultiplied sprite at its opacity.
    return textureSampleLevel(readoutAtlas, readoutSampler, v.uv, 0.0) * v.ink.a;
  }
  if (kind == 0) {
    let r = v.shape;
    let blur = v.params.z;
    if (blur > 0.0) {
      let c = readoutBlurSpan(p.x, r.x, r.z, blur) * readoutBlurSpan(p.y, r.y, r.w, blur);
      return readoutOver(v.ink, c, vec4f(0.0));
    }
    let border = v.params.y;
    let outer = readoutBoxCoverage(p, r);
    let inner = readoutBoxCoverage(p, r + vec4f(border, border, -border, -border));
    let ring = readoutOver(v.a, outer - inner, vec4f(0.0));
    return readoutOver(v.ink, inner, vec4f(0.0)) + ring * (1.0 - v.ink.a * inner);
  }
  // A stroke: its distance from the pixel and how far along it lies.
  var d: f32;
  var along: f32;
  if (kind == 2) {
    let a = v.shape.xy;
    let ab = v.shape.zw - a;
    let t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
    d = length(p - (a + ab * t));
    along = t * length(ab);
  } else {
    let q = p - v.shape.xy;
    d = abs(length(q) - v.shape.z);
    along = (atan2(q.y, q.x) + 3.14159265) * v.shape.z;
  }
  let halfWidth = v.params.y * 0.5;
  var ink = clamp(halfWidth - d + 0.5, 0.0, 1.0);
  let period = v.dash.x + v.dash.y;
  if (v.dash.x > 0.0 && period > 0.0) {
    let at = (along + v.dash.z) % period;
    ink *= clamp(v.dash.x - at + 0.5, 0.0, 1.0) * clamp(at + 0.5, 0.0, 1.0);
  }
  // A blurred stroke's coverage across its width, for each shadow.
  let first = readoutBlurSpan(d, -halfWidth, halfWidth, v.params.z);
  let second = readoutBlurSpan(d, -halfWidth, halfWidth, v.params.w);
  var out = readoutOver(v.b, select(0.0, second, v.params.w > 0.0), vec4f(0.0));
  out = readoutOver(v.a, select(0.0, first, v.params.z > 0.0), out);
  return readoutOver(v.ink, ink, out);
}
