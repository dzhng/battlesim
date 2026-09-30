@group(0) @binding(0) var<storage, read> points: array<vec2f>;

struct TerrainParams {
  region: vec4f,
  counts: vec4u,
  shape: vec4f,
  verge: vec4f,
  feathers: vec4f,
  road: vec4f,
  roadDetail: vec4f,
  forestLitter: vec4f,
  forestMoss: vec4f,
  forestHumus: vec4f,
  forestDetail: vec4f,
  forestVerge: vec4f,
  forestDapple: vec4f,
  waterBed: vec4f,
  water: vec4f,
  shore: vec4f,
  distant: vec4f,
}

@group(1) @binding(0) var<uniform> params_1: TerrainParams;

fn pcgHash(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}

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

fn rectInside(xy: vec2f, r: vec4f) -> f32 {
  return min(min((xy.x - r.x), (r.z - xy.x)), min((xy.y - r.y), (r.w - xy.y)));
}

struct PlotNode {
  line: vec4f,
  children: vec4i,
}

@group(1) @binding(1) var<storage, read> nodes: array<PlotNode>;

struct RoadSegment {
  ends: vec4f,
  half: vec4f,
}

@group(1) @binding(3) var<storage, read> roads: array<RoadSegment>;

@group(1) @binding(4) var<storage, read> rects: array<vec4f>;

fn groundForest(xy: vec2f) -> f32 {
  var forest = -1e+9f;
  for (var i = 0u; (i < params_1.counts.y); i++) {
    forest = max(forest, rectInside(xy, rects[i]));
  }
  return forest;
}

fn groundSite(xy: vec2f) -> vec4f {
  let params = (&params_1);
  let warp = vec2f((valueNoise((xy * (*params).shape.y)) - 0.5f), (valueNoise(((xy * (*params).shape.y) + vec2f(19.700000762939453, 5.300000190734863))) - 0.5f));
  let q = (xy + (warp * ((*params).shape.x * 2f)));
  var edge = rectInside(q, (*params).region);
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
  var road = -1e+9f;
  for (var i = 0u; (i < (*params).counts.x); i++) {
    let seg = (&roads[i]);
    let a = (*seg).ends.xy;
    let ab = ((*seg).ends.zw - a);
    let t = clamp((dot((xy - a), ab) / max(dot(ab, ab), 1e-6f)), 0f, 1f);
    let off = length((xy - (a + (ab * t))));
    road = max(road, ((*seg).half.x - off));
  }
  return vec4f(f32(leaf), edge, road, groundForest(xy));
}

@group(0) @binding(1) var<storage, read_write> output: array<vec4f>;

@compute @workgroup_size(1) fn item(@builtin(global_invocation_id) gid: vec3u) {
        let xy=points[gid.x];let site=groundSite(xy);output[gid.x]=vec4f(xy,site.z,site.w);
      }