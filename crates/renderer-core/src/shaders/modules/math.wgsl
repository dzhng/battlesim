// WGSL owner: math.

// @block math_placedPoint
fn placedPoint(local: vec3f, pose: vec4f) -> vec3f {
  let c = cos(pose.w);
  let s = sin(pose.w);
  return vec3f((((local.x * c) - (local.y * s)) + pose.x), (((local.x * s) + (local.y * c)) + pose.y), (local.z + pose.z));
}

// @block math_LUMA
const LUMA = vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B);

// @block math_atLeast
fn atLeast(colour: vec3f, least: f32) -> vec3f {
  let luminance = dot(colour, LUMA);
  return (colour * max(1f, (least / max(luminance, 1e-5f))));
}

// @block math_polygonEdgeDistance
fn polygonEdgeDistance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
 let ab=b-a;let len2=dot(ab,ab);var t=0.0;if(len2>0.0){t=clamp(dot(p-a,ab)/len2,0.0,1.0);}return length(p-(a+ab*t));
}

// @block math_polygonTriangleInside
fn polygonTriangleInside(xy: vec2f, a: vec2f, b: vec2f, c: vec2f) -> bool {
 let ab=b-a;let bc=c-b;let ca=a-c;
 let area=dot(vec2f(-ab.y,ab.x),c-a);
 let s0=dot(vec2f(-ab.y,ab.x),xy-a);let s1=dot(vec2f(-bc.y,bc.x),xy-b);let s2=dot(vec2f(-ca.y,ca.x),xy-c);
 return area!=0.0&&((min(s0,min(s1,s2))>=0.0)||(max(s0,max(s1,s2))<=0.0));
}

// @block math_radicalInverse
fn radicalInverse(i: u32) -> f32 {
  var b=(i<<16u)|(i>>16u);
  b=((b&0x55555555u)<<1u)|((b&0xAAAAAAAAu)>>1u);
  b=((b&0x33333333u)<<2u)|((b&0xCCCCCCCCu)>>2u);
  b=((b&0x0F0F0F0Fu)<<4u)|((b&0xF0F0F0F0u)>>4u);
  b=((b&0x00FF00FFu)<<8u)|((b&0xFF00FF00u)>>8u);
  return f32(b)*2.3283064365386963e-10;
}

// @block math_rectInside
fn rectInside(xy: vec2f, r: vec4f) -> f32 {
  return min(min((xy.x - r.x), (r.z - xy.x)), min((xy.y - r.y), (r.w - xy.y)));
}

// @block math_srgbToLinear
fn srgbToLinear(c: vec3f) -> vec3f {
  return pow(max(c, vec3f()), vec3f(2.200000047683716));
}

// @block math_strokeCutInside
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

// @block math_triangleRuleHeight
fn triangleRuleHeight(h00: f32, h10: f32, h01: f32, h11: f32, u: f32, v: f32) -> f32 {
  if (u >= v) { return h00 + u * (h10 - h00) + v * (h11 - h10); }
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}

// @block math_turned
fn turned(xy: vec2f, turn: vec2f) -> vec2f {
  return vec2f(((xy.x * turn.x) + (xy.y * turn.y)), ((xy.y * turn.x) - (xy.x * turn.y)));
}

// @block math_coveredSamples
fn coveredSamples(coverage: f32, edge: f32, noise: f32) -> u32 {
    let threshold = mix(noise, 0.5, clamp(edge * 2.0, 0.0, 1.0));
    let samples = u32(clamp(floor(coverage * 4.0 + threshold), 0.0, 4.0));
    return (1u << samples) - 1u;
  }

// @block math_crossing
fn crossing(x: f32, edge: f32, width: f32) -> f32 {
  let w = max(width, 1e-3);
  return smoothstep(edge - w, edge + w, x);
}

// @block math_fbm
fn fbm(p0: vec2f) -> f32 {
  // Warped, so masses round into heaps rather than streaks.
  var p = p0 + vec2f(latticeNoise(p0 * 0.5), latticeNoise(p0 * 0.5 + vec2f(5.2, 1.3))) * 0.9;
  var sum = 0.0;
  var amp = 0.55;
  var norm = 0.0;
  for (var k = 0; k < 5; k++) {
    sum += amp * latticeNoise(p);
    norm += amp;
    p = vec2f(p.x * 1.7 - p.y * 1.1, p.x * 1.1 + p.y * 1.7) + vec2f(17.3, 9.1);
    amp *= 0.45;
  }
  return sum / norm;
}

// @block math_latticeHash
fn latticeHash(i: vec2i) -> f32 {
  var h = (bitcast<u32>(i.x) * 0x8da6b343u) ^ (bitcast<u32>(i.y) * 0xd8163841u);
  h = h ^ (h >> 13u);
  h = h * 0x5bd1e995u;
  h = h ^ (h >> 15u);
  return f32(h & 0xffffffu) / 16777216.0;
}

// @block math_latticeNoise
fn latticeNoise(p: vec2f) -> f32 {
  let i = vec2i(floor(p));
  let f = fract(p);
  let u = f * f * (3.0 - 2.0 * f);
  return mix(mix(latticeHash(i), latticeHash(i + vec2i(1, 0)), u.x), mix(latticeHash(i + vec2i(0, 1)), latticeHash(i + vec2i(1, 1)), u.x), u.y);
}

// @block math_pcgHash
fn pcgHash(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}

// @block math_pixelNoise
fn pixelNoise(pixel: vec2f) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2f(0.06711056, 0.00583715))));
  }

// @block math_valueNoise
fn valueNoise(p: vec2f) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let u = f * f * (3.0 - 2.0 * f);
  let ix = bitcast<u32>(i32(cell.x));
  let iy = bitcast<u32>(i32(cell.y));
  var h = array<f32, 4>();
  for (var k = 0u; k < 4u; k++) {
    let s = pcgHash(((ix + (k & 1u)) * 1597334677u) ^ ((iy + (k >> 1u)) * 3812015801u));
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  return mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
}

// @block math_valueNoise3
fn valueNoise3(p: vec3f) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let u = f * f * (3.0 - 2.0 * f);
  let i = vec3u(bitcast<vec3u>(vec3i(cell)));
  var h = array<f32, 8>();
  for (var k = 0u; k < 8u; k++) {
    let q = i + vec3u(k & 1u, (k >> 1u) & 1u, k >> 2u);
    let s = pcgHash((q.x * 1597334677u) ^ (q.y * 3812015801u) ^ (q.z * 2798796415u));
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  let x0 = mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
  let x1 = mix(mix(h[4], h[5], u.x), mix(h[6], h[7], u.x), u.y);
  return mix(x0, x1, u.z);
}

// @block math_faceEyeNormal
fn faceEyeNormal(normal: vec3f, towardEye: vec3f) -> vec3f {
  var n = normal;
  if ((dot(n, towardEye) < 0f)) {
    n = -(n);
  }
  return n;
}
