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

struct SurfaceRecord {
  ends: vec4f,
  detail: vec4f,
}

@group(1) @binding(3) var<storage, read> surfaces: array<SurfaceRecord>;

fn polygonEdgeDistance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
 let ab=b-a;let len2=dot(ab,ab);var t=0.0;if(len2>0.0){t=clamp(dot(p-a,ab)/len2,0.0,1.0);}return length(p-(a+ab*t));
}

fn pavedSurfaceDistance(xy: vec2f) -> f32 {
 let P=params_1;var paved=-1e9;
 for(var i=0u;i<P.counts.x;i++){
  let seg=surfaces[i];
  let a=seg.ends.xy;let ab=seg.ends.zw-a;
  let t=clamp(dot(xy-a,ab)/max(dot(ab,ab),1e-6),0.0,1.0);
  let off=length(xy-(a+ab*t));paved=max(paved,seg.detail.x-off);
 }
 var inside=false;var nearest=1e9;
 for(var i=0u;i<P.counts.w;i++){
  let tri=surfaces[P.counts.x+i];let mask=u32(tri.detail.z);
  let a=tri.ends.xy;let b=tri.ends.zw;let c=tri.detail.xy;
  let ab=b-a;let bc=c-b;let ca=a-c;
  let area=dot(vec2f(-ab.y,ab.x),c-a);
  let s0=dot(vec2f(-ab.y,ab.x),xy-a);let s1=dot(vec2f(-bc.y,bc.x),xy-b);let s2=dot(vec2f(-ca.y,ca.x),xy-c);
  inside=inside||(area!=0.0&&((min(s0,min(s1,s2))>=0.0)||(max(s0,max(s1,s2))<=0.0)));
  if((mask&1u)!=0u){nearest=min(nearest,polygonEdgeDistance(xy,a,b));}
  if((mask&2u)!=0u){nearest=min(nearest,polygonEdgeDistance(xy,b,c));}
  if((mask&4u)!=0u){nearest=min(nearest,polygonEdgeDistance(xy,c,a));}
  if((mask&8u)!=0u){paved=max(paved,select(-nearest,nearest,inside));inside=false;nearest=1e9;}
 }
 return paved;
}

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
  return vec4f(f32(leaf), edge, pavedSurfaceDistance(xy), groundForest(xy));
}

struct PlotRecord {
  colour: vec4f,
  rows: vec4f,
  detail: vec4f,
}

@group(1) @binding(2) var<storage, read> plots: array<PlotRecord>;

fn mottle(xy: vec2f, footprint: f32, across: vec2f, plot: f32) -> vec2f {
  let shape = (&params_1.shape);
  let along = vec2f(-(across.y), across.x);
  let lane = vec2f((dot(xy, along) * (*shape).w), ((dot(xy, across) * (*shape).w) * 16f));
  let seed = vec2f((fract((plot * 0.6180339f)) * 997f), (fract((plot * 0.7548776f)) * 991f));
  let at = (lane + seed);
  let streak = valueNoise(at);
  let slope = vec2f((((valueNoise((at + vec2f(0.05000000074505806, 0))) - streak) / 0.05f) * (*shape).w), ((((valueNoise((at + vec2f(0, 0.05000000074505806))) - streak) / 0.05f) * (*shape).w) * 16f));
  let inside = ((streak - 0.58f) / max(length(slope), 1e-4f));
  let edge = (max(footprint, 0.3f) * 0.5f);
  let strip = smoothstep(-(edge), edge, inside);
  let fine = (valueNoise(((xy * (*shape).z) + vec2f(37.099998474121094, 11.300000190734863))) - 0.5f);
  let fineShown = (1f - smoothstep(0.25f, 1f, (footprint * (*shape).z)));
  return vec2f(((fine * 0.45f) * fineShown), ((strip * 0.6f) + ((max(fine, 0f) * 0.4f) * fineShown)));
}

fn groundTint(albedo: vec3f, dry: f32) -> vec3f {
  let luma = vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023);
  let tinted = (albedo * (vec3f(1) + (vec3f(0.699999988079071, 0, -0.8999999761581421) * dry)));
  return (tinted * (dot(albedo, luma) / max(dot(tinted, luma), 1e-5f)));
}

fn groundVerge(site: vec4f, footprint: f32) -> f32 {
  let params = (&params_1);
  let vergeEdge = min(site.y, max(-(site.z), 0f));
  let vergeHalf = (*params).verge.w;
  return (1f - smoothstep(vergeHalf, (vergeHalf + max((*params).feathers.x, footprint)), vergeEdge));
}

fn forestVergeInside(xy: vec2f, forest: f32) -> f32 {
  let verge = (&params_1.forestVerge);
  let wander = ((valueNoise(((xy * (*verge).z) + vec2f(71.30000305175781, 23.899999618530273))) - 0.5f) * 2f);
  let patches = (valueNoise(((xy * 0.7f) + vec2f(5.099999904632568, 91.69999694824219))) - 0.5f);
  return (((forest + ((*verge).x * 0.5f)) + (wander * (*verge).y)) + (patches * (*verge).x));
}

fn forestFloorWeight(xy: vec2f, forest: f32, footprint: f32) -> f32 {
  let feather = max(footprint, (params_1.forestVerge.x * 0.1f));
  return smoothstep(-(feather), feather, forestVergeInside(xy, forest));
}

fn forestFloor(xy: vec2f, footprint: f32, noise: f32) -> vec3f {
  let params = (&params_1);
  let detail = (&(*params).forestDetail);
  let at = (xy * (*detail).x);
  let moss = smoothstep(0.45f, 0.7f, valueNoise((at + vec2f(13.300000190734863, 7.699999809265137))));
  let humus = smoothstep(0.5f, 0.75f, valueNoise(((at * 2.3f) + vec2f(2.9000000953674316, 41.099998474121094))));
  var floor_1 = mix((*params).forestLitter.xyz, (*params).forestMoss.xyz, moss);
  floor_1 = mix(floor_1, (*params).forestHumus.xyz, (humus * 0.8f));
  let rootsAt = (xy * (*params).forestVerge.w);
  let ridge = abs((valueNoise((rootsAt + vec2f(29.100000381469727, 3.299999952316284))) - 0.5f));
  let runs = smoothstep(0.55f, 0.75f, valueNoise(((rootsAt * 1.7f) + vec2f(8.300000190734863, 17.899999618530273))));
  let shown = (1f - smoothstep(0.02f, 0.08f, (footprint * (*params).forestVerge.w)));
  let line = ((1f - smoothstep(0.015f, 0.035f, ridge)) * runs);
  let root = ((line * shown) + (0.02f * (1f - shown)));
  floor_1 = (floor_1 * (1f - ((*detail).w * root)));
  return (floor_1 * (1f + ((*detail).y * noise)));
}

fn groundShore(xy: vec2f, water: f32) -> f32 {
  let params = (&params_1);
  let reach = ((*params).shore.w * (0.55f + (0.6f * valueNoise((xy * 0.4f)))));
  return (1f - smoothstep((reach * 0.55f), reach, -(water)));
}

fn groundColour(xy: vec2f, footprint: f32, site: vec4f, water: f32) -> vec4f {
  let params = (&params_1);
  let aa = (footprint * 0.5f);
  let plot = (&plots[i32(site.x)]);
  let variation = mottle(xy, footprint, (*plot).rows.xy, site.x);
  let noise = variation.x;
  var albedo = groundTint(((*plot).colour.xyz * (1f + ((*plot).detail.x * noise))), ((*plot).detail.x * variation.y));
  var roughness = (*plot).colour.w;
  let period = (*plot).rows.z;
  if ((period > 0f)) {
    let phase = (dot(xy, (*plot).rows.xy) / period);
    let stripe = (0.5f + (0.5f * cos((phase * 6.2831853f))));
    let shown = (1f - smoothstep(0.25f, 0.6f, (footprint / period)));
    albedo = (albedo * (1f - ((*plot).rows.w * (0.5f + ((stripe - 0.5f) * shown)))));
  }
  let verge = groundVerge(site, footprint);
  albedo = mix(albedo, ((*params).verge.xyz * (1f + (0.18f * noise))), verge);
  roughness = mix(roughness, 0.95f, verge);
  let inRegion = rectInside(xy, (*params).region);
  let distant = max((1f - smoothstep(0f, 600f, inRegion)), smoothstep((*params).feathers.z, (*params).feathers.w, footprint));
  albedo = mix(albedo, (*params).distant.xyz, distant);
  let forest = forestFloorWeight(xy, site.w, footprint);
  if ((forest > 0f)) {
    albedo = mix(albedo, forestFloor(xy, footprint, noise), forest);
    roughness = mix(roughness, (*params).forestDetail.z, forest);
  }
  let wet = groundShore(xy, water);
  albedo = mix(albedo, ((*params).shore.xyz * (1f + (0.15f * noise))), wet);
  roughness = mix(roughness, 0.6f, wet);
  let roadFeather = (max((*params).feathers.y, footprint) * 0.5f);
  let onRoad = smoothstep(-(roadFeather), roadFeather, site.z);
  let surface = ((*params).road.xyz * (1f + ((*params).roadDetail.x * noise)));
  albedo = mix(albedo, surface, onRoad);
  roughness = mix(roughness, (*params).road.w, onRoad);
  let bed = smoothstep(-(aa), aa, water);
  albedo = mix(albedo, (*params).waterBed.xyz, bed);
  return vec4f(max(albedo, vec3f()), roughness);
}

@group(0) @binding(1) var<storage, read_write> output: array<vec4f>;

@compute @workgroup_size(1) fn item(@builtin(global_invocation_id) gid: vec3u) {
        let xy=points[gid.x];let site=groundSite(xy);let actual=groundColour(xy,0.1,site,-1e9);let reference=groundColour(xy,0.1,vec4f(site.xy,5.0,site.w),-1e9);output[gid.x]=vec4f(site.z,actual.w,reference.w,site.w);
      }