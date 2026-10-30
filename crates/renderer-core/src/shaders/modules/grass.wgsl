// WGSL owner: grass.

// @block grass_GrassClump
struct GrassClump {
  root: vec3f,
  height: f32,
  colour: u32,
  kind: u32,
  yawSin: f32,
  yawCos: f32,
  width: f32,
}

// @block grass_GrassShapeVertex
struct GrassShapeVertex {
  spine: vec4f,
  side: vec4f,
  normal: vec4f,
  tint: vec4f,
}

// @block grass_GrassVertex
struct GrassVertex {
  clip: vec4f,
  world: vec3f,
  root: vec3f,
  normal: vec3f,
  albedo: vec3f,
  ground: f32,
}

// @block grass_ScarParams
struct ScarParams {
  grid: vec4f,
  pages: vec4u,
  clip: vec4f,
  cacheBounds: vec4u,
  defaultWord: vec4u,
  full: vec4f,
  crater: vec4f,
  ejecta: vec4f,
  scorch: vec4f,
  tracks: vec4f,
  trampled: vec4f,
  relief: vec4f,
  grass: vec4f,
}

// @block grass_clumps_grassDrawLayout
@group(1) @binding(2) var<storage, read> clumps: array<GrassClump>;

// @block grass_clumps_grassBuildLayout
@group(0) @binding(3) var<storage, read_write> clumps: array<GrassClump>;

// @block grass_grassGround
fn grassGround(q: vec2f) -> f32 {
  let P = &params;
  let nx = (*P).counts.x;
  let ny = (*P).counts.y;
  let fx = clamp(q.x / (*P).ground.x, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / (*P).ground.x, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u);
  let j = min(u32(fy), ny - 2u);
  return triangleRuleHeight(
    grassSample(i, j), grassSample(i + 1u, j),
    grassSample(i, j + 1u), grassSample(i + 1u, j + 1u),
    fx - f32(i), fy - f32(j));
}

// @block grass_grassHash
fn grassHash(a: u32, b: u32) -> vec3f {
  var s = a * 747796405u + b * 2891336453u + 12345u;
  var out = vec3f(0.0);
  for (var k = 0u; k < 3u; k++) {
    out[k] = f32(pcgHash(s) >> 8u) * (1.0 / 16777216.0);
    s = s * 747796405u + 2891336453u;
  }
  return out;
}

// @block grass_grassPaintGlow
fn grassPaintGlow(paint: vec4f, height: f32) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  let s = paintStyle;
  let colour = paintColour(paint);
  return colour * s.grassGlow * paint.w * exp(-max(height, 0.0) / max(s.grassFalloffM, 1e-3));
}

// @block grass_grassSample
fn grassSample(i: u32, j: u32) -> f32 {
    let size = readWord(0u);
    let cols = readWord(1u);
    let count = readWord(2u);
    if (count == 0u) { return 0.0; }
    let page = readWord(3u + (j / size) * cols + i / size);
    if (page == 0u) { return 0.0; }
    let at = 3u + count + (page - 1u) * size * size + (j % size) * size + i % size;
    return bitcast<f32>(readWord(at));
  }

// @block grass_grassUnderProp
fn grassUnderProp(p: vec2f) -> bool {
  let grid = props[0];
  let size = bitcast<vec4u>(props[1]);
  let g = (p - grid.xy) / grid.z;
  if (g.x < 0.0 || g.y < 0.0) { return false; }
  let c = vec2u(g);
  if (c.x >= size.x || c.y >= size.y) { return false; }
  let cell = c.y * size.x + c.x;
  let pair = bitcast<vec4u>(props[2u + cell / 2u]);
  let run = select(pair.xy, pair.zw, (cell & 1u) == 1u);
  for (var i = 0u; i < run.y; i++) {
    let a = props[run.x + 2u * i];
    let b = props[run.x + 2u * i + 1u];
    let o = p - a.xy;
    if (abs(dot(o, a.zw)) < b.x && abs(dot(o, vec2f(-a.w, a.z))) < b.y) { return true; }
  }
  return false;
}

// @block grass_grassVertex_Output
struct grassVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}

// @block grass_groundShoulderGrass
fn groundShoulderGrass(worn: vec2f) -> f32 {
 return mix(1.0,params_1.roads[u32(worn.y)].edge.z,worn.x);
}

// @block grass_groundSite_alternate
fn groundSite(xy: vec2f, cell: vec4u, paved: GroundPaved) -> vec4f {
  let params_1 = (&params);
  let warp = vec2f((valueNoise((xy * (*params_1).shape.y)) - 0.5f), (valueNoise(((xy * (*params_1).shape.y) + vec2f(19.700000762939453, 5.300000190734863))) - 0.5f));
  let q = (xy + (warp * ((*params_1).shape.x * 2f)));
  var edge = rectInside(q, (*params_1).region);
  var node = 0i;
  var leaf = 0i;
  for (var step_1 = 0; (step_1 < 64i); step_1++) {
    let n = (&nodes[node]);
    let s = (dot((*n).line.xy, q) - (*n).line.z);
    edge = min(edge, abs(s));
    var child = (*n).children.y;
    if ((s >= 0f)) {
      child = (*n).children.x;
    }
    if ((child < 0i)) {
      leaf = (-1i - child);
      break;
    }
    node = child;
  }
  return vec4f(f32(leaf), edge, paved.rule, groundForest(xy, cell));
}

// @block grass_leafClumps
fn leafClumps(p: vec3f, n: vec3f, fade: f32) -> vec4f {
  let q = p * 2.1;
  let bend = vec3f(
    valueNoise3(q),
    valueNoise3(q + vec3f(17.3, 5.1, 9.7)),
    valueNoise3(q + vec3f(3.7, 29.3, 13.1)),
  ) - vec3f(0.5);
  let bent = normalize(n + bend * (0.7 * fade));
  let gap = valueNoise3(p * 3.4 + vec3f(7.0, 1.0, 3.0));
  let shade = mix(1.0, 0.72 + 0.4 * gap, fade);
  return vec4f(bent, shade);
}

// @block grass_params_primary
@group(GROUP) @binding(0) var<uniform> params: GrassParams;

// @block grass_shapes
@group(1) @binding(3) var<storage, read> shapes: array<GrassShapeVertex>;

// @block grass_ranges
@group(0) @binding(5) var<storage, read_write> ranges: array<vec2u>;
