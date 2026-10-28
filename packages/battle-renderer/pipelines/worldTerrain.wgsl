struct Wind {
  toward: vec3f,
  driftMps: f32,
  gustMps: f32,
  gustPerM: f32,
}

struct Camera {
  viewProj: mat4x4f,
  invViewProj: mat4x4f,
  eye: vec3f,
  znear: f32,
  width: f32,
  height: f32,
  time: f32,
  wind: array<Wind, 1>,
  waves: vec4f,
}

@group(0) @binding(0) var<uniform> cam: Camera;

struct meshVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

@vertex fn meshVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> meshVertex_Output {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let local = position;
  let world = vec3f((((local.x * c) - (local.y * s)) + placement.x), (((local.x * s) + (local.y * c)) + placement.y), (local.z + placement.z));
  let normal_1 = vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z);
  return meshVertex_Output((cam.viewProj * vec4f(world, 1f)), world, normal_1, vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

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

@group(1) @binding(5) var<uniform> scarParams: ScarParams;

fn scarRegionContains(world: vec2f) -> bool {
  let r = (&scarParams.clip);
  return (((*r).z <= (*r).x) || ((((world.x >= (*r).x) && (world.y >= (*r).y)) && (world.x < (*r).z)) && (world.y < (*r).w)));
}

struct FogLayer {
  ground: u32,
  seen: u32,
  painted: u32,
  pad: u32,
}

@group(2) @binding(1) var<uniform> layer: FogLayer;

fn fogIsGround() -> bool {
  return (layer.ground == 1u);
}

struct FogParams {
  invViewProj: mat4x4f,
  cameraEye: vec3f,
  translucentLiftM: f32,
  azimuthBins: u32,
  terrainAzimuthBins: u32,
  radialBins: u32,
  tilePx: u32,
  tilesX: u32,
  tileEyesMax: u32,
  eyeCount: u32,
  foliageNx: u32,
  foliageNy: u32,
  heightNx: u32,
  heightNy: u32,
  rebuildCount: u32,
  enabled: u32,
  probeCount: u32,
  firstBinM: f32,
  targetHeightM: f32,
  foliageCellM: f32,
  foliageFullBlock: f32,
  minSightGapM: f32,
  playable: vec4f,
  heightSpacing: f32,
  faceProbeM: f32,
  width: f32,
  height: f32,
  stepMinM: f32,
  stepMaxM: f32,
  stepFraction: f32,
  roofReachM: f32,
  wholeCount: u32,
  wholeNx: u32,
  wholeNy: u32,
  wholeCellM: f32,
  wholeOrigin: vec2f,
  wholeItemsBase: u32,
  wholeBoxesBase: u32,
  wholeFlagsBase: u32,
}

@group(2) @binding(0) var<uniform> params: FogParams;

@group(2) @binding(8) var wholes: texture_2d<u32>;

fn fogWholeWord(i: u32) -> u32 {
  return textureLoad(wholes, vec2u(i % 256u, i / 256u), 0).x;
}

fn fogWholeSeen(p: vec3f) -> bool {
  let P = params;
  if (P.wholeCount == 0u) { return false; }
  let c = (p.xy - P.wholeOrigin) / P.wholeCellM;
  if (c.x < 0.0 || c.y < 0.0 || c.x >= f32(P.wholeNx) || c.y >= f32(P.wholeNy)) { return false; }
  let cell = fogWholeWord(u32(c.y) * P.wholeNx + u32(c.x));
  let start = cell >> 8u;
  let n = cell & 0xffu;
  let m = P.faceProbeM * 2.0;
  for (var k = 0u; k < n; k++) {
    let b = fogWholeWord(P.wholeItemsBase + start + k);
    if (fogWholeWord(P.wholeFlagsBase + b) == 0u) { continue; }
    let o = P.wholeBoxesBase + b * 8u;
    let q = p.xy - vec2f(bitcast<f32>(fogWholeWord(o)), bitcast<f32>(fogWholeWord(o + 1u)));
    let cs = bitcast<f32>(fogWholeWord(o + 2u));
    let sn = bitcast<f32>(fogWholeWord(o + 3u));
    let local = vec2f(q.x * cs + q.y * sn, -q.x * sn + q.y * cs);
    let half = vec2f(bitcast<f32>(fogWholeWord(o + 4u)), bitcast<f32>(fogWholeWord(o + 5u)));
    let base = bitcast<f32>(fogWholeWord(o + 6u));
    let top = bitcast<f32>(fogWholeWord(o + 7u));
    if (all(abs(local) <= half + vec2f(m)) && p.z >= base - 2.0 && p.z <= top + 3.0) {
      return true;
    }
  }
  return false;
}

@group(2) @binding(5) var<storage, read> counts: array<u32>;

fn fogProbePoint(world: vec3f, normal: vec3f, ground: bool) -> vec3f {
  if (ground) { return world + vec3f(0.0, 0.0, params.targetHeightM); }
  return world + normal * params.faceProbeM;
}

struct FogEye {
  position: vec3f,
  reach: f32,
  forward: f32,
  front: f32,
  side: f32,
  rear: f32,
  range: f32,
  slot: u32,
}

@group(2) @binding(2) var<storage, read> eyes: array<FogEye>;

fn fogShape(front: f32, side: f32, rear: f32, off: f32) -> f32 {
  let c = cos(off);
  let end = select(rear, front, c >= 0.0);
  return side * (1.0 - c * c) + end * c * c;
}

fn fogBinEdge(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}

fn fogUnpack(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * 0.005);
}

@group(2) @binding(3) var<storage, read> maps: array<u32>;

fn fogSeenBy(ei: u32, p: vec3f, n: vec3f) -> bool {
  let e = eyes[ei];
  let P = params;
  if (p.x < P.playable.x || p.y < P.playable.y ||
      p.x > P.playable.x + P.playable.z || p.y > P.playable.y + P.playable.w) {
    // The visual margin is outside the simulation. Keep it permanently unseen
    // and avoid all range, horizon and foliage work for it.
    return false;
  }
  let face = dot(n, n) > 0.0;
  if (face && dot(n, e.position - p) <= 0.0) { return false; }
  let dxy = p.xy - e.position.xy;
  let dist = length(dxy);
  if (dist < P.firstBinM) { return true; }
  let theta = atan2(dxy.y, dxy.x);
  let range = e.range * fogShape(e.front, e.side, e.rear, theta - e.forward);
  if (dist > range) { return false; }
  let AZ = P.azimuthBins;
  let R = P.radialBins;
  let lnr = log(e.reach / P.firstBinM);
  let k = min(u32(ceil(log(dist / P.firstBinM) / lnr * f32(R - 1u))), R - 1u);
  let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
  let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
  let a = (theta + 3.14159265) / 6.28318531 * f32(AZ) - 0.5;
  let af = floor(a);
  let fa = a - af;
  let i0 = u32(i32(af) + i32(AZ)) % AZ;
  let i1 = (i0 + 1u) % AZ;
  let w0 = (e.slot * AZ + i0) * R + k;
  let w1 = (e.slot * AZ + i1) * R + k;
  let c0 = fogUnpack(maps[w0]);
  let c1 = fogUnpack(maps[w1]);
  var p0 = vec3f(-1e4, 0.0, 0.0);
  var p1 = vec3f(-1e4, 0.0, 0.0);
  if (k > 0u) {
    p0 = fogUnpack(maps[w0 - 1u]);
    p1 = fogUnpack(maps[w1 - 1u]);
  }
  let t0 = mix(lo, hi, c0.y);
  let t1 = mix(lo, hi, c1.y);
  let eps = 0.1 + (hi - lo) / 255.0;
  var tanInc = 0.0;
  if (face && length(n.xy) > 1e-3) {
    let c = clamp(abs(dot(dxy / dist, normalize(n.xy))), 0.017, 1.0);
    tanInc = sqrt(1.0 - c * c) / c;
  }
  let spread = 1.0 + 0.01 * dist + 1.5 * dist * (6.28318531 / f32(AZ)) * tanInc;
  var horizon = 0.0;
  if (abs(t0 - t1) < spread) {
    let tj = mix(t0, t1, fa);
    horizon = select(mix(p0.x, p1.x, fa), mix(c0.x, c1.x, fa), dist > tj + eps);
  } else {
    let h0 = select(p0.x, c0.x, dist > t0 + eps);
    let h1 = select(p1.x, c1.x, dist > t1 + eps);
    horizon = mix(h0, h1, fa);
  }
  let u = clamp((dist - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
  let foliage = mix(mix(p0.z, c0.z, u), mix(p1.z, c1.z, u), fa);
  if (foliage >= P.foliageFullBlock) { return false; }
  if (dist > range * exp(-foliage)) { return false; }
  return (p.z - e.position.z) / dist >= horizon;
}

fn fogSeenSurface(ei: u32, world: vec3f, normal: vec3f, ground: bool) -> bool {
  let p = fogProbePoint(world, normal, ground);
  let n = select(normal, vec3f(0.0), ground);
  if (fogSeenBy(ei, p, n)) { return true; }
  let e = eyes[ei];
  let P = params;
  if (ground || P.roofReachM <= 0.0 || normal.z < 0.7 || world.z <= e.position.z) {
    return false;
  }
  let dxy = world.xy - e.position.xy;
  let dist = length(dxy);
  if (dist <= P.firstBinM) { return true; }
  let range = e.range * fogShape(e.front, e.side, e.rear, atan2(dxy.y, dxy.x) - e.forward);
  if (dist > range) { return false; }
  let pull = min(P.roofReachM, dist - P.firstBinM);
  let q = vec3f(world.xy - dxy / dist * pull, world.z + P.faceProbeM);
  return fogSeenBy(ei, q, vec3f(0.0));
}

@group(2) @binding(4) var<storage, read> lists: array<u32>;

fn fogTerm(world: vec3f, normal: vec3f, pixel: vec2f, isGround: bool) -> f32 {
  let P = params;
  if (P.enabled == 0u || layer.seen == 1u) { return 1.0; }
  if (fogWholeSeen(world)) { return 1.0; }
  let tile = u32(pixel.x) / P.tilePx + (u32(pixel.y) / P.tilePx) * P.tilesX;
  let count = min(counts[tile], P.tileEyesMax);
  for (var s = 0u; s < count; s++) {
    if (fogSeenSurface(lists[tile * P.tileEyesMax + s], world, normal, isGround)) {
      return 1.0;
    }
  }
  return 0.0;
}

struct RoadLook {
  core: vec4f,
  worn: vec4f,
  shape: vec4f,
  join: vec4f,
  shoulder: vec4f,
  edge: vec4f,
  ruts: vec4f,
  track: vec4f,
  slabs: vec4f,
  curb: vec4f,
  paint: vec4f,
  marks: vec4f,
  crossing: vec4f,
  stones: vec4f,
}

struct ShoreTuning {
  bands: vec4f,
  turns: vec4f,
  ground: vec4f,
  fade: vec4f,
}

struct WaterLook {
  channel: vec4f,
  edge: vec4f,
  surface: vec4f,
  normals: vec4f,
  drifts: vec4f,
  fades: vec4f,
  roughness: vec4f,
  lobes: vec4f,
  refraction: vec4f,
  caustics: vec4f,
}

struct TerrainParams {
  region: vec4f,
  field: vec4f,
  fieldGrid: vec4u,
  shape: vec4f,
  verge: vec4f,
  feathers: vec4f,
  roads: array<RoadLook, 4>,
  roadOrder: vec4u,
  forestLitter: vec4f,
  forestMoss: vec4f,
  forestHumus: vec4f,
  forestDetail: vec4f,
  forestVerge: vec4f,
  forestDapple: vec4f,
  forestLine: vec4f,
  waterBed: vec4f,
  shore: vec4f,
  shoreEarth: vec4f,
  shoreEdge: vec4f,
  distant: vec4f,
  shoreTuning: ShoreTuning,
  waterLooks: array<WaterLook, 8>,
  view: vec4u,
}

@group(1) @binding(0) var<uniform> params_1: TerrainParams;

@group(1) @binding(4) var<storage, read> surfaceIndex: array<u32>;

fn groundCell(xy: vec2f, footprint: f32) -> vec4u {
 let P=params_1;
 var level=0u;var widest=P.field.w;
 while(footprint>widest&&level+1u<P.fieldGrid.z){widest*=2.0;level++;}
 let shift=surfaceIndex[level*2u+1u];
 let finest=clamp(vec2i(floor((xy-P.field.xy)*P.field.z)),vec2i(0),vec2i(P.fieldGrid.xy)-1);
 let cell=vec2u(finest)>>vec2u(shift);
 let cols=((P.fieldGrid.x-1u)>>shift)+1u;
 let at=surfaceIndex[level*2u]+(cell.y*cols+cell.x)*3u;
 return vec4u(surfaceIndex[at],surfaceIndex[at+1u],surfaceIndex[at+2u],surfaceIndex[at+3u]);
}

struct GroundPaved {
  drawn: vec4f,
  lane: vec4f,
  run: vec4f,
  rule: f32,
  list: vec2u,
  bearing: f32,
}

struct SurfaceRecord {
  ends: vec4f,
  detail: vec4f,
}

@group(1) @binding(3) var<storage, read> surfaces: array<SurfaceRecord>;

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

fn polygonTriangleInside(xy: vec2f, a: vec2f, b: vec2f, c: vec2f) -> bool {
 let ab=b-a;let bc=c-b;let ca=a-c;
 let area=dot(vec2f(-ab.y,ab.x),c-a);
 let s0=dot(vec2f(-ab.y,ab.x),xy-a);let s1=dot(vec2f(-bc.y,bc.x),xy-b);let s2=dot(vec2f(-ca.y,ca.x),xy-c);
 return area!=0.0&&((min(s0,min(s1,s2))>=0.0)||(max(s0,max(s1,s2))<=0.0));
}

fn polygonEdgeDistance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
 let ab=b-a;let len2=dot(ab,ab);var t=0.0;if(len2>0.0){t=clamp(dot(p-a,ab)/len2,0.0,1.0);}return length(p-(a+ab*t));
}

fn groundPaved(xy: vec2f, cell: vec4u) -> GroundPaved {
 var paved=vec4f(-1e9);var member=4u;var nearest=1e9;var edge=0u;
 var lane=vec4f(0.0);var run=vec4f(0.0,0.0,0.0,-1.0);var deepest=-1e9;var bearing=-1.0;
 for(var i=cell.x;i<cell.y;i++){
  let entry=surfaceIndex[i];
  let seg=surfaces[entry&536870911u];
  let kind=entry>>30u;
  if(kind==1u){
   let a=seg.ends.xy;let ab=seg.ends.zw-a;
   let t=clamp(dot(xy-a,ab)/max(dot(ab,ab),1e-6),0.0,1.0);
   let away=xy-(a+ab*t);let off=length(away);let own=u32(seg.detail.y);
   let inside=strokeCutInside(xy,seg.ends,seg.detail,seg.detail.x-off);
   paved[own]=max(paved[own],inside);
   let walk=params_1.roads[own].join.zw;
   if(walk.x>0.0){paved[u32(walk.y)]=max(paved[u32(walk.y)],inside+walk.x);}
   if(inside>deepest){
    deepest=inside;lane=vec4f(away/max(off,1e-5),off,seg.detail.x);
    let len=max(length(ab),1e-6);run=vec4f(ab/len,seg.detail.w+t*len,f32(own));
   }
  }
  else if(kind==2u){
   if(polygonTriangleInside(xy,seg.ends.xy,seg.ends.zw,seg.detail.xy)&&u32(seg.detail.z)<=member){
    member=u32(seg.detail.z);bearing=seg.detail.w;
   }
  }
  else{
   let off=polygonEdgeDistance(xy,seg.ends.xy,seg.ends.zw);
   if(off<nearest){nearest=off;edge=u32(seg.detail.x);}
  }
 }
 var rule=deepest;
 if(member<4u){
  paved[member]=max(paved[member],nearest);rule=max(rule,nearest);
 }
 else if(nearest<1e9){
  paved[edge]=max(paved[edge],-nearest);rule=max(rule,-nearest);
 }
 return GroundPaved(paved,lane,run,rule,cell.xy,bearing);
}

fn groundClassView() -> bool {
  return (params_1.view.x == 1u);
}

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

fn groundForest(xy: vec2f, cell: vec4u) -> f32 {
 var forest=-1e9;var distance=-1e9;var nearest=1e9;var inside=false;
 for(var i=cell.y;i<cell.z;i++){
  let entry=surfaceIndex[i];
  let record=surfaces[entry&536870911u];
  let kind=entry>>30u;
  if((entry&536870912u)!=0u){
   forest=max(forest,max(distance,select(-nearest,nearest,inside)));
   distance=-1e9;nearest=1e9;inside=false;
  }
  if(kind==0u){forest=max(forest,rectInside(xy,record.ends));}
  else if(kind==1u){distance=max(distance,strokeCutInside(xy,record.ends,record.detail,record.detail.x-polygonEdgeDistance(xy,record.ends.xy,record.ends.zw)));}
  else if(kind==2u){inside=inside||polygonTriangleInside(xy,record.ends.xy,record.ends.zw,record.detail.xy);}
  else{nearest=min(nearest,polygonEdgeDistance(xy,record.ends.xy,record.ends.zw));}
 }
 return max(forest,max(distance,select(-nearest,nearest,inside)));
}

fn groundSite(xy: vec2f, cell: vec4u, paved: GroundPaved) -> vec4f {
  let params_2 = (&params_1);
  let warp = vec2f((valueNoise((xy * (*params_2).shape.y)) - 0.5f), (valueNoise(((xy * (*params_2).shape.y) + vec2f(19.700000762939453, 5.300000190734863))) - 0.5f));
  let q = (xy + (warp * ((*params_2).shape.x * 2f)));
  var edge = rectInside(q, (*params_2).region);
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

fn groundWater(xy: vec2f, cell: vec4u) -> f32 {
 var bed=-1e9;
 for(var i=cell.z;i<cell.w;i++){
  let stretch=surfaces[surfaceIndex[i]&536870911u];
  let a=stretch.ends.xy;let ab=stretch.ends.zw-a;let len2=dot(ab,ab);
  var t=0.0;if(len2>0.0){t=clamp(dot(xy-a,ab)/len2,0.0,1.0);}
  let half=stretch.detail.x+(stretch.detail.y-stretch.detail.x)*t;
  bed=max(bed,half-length(xy-(a+ab*t)));
 }
 return bed;
}

struct PlotRecord {
  colour: vec4f,
  rows: vec4f,
  detail: vec4f,
  grain: vec4f,
  dry: vec4f,
  tram: vec4f,
}

@group(1) @binding(2) var<storage, read> plots: array<PlotRecord>;

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

fn groundTreeLine(xy: vec2f, cell: vec4u) -> vec2f {
 var inside=-1e9;var band=-1e9;
 let taper=params_1.forestLine.x;
 for(var i=cell.y;i<cell.z;i++){
  let entry=surfaceIndex[i];
  if((entry>>30u)!=1u){continue;}
  let record=surfaces[entry&536870911u];
  let here=strokeCutInside(xy,record.ends,record.detail,record.detail.x-polygonEdgeDistance(xy,record.ends.xy,record.ends.zw));
  inside=max(inside,here);
  let cuts=u32(record.detail.z);
  var narrow=here;
  if(cuts!=0u){
   let a=record.ends.xy;let ab=record.ends.zw-a;let len=length(ab);let rel=xy-a;
   let along=dot(rel,ab)/len;
   let end=min(select(1e9,along,(cuts&1u)!=0u),select(1e9,len-along,(cuts&2u)!=0u));
   narrow=min(here,record.detail.x*min(1.0,end/taper)-abs(rel.x*ab.y-rel.y*ab.x)/len);
  }
  band=max(band,narrow);
 }
 return vec2f(inside,band);
}

fn groundLineBand(xy: vec2f, cell: vec4u, forest: f32) -> vec2f {
  let params_2 = (&params_1);
  let line = groundTreeLine(xy, cell);
  if ((line.x < forest)) {
    return vec2f(0, -1000000000);
  }
  let at = ((xy * (*params_2).forestVerge.z) + vec2f(71.30000305175781, 23.899999618530273));
  return vec2f(1f, (line.y + (((valueNoise(at) - 0.5f) * 2f) * (*params_2).forestLine.y)));
}

fn groundFloor(xy: vec2f, footprint: f32, forest: f32) -> vec2f {
  let params_2 = (&params_1);
  let wood = forestFloorWeight(xy, forest, footprint);
  if (((wood > 0f) || (forest > (-((*params_2).forestVerge.x) - (*params_2).forestVerge.y)))) {
    let band = groundLineBand(xy, groundCell(xy, footprint), forest);
    if ((band.x > 0.5f)) {
      let feather = max(footprint, ((*params_2).forestVerge.x * 0.1f));
      return vec2f(0f, smoothstep(-(feather), feather, band.y));
    }
  }
  return vec2f(wood, 0f);
}

fn classDistance(inside: f32) -> f32 {
  return (clamp(round((128f - (inside * 8f))), 1f, 255f) / 255f);
}

fn groundClasses(xy: vec2f, footprint: f32, site: vec4f, water: f32) -> vec3f {
  let plot = (&plots[i32(site.x)]);
  var forest = 0u;
  if ((site.w >= 0f)) {
    forest = 2u;
  }
  else {
    let floor_1 = groundFloor(xy, footprint, site.w);
    if ((max(floor_1.x, floor_1.y) > 0.5f)) {
      forest = 1u;
    }
  }
  let kind = min(u32((*plot).detail.y), 15u);
  let hash = (pcgHash(u32(site.x)) & 3u);
  let packed = (((forest << 6u) | (kind << 2u)) | hash);
  return vec3f(classDistance(site.z), classDistance(water), (f32(packed) / 255f));
}

struct terrainFragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

fn mottle(xy: vec2f, footprint: f32, across: vec2f, plot: f32, reach: vec2f) -> vec2f {
  let shape = (&params_1.shape);
  let along = vec2f(-(across.y), across.x);
  let lane = vec2f((dot(xy, along) * reach.x), (dot(xy, across) * reach.y));
  let seed = vec2f((fract((plot * 0.6180339f)) * 997f), (fract((plot * 0.7548776f)) * 991f));
  let at = (lane + seed);
  let streak = valueNoise(at);
  let slope = vec2f((((valueNoise((at + vec2f(0.05000000074505806, 0))) - streak) / 0.05f) * reach.x), (((valueNoise((at + vec2f(0, 0.05000000074505806))) - streak) / 0.05f) * reach.y));
  let inside = ((streak - 0.58f) / max(length(slope), 1e-4f));
  let edge = (max(footprint, 0.3f) * 0.5f);
  let parched = smoothstep(-(edge), edge, inside);
  let fine = (valueNoise(((xy * (*shape).z) + vec2f(37.099998474121094, 11.300000190734863))) - 0.5f);
  let fineShown = (1f - smoothstep(0.25f, 1f, (footprint * (*shape).z)));
  return vec2f(((fine * 0.45f) * fineShown), ((parched * 0.6f) + ((max(fine, 0f) * 0.4f) * fineShown)));
}

fn groundWheeling(xy: vec2f, footprint: f32, plot: i32) -> f32 {
  let rows = plots[plot].rows;
  let tram = plots[plot].tram;
  if (tram.x <= 0.0 || rows.z <= 0.0) { return 0.0; }
  let phase = dot(xy, rows.xy) / rows.z;
  let furrow = round(phase);
  let nth = furrow - tram.x * floor(furrow / tram.x);
  if (nth != 0.0 && nth != 2.0) { return 0.0; }
  let off = abs(phase - furrow) * rows.z;
  let pixel = max(footprint, 0.02);
  return clamp((tram.y - off) / pixel + 0.5, 0.0, 1.0) * min(1.0, 2.0 * tram.y / pixel);
}

fn fieldTexture(xy: vec2f, footprint: f32, plot: i32) -> f32 {
  let rows = plots[plot].rows;
  let grain = plots[plot].grain;
  let breaks = plots[plot].detail.z;
  let u = dot(xy, vec2f(-rows.y, rows.x));
  let v = dot(xy, rows.xy);
  var value = 1.0;
  if (rows.z > 0.0) {
    let phase = v / rows.z;
    let shown = 1.0 - smoothstep(0.15, 0.45, footprint / rows.z);
    let stripe = 0.5 + 0.5 * cos(phase * 6.2831853);
    var furrow = stripe * stripe * stripe;
    let along = grain.x * grain.z / 2.0;
    let seen = shown * (1.0 - smoothstep(0.3, 0.9, footprint * along));
    if (breaks > 0.0 && seen > 0.0) {
      // Each furrow has its own noise along its length; two meet on the bed
      // between them, where neither darkens anything, so nothing steps.
      let gap = valueNoise(vec2f(u * along, floor(phase + 0.5) * 7.31 + 0.5));
      furrow *= 1.0 + breaks * (gap * 2.0 - 1.0) * seen;
    }
    let bare = plots[plot].tram.z * groundWheeling(xy, footprint, plot);
    value = (1.0 - rows.w * (0.5 + (furrow - 0.3125) * shown)) * (1.0 - bare);
  }
  if (grain.y > 0.0) {
    var size = grain.x / 7.290000000000001;
    var sum = 0.0;
    var push = vec2f(0.0);
    var weights = array<f32, 4>(0.15, 0.25, 0.35, 0.25);
    var turns = array<vec2f, 4>(vec2f(0.8525245220595057, 0.5226872289306592), vec2f(-0.32328956686350335, 0.9463000876874145), vec2f(-0.9709581651495905, 0.23924932921398243), vec2f(-0.40079917207997545, -0.9161659367494549));
    for (var i = 0u; i < 4u; i++) {
      let shown = 1.0 - smoothstep(0.3, 0.9, footprint * size);
      if (shown > 0.0) {
        let turn = turns[i];
        let p = vec2f(u * grain.z, v) * size;
        let seed = vec2f(f32(i) * 17.31 + 3.7, f32(plot % 64) * 1.37);
        let n = valueNoise(vec2f(dot(p, turn), dot(p, vec2f(-turn.y, turn.x))) + seed + push);
        sum += weights[i] * shown * clamp((n - 0.5) * 4.0, -1.0, 1.0);
        push = vec2f(cos(n * 6.2831853), sin(n * 6.2831853)) * 0.45;
      }
      size *= 2.7;
    }
    value *= 1.0 + grain.y * sum;
  }
  return value;
}

fn groundTint(albedo: vec3f, dry: f32) -> vec3f {
  let tinted = (albedo * (vec3f(1) + (vec3f(0.699999988079071, 0, -0.8999999761581421) * dry)));
  return (tinted * (dot(albedo, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)) / max(dot(tinted, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)), 1e-5f)));
}

fn groundVerge(site: vec4f, footprint: f32) -> f32 {
  let params_2 = (&params_1);
  let vergeHalf = (*params_2).verge.w;
  return (1f - smoothstep(vergeHalf, (vergeHalf + max((*params_2).feathers.x, footprint)), site.y));
}

fn forestFloor(xy: vec2f, footprint: f32, noise: f32) -> vec3f {
  let params_2 = (&params_1);
  let detail = (&(*params_2).forestDetail);
  let at = (xy * (*detail).x);
  let turned = vec2f(((at.x * 0.8f) - (at.y * 0.6f)), ((at.x * 0.6f) + (at.y * 0.8f)));
  let across = vec2f(((at.x * 0.28f) + (at.y * 0.96f)), ((at.y * 0.28f) - (at.x * 0.96f)));
  let moss = smoothstep(0.38f, 0.72f, ((valueNoise((turned + vec2f(13.300000190734863, 7.699999809265137))) * 0.65f) + (valueNoise(((across * 2.1f) + vec2f(4.099999904632568, 9.199999809265137))) * 0.35f)));
  let humus = smoothstep(0.42f, 0.78f, ((valueNoise(((across * 1.3f) + vec2f(2.9000000953674316, 41.099998474121094))) * 0.6f) + (valueNoise(((turned * 3.1f) + vec2f(17.299999237060547, 5.900000095367432))) * 0.4f)));
  var floor_1 = mix((*params_2).forestLitter.xyz, (*params_2).forestMoss.xyz, (moss * (*params_2).forestMoss.w));
  floor_1 = mix(floor_1, (*params_2).forestHumus.xyz, (humus * (*params_2).forestHumus.w));
  let rootsAt = (xy * (*params_2).forestVerge.w);
  let ridge = abs((valueNoise((rootsAt + vec2f(29.100000381469727, 3.299999952316284))) - 0.5f));
  let runs = smoothstep(0.55f, 0.75f, valueNoise(((rootsAt * 1.7f) + vec2f(8.300000190734863, 17.899999618530273))));
  let shown = (1f - smoothstep(0.02f, 0.08f, (footprint * (*params_2).forestVerge.w)));
  let line = ((1f - smoothstep(0.015f, 0.035f, ridge)) * runs);
  let root = ((line * shown) + (0.02f * (1f - shown)));
  floor_1 = (floor_1 * (1f - ((*detail).w * root)));
  return (floor_1 * (1f + ((*detail).y * noise)));
}

fn turned(xy: vec2f, turn: vec2f) -> vec2f {
  return vec2f(((xy.x * turn.x) + (xy.y * turn.y)), ((xy.y * turn.x) - (xy.x * turn.y)));
}

fn groundShore(xy: vec2f, footprint: f32, water: f32) -> vec3f {
  let params_2 = (&params_1);
  let out = -(water);
  let wet = (*params_2).shore.w;
  let reach = (*params_2).shoreEarth.w;
  if ((out >= reach)) {
    return vec3f();
  }
  let t = (&(*params_2).shoreTuning);
  let broad = valueNoise((turned(xy, (*t).turns.xy) * (*params_2).shoreEdge.y));
  let scale = ((*params_2).shoreEdge.y * (*t).bands.z);
  let fine = mix(valueNoise(((turned(xy, (*t).turns.zw) * scale) + vec2f(41.70000076293945, 13.100000381469727))), 0.5f, smoothstep((*t).fade.x, (*t).fade.y, (footprint * scale)));
  let line = (reach * (1f - ((*params_2).shoreEdge.x * mix(broad, fine, (*t).bands.w))));
  return vec3f((1f - smoothstep((wet * (*t).bands.x), wet, out)), (1f - smoothstep(mix(wet, line, (*t).bands.y), line, out)), (1f - smoothstep(wet, line, out)));
}

fn atLeast(colour: vec3f, least: f32) -> vec3f {
  let luminance = dot(colour, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023));
  return (colour * max(1f, (least / max(luminance, 1e-5f))));
}

fn wanderNoise(at: vec2f) -> f32 {
 let push=vec2f(valueNoise(at*0.61+vec2f(11.3,47.1)),valueNoise(at*0.61+vec2f(83.9,5.7)))-0.5;
 let p=at+push*1.4;
 let a=valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y));
 let b=valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*2.03+vec2f(19.1,7.7));
 let c=valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*4.01+vec2f(3.3,41.9));
 return a*0.5+b*0.3+c*0.2;
}

fn roadLane(paved: GroundPaved) -> i32 {
 if(paved.run.w<0.0){return -1;}
 let own=u32(paved.run.w);
 if(paved.drawn[own]<=0.0){return -1;}
 for(var n=4u;n>0u;n--){
  let k=params_1.roadOrder[n-1u];
  if(k==own){break;}
  if(paved.drawn[k]>0.0){return -1;}
 }
 return i32(own);
}

fn groundStrip(xy: vec2f, footprint: f32, paved: GroundPaved) -> f32 {
 let kind=roadLane(paved);
 if(kind<0||params_1.view.y==1u){return 0.0;}
 let look=params_1.roads[kind];
 let half=look.track.y;
 if(half<=0.0||paved.lane.w>look.track.z){return 0.0;}
 let reach=half*(1.0+(wanderNoise(xy*0.4+vec2f(41.3,9.7))-0.5)*3);
 let soft=max(footprint,0.3);
 let shown=1.0-smoothstep(0.2,0.4,footprint/(2.0*half));
 return (1.0-smoothstep(reach-soft,reach+soft,paved.lane.z))*shown;
}

fn groundShoulder(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec2f {
 var worn=vec2f(0.0);
 let plain=params_1.view.y==1u;
 for(var k=0u;k<4u;k++){
  let look=params_1.roads[k];
  let width=look.shoulder.w;
  let out=-paved.drawn[k];
  if(out<=0.0){worn=vec2f(1.0,f32(k));continue;}
  if(plain||out>=width){continue;}
  let wander=saturate((wanderNoise(xy*look.edge.y+vec2f(27.3,88.1))-0.5)*5+0.5);
  let reach=width*(1.0-look.edge.x*wander);
  let across=saturate(out/reach);
  let tufts=1.0-across*(1.0-smoothstep(0.44,0.52,wanderNoise(xy*0.9090909090909091+vec2f(5.9,63.1))));
  let wear=(1.0-smoothstep(0.35,1.0,out/reach))*tufts*(1.0-smoothstep(0.03,0.08,footprint/width));
  if(wear>worn.x){worn=vec2f(wear,f32(k));}
 }
 let strip=groundStrip(xy,footprint,paved);
 if(strip>0.0){worn=vec2f(1.0-strip*0.75,f32(roadLane(paved)));}
 return worn;
}

fn roadGrain(p: vec2f, pixel: f32) -> f32 {
 let shown=vec4f(1.0)-smoothstep(vec4f(0.6),vec4f(1.2),pixel*vec4f(1.0,2.03,4.01,8.3));
 var grain=0.0;
 if(shown.x>0.0){grain+=(valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y))-0.5)*0.25*shown.x;}
 if(shown.y>0.0){grain+=(valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*2.03+vec2f(19.1,7.7))-0.5)*0.35*shown.y;}
 if(shown.z>0.0){grain+=(valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*4.01+vec2f(3.3,41.9))-0.5)*0.4*shown.z;}
 if(shown.w>0.0){
  let stone=valueNoise(p*8.3+vec2f(71.9,13.7));
  grain+=(smoothstep(0.7,0.76,stone)-smoothstep(0.7,0.76,1.0-stone))*0.5*shown.w;
 }
 return grain;
}

fn groundRuts(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec3f {
 let kind=roadLane(paved);
 if(kind<0||params_1.view.y==1u){return vec3f(0.0);}
 let look=params_1.roads[kind];
 let width=look.ruts.z;
 if(width<=0.0){return vec3f(0.0);}
 let shown=1.0-smoothstep(0.2,0.4,footprint/width);
 if(shown<=0.0){return vec3f(0.0);}
 let half=paved.lane.w;
 var depth=0.0;var rise=0.0;var ruts=0.0;
 for(var i=0;i<2;i++){
  let at=look.ruts[i];
  if(at<=0.0||at+width>half){continue;}
  ruts+=2.0;
  let u=(paved.lane.z-at)/(width*0.5);
  if(abs(u)<1.0){let b=1.0-u*u;depth=b*b;rise=4.0*u*b;}
 }
 if(ruts==0.0){return vec3f(0.0);}
 let faint=mix(0.4,1.0,smoothstep(0.4,0.6,wanderNoise(xy*0.16666666666666666+vec2f(13.9,57.3))))*shown;
 let mean=ruts*width*0.5333333333333333/(2.0*half);
 return vec3f(paved.lane.xy*(look.ruts.w*rise*0.6495193556767991*faint),look.track.x*(mean-depth)*faint);
}

fn strokeJoint(along: f32, perMetre: f32, footprint: f32, width: f32) -> f32 {
 let shown=1.0-smoothstep(0.7,2.5,footprint/width);
 if(perMetre<=0.0||shown<=0.0){return 0.0;}
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(width*0.5,width*0.5+footprint,to))*shown;
}

fn walkJoint(footprint: f32, paved: GroundPaved) -> f32 {
 if(paved.run.w<0.0||params_1.view.y==1u){return 0.0;}
 let road=params_1.roads[u32(paved.run.w)];
 let beyond=paved.lane.z-paved.lane.w;
 if(beyond<=0.0||beyond>=road.join.z){return 0.0;}
 return strokeJoint(paved.run.z,road.slabs.x,footprint,0.03)*road.slabs.y;
}

fn areaJoint(along: f32, perMetre: f32, footprint: f32, width: f32) -> f32 {
 let drawn=max(width,footprint);
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(drawn*0.5-footprint*0.5,drawn*0.5+footprint*0.5,to))*(width/drawn);
}

fn groundMarks(xy: vec2f, footprint: f32, paved: GroundPaved) -> f32 {
 let kind=roadLane(paved);
 if(kind<0||params_1.view.y==1u){return 0.0;}
 let look=params_1.roads[kind];
 let line=look.marks.x;
 if(line<=0.0){return 0.0;}
 let bars=look.crossing;
 let shown=vec2f(1.0)-smoothstep(vec2f(0.35),vec2f(0.85),footprint/vec2f(2.0*line,bars.x));
 if(shown.x<=0.0&&shown.y<=0.0){return 0.0;}
 let far=bars.z+bars.y;
 // How far along this stroke the point is from its dash's middle.
 let dash=(fract(paved.run.z/look.marks.z)-0.5)*look.marks.z;
 // How far inside the nearest road that crosses this one the point's dash
 // reaches (near), and the point itself lies where that road runs on
 // across this road's width (cross).
 var near=-1e9;var cross=-1e9;
 for(var i=paved.list.x;i<paved.list.y;i++){
  let entry=surfaceIndex[i];
  if((entry>>30u)!=1u){continue;}
  let seg=surfaces[entry&536870911u];
  if(params_1.roads[u32(seg.detail.y)].track.w<=0.0){continue;}
  let a=seg.ends.xy;let ab=seg.ends.zw-a;let len=max(length(ab),1e-6);
  if(abs(dot(ab,paved.run.xy))>0.8*len){continue;}
  let free=dot(xy-a,ab)/len;
  let away=xy-(a+ab*(clamp(free,0.0,len)/len));let off=length(away);
  let inside=strokeCutInside(xy,seg.ends,seg.detail,seg.detail.x-off);
  // Toward the crossing road its inside grows by this much a metre along
  // this stroke.
  let nearer=dot(paved.run.xy,away)/max(off,1e-5);
  near=max(near,inside+nearer*dash+abs(nearer)*look.marks.y*0.5);
  let cuts=u32(seg.detail.z);
  let room=2.0*paved.lane.w;
  // Beside it: not past an end its stroke is cut at, nor within this
  // road's width of one (round a bend its stretches meet end to end), and
  // beside the stretch's own length: past its end the distance is to a
  // point, and a crossing laid by it would be a fan of bars round the end
  // of a street that only joins this road.
  let ends=((cuts&1u)!=0u&&free<room)||((cuts&2u)!=0u&&len-free<room);
  if(!ends&&free>=0.0&&free<=len){cross=max(cross,inside);}
 }
 let soft=max(footprint,0.03)*0.5;
 let centre=(1.0-smoothstep(line-soft,line+soft,paved.lane.z))
  *(1.0-smoothstep(look.marks.y*0.5-soft,look.marks.y*0.5+soft,abs(dash)))
  *(1.0-step(-far-0.3,near));
 let zone=smoothstep(-far-soft,-far+soft,cross)*(1.0-smoothstep(-bars.z-soft,-bars.z+soft,cross));
 let across=abs(fract(paved.lane.z/(2.0*bars.x))-0.5)*2.0*bars.x;
 let bar=1.0-smoothstep(bars.x*0.5-soft,bars.x*0.5+soft,across);
 let kept=1.0-smoothstep(paved.lane.w-bars.w-soft,paved.lane.w-bars.w+soft,paved.lane.z);
 return max(centre*shown.x,zone*bar*kept*shown.y);
}

fn groundCurb(footprint: f32, paved: GroundPaved) -> vec3f {
 if(paved.run.w<0.0||params_1.view.y==1u){return vec3f(0.0);}
 let own=u32(paved.run.w);
 let look=params_1.roads[own];
 let width=look.curb.w;
 if(width<=0.0){return vec3f(0.0);}
 let inside=paved.drawn[own];
 if(inside<-2.0*width||inside>2.0*look.slabs.z){return vec3f(0.0);}
 let shown=1.0-smoothstep(0.25,0.6,footprint/width);
 if(shown<=0.0){return vec3f(0.0);}
 let soft=max(footprint,0.03)*0.5;
 // Another carriageway's surface over the edge leaves no curb there.
 var open=1.0;
 for(var k=0u;k<4u;k++){
  if(k!=own&&params_1.roads[k].track.w>0.0){open*=1.0-smoothstep(-soft,soft,paved.drawn[k]);}
 }
 let edge=smoothstep(-soft,soft,inside);
 let stones=smoothstep(-width-soft,-width+soft,inside)*(1.0-edge);
 let face=edge*(1.0-smoothstep(look.slabs.z-soft,look.slabs.z+soft,inside));
 return vec3f(paved.lane.xy*(look.slabs.w*face),stones)*(open*shown);
}

fn groundRoads(xy: vec2f, footprint: f32, paved: GroundPaved, under: vec4f) -> vec4f {
 var surface=under;
 let plain=params_1.view.y==1u;
 let worn=groundShoulder(xy,footprint,paved);
 if(worn.x>0.0){
  let look=params_1.roads[u32(worn.y)];
  let luma=vec3f(0.2126,0.7152,0.0722);
  let lift=max(1.0,dot(under.xyz,luma)/max(dot(look.shoulder.xyz,luma),1e-5));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w)*0.5;
  surface=mix(under,vec4f(look.shoulder.xyz*lift*(1.0+look.shape.z*grain),look.core.w),worn.x*look.edge.w);
 }
 let lane=roadLane(paved);
 // How far the roads already laid here are carried onto the next: each
 // one's own carry, by how much of the ground it covers.
 var carried=0.0;
 for(var n=0u;n<4u;n++){
  let k=params_1.roadOrder[n];
  let look=params_1.roads[k];
  var feather=max(look.shape.x,footprint)*0.5;
  var inside=paved.drawn[k];
  if(carried>0.0){
   // The blend starts at this road's edge and runs inward, from a line
   // that wanders as far either way: gravel spilt over asphalt in drifts,
   // never a ruled end across a road.
   feather=max(feather,carried*0.5);
   inside-=feather;
   if(!plain){inside+=(wanderNoise(xy*0.4+vec2f(7.9,52.3))-0.5)*2.0*carried;}
  }
  if(inside<=-feather){continue;}
  var core=look.core.xyz;
  if(!plain){
   let hue=smoothstep(0.52,0.66,wanderNoise(xy*look.shape.y+vec2f(61.7,17.3)))*look.worn.w;
   let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
   core=mix(core,look.worn.xyz,hue)*(1.0+look.shape.z*grain);
   if(i32(k)==lane){
    core*=1.0+groundRuts(xy,footprint,paved).z;
    let strip=groundStrip(xy,footprint,paved)*0.6;
    core=mix(core,params_1.verge.xyz*(1.0+look.shape.z*grain*0.5),strip);
   }
   else if(lane<0){
    // Beside a street, its walk and the walk's slabs; past the walk, an
    // area's own slabs, on its own grid (SURFACE_AREA_BEARING), with a
    // deeper joint along the walk's outer edge (the carriageway's, where it
    // has none), so the walk or the road reads as its own strip.
    var out=1e9;
    if(paved.run.w>=0.0){
     let street=params_1.roads[u32(paved.run.w)];
     out=paved.lane.z-paved.lane.w-street.join.z;
     if(out<0.0&&u32(street.join.w)==k){core*=1.0-walkJoint(footprint,paved);}
    }
    if(out>=0.0&&look.stones.z>0.0&&paved.bearing>=0.0&&!plain){
     // A plain border a slab wide (at most slabs.areaBorderM) runs along the
     // edge line before the grid starts: where the grid is not square to
     // the street, it meets the border, not the kerb.
     let border=1.0-smoothstep(0.0,footprint,out-min(1.0/look.stones.z,2));
     let w=0.1;
     let along=vec2f(cos(paved.bearing),sin(paved.bearing));
     // The grid fades out before a slab is a few pixels across, so far
     // paving reads as its colour rather than a shimmering mesh.
     let near=(1.0-smoothstep(0.041666666666666664,0.4,footprint*look.stones.z))*(1.0-border);
     let joint=near*max(areaJoint(dot(xy,along),look.stones.z,footprint,w),areaJoint(dot(xy,vec2f(-along.y,along.x)),look.stones.z,footprint,w));
     let drawn=max(w,footprint);
     let edge=(1.0-smoothstep(drawn-footprint*0.5,drawn+footprint*0.5,out))*(w/drawn);
     core*=1.0-max(joint,edge*1.5)*look.stones.w;
    }
   }
  }
  let on=smoothstep(-feather,feather,inside);
  surface=mix(surface,vec4f(core,look.core.w),on);
  // Only a carriageway is carried onto the road it joins.
  if(look.track.w>0.0){carried=max(carried,look.join.x*on);}
 }
 let paint=groundMarks(xy,footprint,paved);
 if(paint>0.0){
  let look=params_1.roads[u32(paved.run.w)];
  let worn=1.0-look.marks.w*smoothstep(0.4,0.6,wanderNoise(xy*0.7692307692307692+vec2f(33.7,71.1)));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  surface=mix(surface,vec4f(look.paint.xyz*(1.0+look.shape.z*grain),look.core.w),paint*look.paint.w*worn);
 }
 let stones=groundCurb(footprint,paved).z;
 if(stones>0.0){
  let look=params_1.roads[u32(paved.run.w)];
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  let joint=strokeJoint(paved.run.z,look.stones.x,footprint,0.03)*look.stones.y;
  surface=mix(surface,vec4f(look.curb.xyz*(1.0+look.shape.z*grain)*(1.0-joint),look.core.w),stones);
 }
 return surface;
}

fn groundColour(xy: vec2f, footprint: f32, site: vec4f, paved: GroundPaved, water: f32) -> vec4f {
  let params_2 = (&params_1);
  let aa = (footprint * 0.5f);
  let plot = (&plots[i32(site.x)]);
  let variation = mottle(xy, footprint, (*plot).rows.xy, site.x, (*plot).dry.xy);
  let noise = variation.x;
  var albedo = groundTint(((*plot).colour.xyz * fieldTexture(xy, footprint, i32(site.x))), ((*plot).detail.x * variation.y));
  var roughness = (*plot).colour.w;
  let verge = groundVerge(site, footprint);
  let vergeColour = ((*params_2).verge.xyz * (1f + (0.18f * noise)));
  albedo = mix(albedo, vergeColour, verge);
  roughness = mix(roughness, 0.95f, verge);
  let inRegion = rectInside(xy, (*params_2).region);
  let distant = max((1f - smoothstep(0f, 600f, inRegion)), smoothstep((*params_2).feathers.y, (*params_2).feathers.z, footprint));
  albedo = mix(albedo, (*params_2).distant.xyz, distant);
  let floor_1 = groundFloor(xy, footprint, site.w);
  if ((floor_1.x > 0f)) {
    albedo = mix(albedo, forestFloor(xy, footprint, noise), floor_1.x);
    roughness = mix(roughness, (*params_2).forestDetail.z, floor_1.x);
  }
  if ((floor_1.y > 0f)) {
    albedo = mix(albedo, vergeColour, floor_1.y);
    roughness = mix(roughness, 0.95f, floor_1.y);
  }
  let shore = groundShore(xy, footprint, water);
  if ((shore.y > 0f)) {
    let field = mix(dot((*params_2).verge.xyz, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)), (dot((*plot).colour.xyz, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)) * (1f - (0.5f * (*plot).rows.w))), smoothstep(0f, (*params_2).shoreTuning.ground.y, site.y));
    let least = (field * (*params_2).shoreEdge.z);
    let earth = ((*params_2).shoreEarth.xyz * (1f + ((*params_2).shoreTuning.ground.x * noise)));
    albedo = mix(albedo, atLeast(earth, least), shore.y);
    roughness = mix(roughness, (*params_2).shoreTuning.ground.z, shore.y);
    albedo = mix(albedo, atLeast((*params_2).shore.xyz, least), shore.x);
    roughness = mix(roughness, (*params_2).shoreTuning.ground.w, shore.x);
  }
  let paving = groundRoads(xy, footprint, paved, vec4f(albedo, roughness));
  let bed = smoothstep(-(aa), max(aa, (*params_2).shoreTuning.fade.z), water);
  albedo = mix(paving.xyz, (*params_2).waterBed.xyz, bed);
  return vec4f(max(albedo, vec3f()), paving.w);
}

fn srgbToLinear(c: vec3f) -> vec3f {
  return pow(max(c, vec3f()), vec3f(2.200000047683716));
}

fn groundAlbedo(world: vec3f, tint: vec4f, cell: vec4u, paved: GroundPaved) -> vec4f {
  let footprint = length(fwidth(world.xy));
  let xy = world.xy;
  let site = groundSite(xy, cell, paved);
  let surface = groundColour(xy, footprint, site, paved, groundWater(xy, cell));
  let albedo = mix(surface.xyz, srgbToLinear(tint.xyz), tint.w);
  return vec4f(albedo, mix(surface.w, 0.85f, tint.w));
}

struct ScarSample {
  weights: vec4f,
  relief: vec4f,
  at: vec4f,
}

struct ScarTiles {
  lo: vec2u,
  e00: vec2u,
  e10: vec2u,
  e01: vec2u,
  e11: vec2u,
}

@group(1) @binding(6) var scarPages: texture_2d<u32>;

fn scarEntry(tile: vec2u) -> vec2u {
 let P=scarParams;
 if(any(tile<P.cacheBounds.xy)||any(tile>P.cacheBounds.zw)){return vec2u(0u);}
 let tilesX=(P.pages.x+15u)/16u;let key=tile.y*tilesX+tile.x;
 var at=((key*0x9e3779b1u)^(key>>16u))&P.pages.z;
 loop {let dim=textureDimensions(scarPages);let e=textureLoad(scarPages,vec2i(i32(at%dim.x),i32(at/dim.x)),0).xy;
  if(e.x==0u){break;}
  if((e.x&536870911u)==key+1u){return e;}at=(at+1u)&P.pages.z;}
 return vec2u(2147483648u,P.defaultWord.x);
}

fn scarTiles(lo: vec2u, hi: vec2u) -> ScarTiles {
 let a=lo/16u;let span=hi/16u-a;var e=array<vec2u,4>();
 // One probe site in a loop, as for the corners in `scarLinear`; a tile
 // that repeats where the cells span one is copied, not probed again.
 for(var k=0u;k<4u;k++){
  let o=min(vec2u(k&1u,k>>1u),span);let j=o.x+o.y*2u;
  if(j<k){e[k]=e[j];}else{e[k]=scarEntry(a+o);}
 }
 return ScarTiles(a,e[0],e[1],e[2],e[3]);
}

fn scarTilesBlank(t: ScarTiles) -> bool {
 let x=vec4u(t.e00.x,t.e10.x,t.e01.x,t.e11.x);let y=vec4u(t.e00.y,t.e10.y,t.e01.y,t.e11.y);
 let single=(x&vec4u(2147483648u))!=vec4u(0u);
 return all((x==vec4u(0u))|(single&(y==vec4u(0u))));
}

struct GroundCubic {
  taps: vec4f,
  weights: vec4f,
}

fn groundCubic(uv: vec2f) -> GroundCubic {
  let size = vec2f(scarParams.pages.xy);
  let p = uv * size - 0.5;
  let i = floor(p);
  let f = p - i;
  let f2 = f * f;
  let f3 = f2 * f;
  let w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
  let w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
  let w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
  let w3 = f3 / 6.0;
  let g0 = w0 + w1;
  let g1 = w2 + w3;
  return GroundCubic(vec4f((i - 0.5 + w1 / g0) / size, (i + 1.5 + w3 / g1) / size), vec4f(g0, g1));
}

fn groundCubicBlend(cubic: GroundCubic, a: vec4f, b: vec4f, c: vec4f, e: vec4f) -> vec4f {
  let g0 = cubic.weights.xy;
  let g1 = cubic.weights.zw;
  // Keep the original hardware cubic outer contraction explicit after inlining.
  return fma(vec4f(g0.y), g0.x * a + g1.x * b, g1.y * (g0.x * c + g1.x * e));
}

fn groundCubicTap(cubic: GroundCubic, k: u32) -> vec2f {
  return vec2f(select(cubic.taps.x, cubic.taps.z, (k & 1u) == 1u), select(cubic.taps.y, cubic.taps.w, k >= 2u));
}

fn scarFilterAxis(uv: f32, size: f32) -> u32 {
 let bits=bitcast<u32>(clamp(uv,0.0,1.0))&0x7fffffffu;let exponent=bits>>23u;
 let mantissa=(bits&0x7fffffu)|select(0u,0x800000u,exponent!=0u);
 let product=bitcast<u32>(f32(mantissa)*size);
 let value=(product&0x7fffffu)|select(0u,0x800000u,product>>23u!=0u);
 // Each decoded f32 mantissa has exponent bias 150; Q8 adds eight bits.
 let shift=292-i32(exponent)-i32(product>>23u);
 if(shift>=32){return 0u;}if(shift<=0){return value<<u32(-shift);}
 return (value+(1u<<u32(shift-1)))>>u32(shift);
}

fn scarFilterPosition(uv: vec2f, size: vec2f) -> vec2f {
 let q=vec2f(f32(scarFilterAxis(uv.x,size.x)),f32(scarFilterAxis(uv.y,size.y)));
 return clamp((q-128.0)/256.0,vec2f(0.0),size-1.0);
}

fn scarTileEntry(t: ScarTiles, tile: vec2u) -> vec2u {
 let o=tile-t.lo;
 return select(select(t.e00,t.e10,o.x==1u),select(t.e01,t.e11,o.x==1u),o.y==1u);
}

@group(1) @binding(7) var scars: texture_2d_array<f32>;

fn scarPoolWord(offset: u32) -> vec4f {
 let width=scarParams.pages.w;
 return textureLoad(scars,vec2i(i32(offset%width),i32(offset/width)),0,0);
}

fn scarNormalized(q: u32) -> f32 {
 let bytes=vec4u(round(scarPoolWord(q)*255.0));return bitcast<f32>(bytes.x|(bytes.y<<8u)|(bytes.z<<16u)|(bytes.w<<24u));
}

fn scarWord(word: u32) -> vec4f {
 return vec4f(scarNormalized((word&255u)*16u),scarNormalized(((word>>8u)&255u)*16u),scarNormalized(((word>>16u)&255u)*16u),scarNormalized((word>>24u)*16u));
}

fn scarCell(c: vec2u, e: vec2u) -> vec4f {
 if(e.x==0u){return vec4f(0.0);}if((e.x&2147483648u)!=0u){return scarWord(e.y);}
 let local=(c.y%16u)*16u+c.x%16u;
 if((e.x&536870912u)!=0u){return scarPoolWord(e.y+local);}
 let offset=e.y>>8u;var lo=0u;var hi=e.y&255u;
 while(lo<hi){let mid=(lo+hi)/2u;let bytes=vec4u(round(scarPoolWord(offset+mid*2u)*255.0));let end=bytes.x+bytes.y*256u;if(end<=local){lo=mid+1u;}else{hi=mid;}}
 return scarPoolWord(offset+lo*2u+1u);
}

fn scarFilterQuanta(a: vec4u, b: vec4u, c: vec4u, other: vec4u, f: vec2f) -> vec4u {
 let wx=u32(floor(f.x*256.0+0.5));let wy=u32(floor(f.y*256.0+0.5));
 let total=a*((256u-wx)*(256u-wy))+b*(wx*(256u-wy))+c*((256u-wx)*wy)+other*(wx*wy);
 return (total+vec4u(2048u))/vec4u(4096u);
}

fn scarLinear(uv: vec2f, tiles: ScarTiles) -> vec4f {
 let P=scarParams;let size=vec2f(P.pages.xy);let p=scarFilterPosition(uv,size);
 let cell=vec2i(floor(p));let f=fract(p);var q=array<vec4u,4>();
 for(var k=0u;k<4u;k++){
  let c=vec2u(clamp(cell+vec2i(i32(k&1u),i32(k>>1u)),vec2i(0),vec2i(P.pages.xy)-1));let e=scarTileEntry(tiles,c/16u);
  // The cell's own tile, one word throughout, with all four corners in it.
  if(k==0u&&(e.x==0u||(e.x&2147483648u)!=0u)&&all(c%16u<vec2u(15u))){return scarWord(e.y);}
  q[k]=vec4u(round(scarCell(c,e)*255.0));
 }
 let w=scarFilterQuanta(q[0],q[1],q[2],q[3],f);
 return vec4f(scarNormalized(w.x),scarNormalized(w.y),scarNormalized(w.z),scarNormalized(w.w));
}

fn crossing(x: f32, edge: f32, width: f32) -> f32 {
  let w = max(width, 1e-3);
  return smoothstep(edge - w, edge + w, x);
}

fn groundScars(xy: vec2f, footprint: f32) -> ScarSample {
  var out: ScarSample;
  out.at = vec4f(xy, 0.0, 0.0);
  let P = scarParams;
  if (P.grid.w <= 0.0) { return out; }
  let cell = P.grid.z;
  let uv = xy * P.grid.xy;
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return out; }
  let texel = P.grid.xy * cell;
  // Every cell the taps below read is in these tiles: where none is marked,
  // every tap is zero, and the cheap reject would return the same.
  let centre = uv * vec2f(P.pages.xy) - 0.5;
  let last = vec2i(P.pages.xy) - 1;
  let tiles = scarTiles(
    vec2u(clamp(vec2i(floor(centre - 3.0)), vec2i(0), last)),
    vec2u(clamp(vec2i(floor(centre + 3.0)) + 1, vec2i(0), last)));
  if (scarTilesBlank(tiles)) { return out; }
  // Twelve bilinear taps through one call, as a shader compiler inlines each
  // call: the rim's four diagonals (a, b, c, e), the cubic's four, then
  // either side of `uv` along x and y for the slopes (xp, xm, yp, ym).
  let r = 1.5;
  let h = 0.75;
  let cubic = groundCubic(uv);
  var taps = array<vec4f, 12>();
  for (var t = 0u; t < 12u; t++) {
    if (t == 8u) {
      // Cheap reject: nothing marked within reach.
      let near = max(max(taps[0], taps[1]), max(taps[2], taps[3]));
      let s = groundCubicBlend(cubic, taps[4], taps[5], taps[6], taps[7]);
      if (max(max(s.x, s.y), max(s.z, s.w)) + max(max(near.x, near.y), max(near.z, near.w)) <= 0.0) {
        return out;
      }
    }
    var at: vec2f;
    if (t < 4u) {
      at = uv + texel * vec2f(select(-r, r, (t & 1u) == 1u), select(-r, r, t >= 2u));
    } else if (t < 8u) {
      at = groundCubicTap(cubic, t - 4u);
    } else {
      let side = texel * select(vec2f(0.0, h), vec2f(h, 0.0), t < 10u);
      at = select(uv - side, uv + side, (t & 1u) == 0u);
    }
    taps[t] = scarLinear(at, tiles);
  }
  let a = taps[0];
  let b = taps[1];
  let c = taps[2];
  let e = taps[3];
  let near = max(max(a, b), max(c, e));
  let s = groundCubicBlend(cubic, taps[4], taps[5], taps[6], taps[7]);
  let xp = taps[8];
  let xm = taps[9];
  let yp = taps[10];
  let ym = taps[11];
  let detail = 1.0 - smoothstep(0.5, 2.0, footprint / cell);
  let k = P.full;
  let depth = min(s.x * k.x, 1.6);
  let wide = min((a.x + b.x + c.x + e.x) * 0.25 * k.x, 1.6);
  // Slopes per metre of the crater field, from the taps either side.
  let bowlSlope = vec2f(xp.x - xm.x, yp.x - ym.x) * k.x / (2.0 * h * cell);
  let wideSlope = vec2f((b.x + e.x) - (a.x + c.x), (c.x + e.x) - (a.x + b.x)) * 0.5 * k.x / (2.0 * r * cell);
  // The height: down into the bowl, up onto the rim just outside it.
  let onRim = select(0.0, 1.0, wide > depth);
  let slope = (P.relief.y * onRim * (wideSlope - bowlSlope) - bowlSlope) * P.relief.x * detail;
  // Iso-lines, each crossing within a pixel.
  let grain = valueNoise(xy * 2.2222222222222223 + vec2f(5.7, 2.1)) - 0.5;
  let pixelDepth = length(bowlSlope) * footprint;
  let bowl = crossing(depth, 0.3 + 0.14 * grain, pixelDepth);
  let skirt = crossing(depth, 0.035 + 0.03 * grain, pixelDepth) * (1.0 - bowl);
  // The deepest crater within the taps: a lone pock's skirt is scorched
  // earth, a real crater's is its thrown soil.
  let peak = max(max(max(s.x, xp.x), max(xm.x, yp.x)), max(ym.x, near.x)) * k.x;
  let soil = smoothstep(0.05, 0.09, peak);
  let ring = skirt * soil;
  let scorched = crossing(depth, 0.013 + 0.006 * grain, pixelDepth) * (1.0 - bowl);
  // Darkest at its heart, browner toward its edge, as a burn is.
  let heart = 0.45 + 0.55 * smoothstep(0.013, 0.035, depth);
  let pock = scorched * (1.0 - soil) * 0.85 * heart;
  // Soot: ash in flecks over the scorch, thickest near the burst, never a
  // solid sheet: a flecked dark is one thing no shadow here is.
  let burnt = saturate(s.y * k.y);
  let fleckAt = valueNoise(xy * 4.5 + vec2f(8.1, 3.3)) * 0.75 + valueNoise(xy * 13.950000000000001) * 0.25;
  let ash = crossing(fleckAt, mix(0.97, 0.72, burnt), footprint * 4.5) * step(0.45, burnt);
  let soot = max(ash * (1.0 - bowl) * (1.0 - ring), pock);
  let wear = 1.0 - (1.0 - saturate(s.zw * k.zw)) * (1.0 - saturate(s.zw * k.zw));
  // Ruts have edges: tracks cross their iso-line within a pixel, ragged.
  let trackSlope = vec2f(xp.z - xm.z, yp.z - ym.z) * k.z / (2.0 * h * cell);
  let rut = crossing(wear.x, 0.35 + 0.2 * grain, length(trackSlope) * footprint);
  out.weights = vec4f(bowl, soot, rut * wear.x, wear.y);
  out.relief = vec4f(slope, ring, depth);
  return out;
}

fn groundScarsSeen(world: vec3f, eye: vec3f, footprint: f32) -> ScarSample {
  // One call in the loop, as a shader compiler inlines each call.
  let view = normalize(eye - world);
  let run = -view.xy / max(view.z, 0.25);
  let deep = scarParams.relief.x;
  var scar: ScarSample;
  var at = world.xy;
  for (var i = 0; i < 3; i++) {
    scar = groundScars(at, footprint);
    if (i == 0 && scar.relief.w <= 0.0) { break; }
    at = world.xy + run * min(scar.relief.w, 1.6) * deep * 0.5;
  }
  return scar;
}

fn scarredSurface(surface: vec4f, scar: ScarSample) -> vec4f {
  let P = scarParams;
  let w = scar.weights;
  let xy = scar.at.xy;
  var albedo = surface.xyz;
  var roughness = surface.w;
  let grain = valueNoise(xy * 1.7);
  let fleck = valueNoise(xy * 6.3 + vec2f(2.2, 9.1));
  albedo = mix(albedo, P.trampled.xyz * mix(0.9, 1.1, grain), w.w * P.trampled.w);
  albedo = mix(albedo, P.tracks.xyz * mix(0.85, 1.1, grain), w.z * P.tracks.w);
  // Soot is uneven: ash grey and char black in flecks and drifts.
  let ash = mix(0.6, 2.4, fleck * 0.6 + valueNoise(xy * 0.7 + vec2f(4.4, 0.3)) * 0.4);
  albedo = mix(albedo, P.scorch.xyz * ash, w.y * P.scorch.w);
  let ring = scar.relief.z * P.ejecta.w;
  albedo = mix(albedo, P.ejecta.xyz * mix(0.8, 1.15, fleck), ring);
  let bowl = w.x * P.crater.w;
  // The deeper, the darker: the floor sees less sky than the lip.
  let cavity = mix(1.0, 0.7, saturate(scar.relief.w));
  albedo = mix(albedo, P.crater.xyz * mix(0.8, 1.15, fleck) * cavity, bowl);
  // Churned soil and ash are fully rough: a tilted wall never catches sky.
  roughness = mix(roughness, 1.0, max(w.y * P.scorch.w, max(bowl, ring)));
  return vec4f(albedo, roughness);
}

fn groundBank(xy: vec2f, cell: vec4u, footprint: f32) -> vec3f {
 var sum=vec4f(0.0);
 for(var i=cell.z;i<cell.w;i++){
  let at=surfaceIndex[i]&536870911u;
  let stretch=surfaces[at];
  let a=stretch.ends.xy;let ab=stretch.ends.zw-a;let len2=dot(ab,ab);
  var t=0.0;if(len2>0.0){t=clamp(dot(xy-a,ab)/len2,0.0,1.0);}
  let away=xy-(a+ab*t);let off=length(away);
  let half=stretch.detail.x+(stretch.detail.y-stretch.detail.x)*t;
  let inside=half-off;
  let grade=stretch.detail.z+(stretch.detail.w-stretch.detail.z)*t;
  let heights=surfaces[at+1u].ends.xy;
  let run=(heights.x+(heights.y-heights.x)*t)/max(grade,1e-4);
  let ease=max(footprint,0.75);
  let sloped=(1.0-smoothstep(run-ease,run+ease,-inside))*(1.0-smoothstep(0.0,ease,inside));
  let top=run+params_1.waterBed.w;
  let weight=exp(clamp(inside,-50,50)/0.7);
  let shown=min(grade*params_1.shoreEdge.w,0.84);
  sum+=vec4f(away/max(off,1e-4)*shown*sloped,1.0-smoothstep(top,top+2.0*ease,-inside),1.0)*weight;
 }
 if(sum.w<=0.0){return vec3f(0.0);}
 return sum.xyz/sum.w;
}

fn groundRoadRelief(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec2f {
  return (groundRuts(xy, footprint, paved).xy + groundCurb(footprint, paved).xy);
}

fn scarredNormal(n: vec3f, scar: ScarSample) -> vec3f {
  // At most 40° of tilt: steeper walls turn to the blue sky and read as haze.
  let slope = scar.relief.xy;
  let steep = length(slope);
  let capped = slope * min(1.0, 0.84 / max(steep, 1e-6));
  return normalize(n - vec3f(capped, 0.0) * n.z);
}

@group(2) @binding(7) var paint: texture_2d<f32>;

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

fn groundDapple(xy: vec2f, footprint: f32, cell: vec4u) -> f32 {
  let params_2 = (&params_1);
  let forest = groundForest(xy, cell);
  if ((forest < (-((*params_2).forestVerge.x) - (*params_2).forestVerge.y))) {
    return 0f;
  }
  let dapple = (&(*params_2).forestDapple);
  let at = (xy * (*dapple).x);
  let n = ((valueNoise(at) * 0.65f) + (valueNoise(((at * 2.7f) + vec2f(3.700000047683716, 8.100000381469727))) * 0.35f));
  let cut = (1f - (*dapple).y);
  let fleck = smoothstep((cut - 0.06f), (cut + 0.06f), n);
  let shown = (1f - smoothstep(0.1f, 0.5f, (footprint * (*dapple).x)));
  let flecks = mix((*dapple).y, fleck, shown);
  return ((flecks * (*dapple).z) * forestFloorWeight(xy, forest, footprint));
}

struct Aerial {
  extinction: vec3f,
  clearRadiusKm: f32,
  sunTint: vec3f,
  sunStrength: f32,
  mistColor: vec3f,
  mistColorStrength: f32,
  overviewFromM: f32,
  rangeNearM: f32,
  rangeFarM: f32,
  rangePower: f32,
  rangeStrength: f32,
  mistOpacity: f32,
  sunPower: f32,
  mistHeightM: vec2f,
  mistDistanceM: vec2f,
  horizonFade: vec2f,
}

struct Environment {
  worldToView: mat4x4f,
  observer: vec4f,
  sunDirection: vec4f,
  sunRadiance: vec4f,
  settings: vec4f,
  fill: vec4f,
  aerial: Aerial,
}

struct SunCascade {
  matrix: mat4x4f,
  bias: vec4f,
  interval: vec4f,
}

struct SunShadow {
  cascades: array<SunCascade, 4>,
  control: vec4f,
}

fn shadowPcf(depth: texture_depth_2d_array, compare: sampler_comparison, layer: i32, uv: vec2f, z: f32, pixel: vec2f, radius: f32) -> f32 {
  let phi=fract(52.9829189*fract(dot(pixel,vec2f(0.06711056,0.00583715))))*6.28318530718;
  let cphi=cos(phi);let sphi=sin(phi);
  let scale=radius/f32(textureDimensions(depth).x);
  var visibility=0.0;
  let b0=vec2f(0.31622776601683794,0);let o0=vec2f(b0.x*cphi-b0.y*sphi,b0.x*sphi+b0.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o0,layer,z);let b1=vec2f(-0.40387356772577226,0.36998127154283905);let o1=vec2f(b1.x*cphi-b1.y*sphi,b1.x*sphi+b1.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o1,layer,z);let b2=vec2f(0.06181932279751069,-0.7043992982171811);let o2=vec2f(b2.x*cphi-b2.y*sphi,b2.x*sphi+b2.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o2,layer,z);let b3=vec2f(0.5090564735709379,0.6639740256331726);let o3=vec2f(b3.x*cphi-b3.y*sphi,b3.x*sphi+b3.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o3,layer,z);let b4=vec2f(-0.934181236883857,-0.16524350714671632);let o4=vec2f(b4.x*cphi-b4.y*sphi,b4.x*sphi+b4.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o4,layer,z);
  return visibility*0.2;
}

fn shadowVisibility(depth: texture_depth_2d_array, compare: sampler_comparison, layer: i32, matrix: mat4x4f, settings: vec4f, world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  let clip=matrix*vec4f(world+normal*settings.y,1);
  let coord=clip.xyz/clip.w;
  let uv=vec2f(coord.x*0.5+0.5,0.5-coord.y*0.5);
  let z=coord.z-settings.x;
  let inside=all(uv>=vec2f(0))&&all(uv<=vec2f(1))&&z>=0&&z<=1;
  if (!inside) { return 1.0; }
  return shadowPcf(depth,compare,layer,uv,z,pixel,settings.z);
}

fn inheritedSunShadow(environment: Environment, sunShadow: SunShadow, sunDepth: texture_depth_2d_array, sunCompare: sampler_comparison, world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  let viewZ=(environment.worldToView*vec4f(world,1.0)).z;
  // Breaks are fractions of the receiver range [control.z, control.x]
  // (split near, capped far); the receiver normalises its depth the same way.
  // The range is sampled on a ray grid, so a sliver of map can sit just
  // nearer than it: the first cascade takes that too.
  let linearDepth=max((-viewZ-sunShadow.control.z)/(sunShadow.control.x-sunShadow.control.z),0.0);
  var shade=1.0;
  {
    let interval=sunShadow.cascades[0].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,0,sunShadow.cascades[0].matrix,sunShadow.cascades[0].bias,world,normal,pixel))*select(1.0,ratio,linearDepth>centre);
    }
  }
  {
    let interval=sunShadow.cascades[1].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,1,sunShadow.cascades[1].matrix,sunShadow.cascades[1].bias,world,normal,pixel))*ratio;
    }
  }
  {
    let interval=sunShadow.cascades[2].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,2,sunShadow.cascades[2].matrix,sunShadow.cascades[2].bias,world,normal,pixel))*ratio;
    }
  }
  {
    let interval=sunShadow.cascades[3].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,3,sunShadow.cascades[3].matrix,sunShadow.cascades[3].bias,world,normal,pixel))*ratio;
    }
  }
  return shade;
}

@group(3) @binding(0) var<uniform> data: Environment;

@group(3) @binding(6) var<uniform> sun: SunShadow;

@group(3) @binding(7) var sunDepth: texture_depth_2d_array;

@group(3) @binding(8) var sunCompare: sampler_comparison;

fn sampleSunShadow(world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  return inheritedSunShadow(data, sun, sunDepth, sunCompare, world, normal, pixel);
}

struct GroundPaintStyle {
  albedo: f32,
  emissive: f32,
  fogKeep: f32,
  grassGlow: f32,
  grassFalloff: f32,
  saturation: f32,
  pad1: f32,
  pad2: f32,
}

@group(2) @binding(6) var<uniform> paintStyle: GroundPaintStyle;

fn paintColour(paint: vec4f) -> vec3f {
  let c = pow(max(paint.xyz / max(paint.w, 1e-4), vec3f(0.0)), vec3f(2.2));
  let grey = dot(c, vec3f(0.2126, 0.7152, 0.0722));
  return max(mix(vec3f(grey), c, paintStyle.saturation), vec3f(0.0));
}

fn paintedAlbedo(albedo: vec3f, paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return albedo; }
  let colour = min(paintColour(paint), vec3f(1.0));
  return mix(albedo, colour * paintStyle.albedo, paint.w);
}

struct CastLight {
  position: vec4f,
  color: vec4f,
}

struct CastLights {
  header: vec4f,
  lights: array<CastLight, 24>,
}

@group(3) @binding(1) var<uniform> lights: CastLights;

fn castLight(base: vec3f, normal: vec3f, position: vec3f) -> vec3f {
  let count = u32(lights.header.x);
  let wrap = lights.header.y;
  let n = normalize(normal);
  var sum = vec3f();
  for (var i = 0u; (i < count); i++) {
    let toward = (lights.lights[i].position.xyz - position);
    let d2 = dot(toward, toward);
    let x2 = (d2 * lights.lights[i].position.w);
    if ((x2 < 1f)) {
      let falloff = (((1f - x2) * (1f - x2)) / (1f + (6f * x2)));
      let facing = (dot(n, toward) * inverseSqrt(max(d2, 1e-4f)));
      let wrapped = clamp(((facing + wrap) / (1f + wrap)), 0f, 1f);
      sum = (sum + (lights.lights[i].color.xyz * (falloff * wrapped)));
    }
  }
  let grey = dot(base, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023));
  let albedo = max(mix(base, vec3f(grey), 0.35f), vec3f(0.10000000149011612));
  return (albedo * (sum * 0.3183098861837907f));
}

fn roughnessMip(r: f32) -> f32 {
  if(r>=0.8){return (1.0-r)/0.2-2.0;}
  if(r>=0.4){return (0.8-r)*3.0/0.4-1.0;}
  if(r>=0.305){return (0.4-r)/0.095+2.0;}
  if(r>=0.21){return (0.305-r)/0.095+3.0;}
  return -2.0*log2(1.16*r);
}

fn cubeFace(d: vec3f) -> f32 {
  let a=abs(d);
  if(a.x>a.z) {
    if(a.x>a.y){return select(3.0,0.0,d.x>0.0);}
    return select(4.0,1.0,d.y>0.0);
  }
  if(a.z>a.y){return select(5.0,2.0,d.z>0.0);}
  return select(4.0,1.0,d.y>0.0);
}

fn cubeUv(d: vec3f, face: f32) -> vec2f {
  var p=vec2f(0);
  if(face==0){p=vec2f(d.z,d.y)/abs(d.x);}
  else if(face==1){p=vec2f(-d.x,-d.z)/abs(d.y);}
  else if(face==2){p=vec2f(-d.x,d.y)/abs(d.z);}
  else if(face==3){p=vec2f(-d.z,d.y)/abs(d.x);}
  else if(face==4){p=vec2f(-d.x,d.z)/abs(d.y);}
  else{p=vec2f(d.x,d.y)/abs(d.z);}
  return 0.5*(p+vec2f(1));
}

fn cubeSample(env: texture_2d<f32>, linear: sampler, d: vec3f, mip: f32, maxMip: f32) -> vec3f {
  var face=cubeFace(d);
  let filterIndex=max(4.0-mip,0.0);
  let size=exp2(max(mip,4.0));
  var uv=cubeUv(d,face)*(size-2.0)+vec2f(1);
  if(face>2.0){uv.y+=size;face-=3.0;}
  uv.x+=face*size;
  uv.x+=filterIndex*48.0;
  uv.y+=4.0*(exp2(maxMip)-size);
  uv/=vec2f(textureDimensions(env));
  return textureSampleGrad(env,linear,uv,vec2f(0),vec2f(0)).rgb;
}

fn samplePmrem(env: texture_2d<f32>, linear: sampler, d: vec3f, r: f32, maxMip: f32) -> vec3f {
  let mip=clamp(roughnessMip(r),-2.0,maxMip);
  let f=fract(mip);
  let a=cubeSample(env,linear,d,floor(mip),maxMip);
  if(f==0){return a;}
  return mix(a,cubeSample(env,linear,d,floor(mip)+1.0,maxMip),f);
}

fn standardPbr(base: vec3f, emissive: vec3f, authoredRoughness: f32, geometryRoughness: f32, metal: f32, ao: f32, n: vec3f, v: vec3f, l: vec3f, sunRadiance: vec3f, shadow: f32, environmentIntensity: vec3f, pmrem: texture_2d<f32>, linear: sampler, maxMip: f32, dfg: texture_2d<f32>, dfgSampler: sampler) -> vec3f {
  let roughness=min(max(authoredRoughness,0.0525)+geometryRoughness,1.0);
  let nl=clamp(dot(n,l),0.0,1.0);
  let nv=clamp(dot(n,v),0.0,1.0);
  let h=normalize(l+v);
  let nh=clamp(dot(n,h),0.0,1.0);
  let vh=clamp(dot(v,h),0.0,1.0);
  let alpha=roughness*roughness;
  let a2=alpha*alpha;
  let fresnel=exp2((-5.55473*vh-6.98316)*vh);
  let f0=mix(vec3f(0.04),base,metal);
  let f=f0*(1.0-fresnel)+vec3f(fresnel);
  let gv=nl*sqrt(a2+(1.0-a2)*nv*nv);
  let gl=nv*sqrt(a2+(1.0-a2)*nl*nl);
  let visibility=0.5/max(gv+gl,0.000001);
  let denominator=1.0-nh*nh*(1.0-a2);
  let distribution=a2/(denominator*denominator)*0.3183098861837907;
  let singleDirect=f*visibility*distribution;
  let fabV=textureSampleLevel(dfg,dfgSampler,vec2f(roughness,nv),0.0).rg;
  let fabL=textureSampleLevel(dfg,dfgSampler,vec2f(roughness,nl),0.0).rg;
  let ssV=f0*fabV.x+vec3f(fabV.y);
  let ssL=f0*fabL.x+vec3f(fabL.y);
  let emsV=1.0-(fabV.x+fabV.y);
  let emsL=1.0-(fabL.x+fabL.y);
  let favg=f0+(vec3f(1)-f0)*0.047619;
  let directMs=ssV*ssL*favg/(vec3f(1)-emsV*emsL*favg*favg+vec3f(0.000001))*(emsV*emsL);
  let diffuseContribution=base*(1.0-metal);
  let irradiance=nl*(sunRadiance*shadow);
  let directDiffuse=irradiance*(diffuseContribution*0.3183098861837907);
  let directSpecular=irradiance*(singleDirect+directMs);

  let dielectricSs=vec3f(0.04)*fabV.x+vec3f(fabV.y);
  let dielectricAvg=vec3f(0.04)+(vec3f(1)-vec3f(0.04))*0.047619;
  let dielectricMs=dielectricSs*dielectricAvg/(vec3f(1)-emsV*dielectricAvg)*emsV;
  let metallicSs=base*fabV.x+vec3f(fabV.y);
  let metallicAvg=base+(vec3f(1)-base)*0.047619;
  let metallicMs=metallicSs*metallicAvg/(vec3f(1)-emsV*metallicAvg)*emsV;
  let singleScatter=mix(dielectricSs,metallicSs,metal);
  let multiScatter=mix(dielectricMs,metallicMs,metal);
  let diffuse=diffuseContribution*(vec3f(1)-(dielectricSs+dielectricMs));
  let reflected=normalize(mix(reflect(-v,n),n,roughness*roughness*roughness*roughness));
  let radiance=samplePmrem(pmrem,linear,vec3f(reflected.x,-reflected.y,reflected.z),roughness,maxMip)*environmentIntensity;
  let iblIrradiance=samplePmrem(pmrem,linear,vec3f(n.x,-n.y,n.z),1.0,maxMip)*3.141592653589793*environmentIntensity;
  let cosineIrradiance=iblIrradiance*0.3183098861837907;
  let aoSpecular=clamp(ao-(1.0-pow(nv+ao,exp2(-(1.0+16.0*roughness)))),0.0,1.0);
  let indirectSpecular=(radiance*singleScatter+multiScatter*cosineIrradiance)*aoSpecular;
  let indirectDiffuse=diffuse*cosineIrradiance*ao;
  return max((directDiffuse+indirectDiffuse)+(directSpecular+indirectSpecular)+emissive,vec3f(0));
}

fn lightAlgorithm(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normalWorld: vec3f, worldPosition: vec3f, shadow: f32, eye: vec3f, sunDirection: vec3f, sunRadiance: vec3f, environmentIntensity: vec3f, maxMip: f32, pmrem: texture_2d<f32>, dfg: texture_2d<f32>, linear: sampler) -> vec3f {
 return standardPbr(base,emissive,roughness,geomRoughness,metal,ao,normalize(normalWorld),normalize(eye-worldPosition),sunDirection,sunRadiance,shadow,environmentIntensity,pmrem,linear,maxMip,dfg,linear);
 }

@group(3) @binding(3) var pmrem: texture_2d<f32>;

@group(3) @binding(4) var dfg: texture_2d<f32>;

@group(3) @binding(5) var linear: sampler;

fn surfaceLight(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec3f {
  let sun_1 = mix(data.settings.y, 1f, shadow);
  return lightAlgorithm(base, (emissive + castLight(base, normal, position)), roughness, geomRoughness, metal, ao, normal, position, sun_1, eye, data.sunDirection.xyz, data.sunRadiance.xyz, data.fill.xyz, data.settings.x, pmrem, dfg, linear);
}

fn equirectUv(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * 1.5915494309189535e-1f + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * 3.1830988618379069e-1f + 0.5);
}

fn applyAerial(surface: vec4f, position: vec3f, camera: vec3f, observer: vec3f, sun: vec3f, air: Aerial, lut: texture_2d<f32>, linear: sampler) -> vec4f {
    let reach=position-observer;
    let distM=length(reach)/max(1.0,distance(camera,observer)/air.overviewFromM);
    let distKm=max(distM/1000.0-air.clearRadiusKm,0.0);
    let rangeDepth=pow(max(distM-air.rangeNearM,0.0)/air.rangeFarM,air.rangePower)*air.rangeStrength;
    let heightMist=1.0-smoothstep(air.mistHeightM.x,air.mistHeightM.y,position.z);
    let farMist=smoothstep(air.mistDistanceM.x,air.mistDistanceM.y,distM);
    let mistWeight=heightMist*farMist;
    let transmit=exp(-(air.extinction*distKm+vec3f(rangeDepth)+vec3f(mistWeight*air.mistOpacity)));
    let view=normalize(position-camera);
    let viewSky=textureSample(lut,linear,equirectUv(normalize(vec3f(view.xy,max(view.z,4.0000000000000001e-3f))))).rgb;
    let horizonView=normalize(vec3f(view.x,view.y,4.0000000000000001e-3f));
    let horizonSky=textureSample(lut,linear,equirectUv(horizonView)).rgb;
    let below=smoothstep(air.horizonFade.x,0.0,view.z);
    let above=1.0-smoothstep(0.0,air.horizonFade.y,view.z);
    let sunMie=pow(clamp(dot(view,sun),0.0,1.0),air.sunPower)*air.sunStrength;
    let skyLight=mix(mix(viewSky,horizonSky,below*above),air.sunTint,sunMie);
    let mistSkyLight=mix(skyLight,air.mistColor,mistWeight*air.mistColorStrength);
    return vec4f(surface.rgb*transmit+mistSkyLight*(vec3f(1)-transmit),surface.a);
  }

@group(3) @binding(2) var sky: texture_2d<f32>;

fn air(colour: vec3f, position: vec3f, eye: vec3f) -> vec4f {
  return applyAerial(vec4f(colour, 1f), position, eye, data.observer.xyz, data.sunDirection.xyz, data.aerial, sky, linear);
}

fn shade(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec4f {
  return air(surfaceLight(base, emissive, roughness, geomRoughness, metal, ao, normal, position, shadow, eye), position, eye);
}

fn paintGlow(paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  return paintColour(paint) * paintStyle.emissive * paint.w;
}

fn paintedSeen(seen: f32, paint: vec4f) -> f32 {
  return mix(seen, mix(1.0, seen, paintStyle.fogKeep), paint.w);
}

fn fogCoverage(seen: f32, alpha: f32) -> vec4f {
  if (params.enabled == 0u || layer.seen == 1u) {
    return vec4f(0.0, 0.0, 0.0, alpha);
  }
  return vec4f(1.0 - seen, seen, f32(layer.ground), alpha);
}

struct terrainFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

@fragment fn terrainFragment(_arg_0: terrainFragment_Input, @builtin(position) clip: vec4f) -> terrainFragment_Output {
  if (!(scarRegionContains(_arg_0.world.xy))) {
    discard;;
  }
  let eye = (&cam.eye);
  let n = normalize(_arg_0.normal);
  let seen = fogTerm(_arg_0.world, n, clip.xy, fogIsGround());
  let footprint = length(fwidth(_arg_0.world.xy));
  let cell = groundCell(_arg_0.world.xy, footprint);
  let paved = groundPaved(_arg_0.world.xy, cell);
  if (groundClassView()) {
    let xy = _arg_0.world.xy;
    let site = groundSite(xy, cell, paved);
    let classes = groundClasses(xy, footprint, site, groundWater(xy, cell));
    return terrainFragment_Output(vec4f(classes, 1f), vec4f(0, 1, 1, 1));
  }
  let plain = groundAlbedo(_arg_0.world, _arg_0.color, cell, paved);
  let biome = (1f - _arg_0.color.w);
  let scar = groundScarsSeen(_arg_0.world, (*eye), footprint);
  let surface = mix(plain, scarredSurface(plain, scar), biome);
  let bank = groundBank(_arg_0.world.xy, cell, footprint);
  var ground = n;
  if ((bank.z > 0f)) {
    ground = normalize(mix(n, normalize(vec3f(-(bank.x), -(bank.y), 1f)), bank.z));
  }
  let relief = groundRoadRelief(_arg_0.world.xy, footprint, paved);
  ground = normalize((ground - (vec3f(relief, 0f) * (ground.z * biome))));
  let shading = normalize(mix(ground, scarredNormal(ground, scar), biome));
  let water = groundWater(_arg_0.world.xy, cell);
  let dry = (1f - smoothstep((-(footprint) * 0.5f), (footprint * 0.5f), water));
  let paint_1 = (groundPaint(_arg_0.world) * dry);
  let flecks = groundDapple(_arg_0.world.xy, footprint, cell);
  let sun_1 = max(sampleSunShadow(_arg_0.world, n, clip.xy), flecks);
  let lit = shade(paintedAlbedo(surface.xyz, paint_1), vec3f(), surface.w, 0f, 0f, 1f, shading, _arg_0.world, sun_1, (*eye));
  return terrainFragment_Output(vec4f((lit.xyz + paintGlow(paint_1)), 1f), fogCoverage(paintedSeen(seen, paint_1), 1f));
}