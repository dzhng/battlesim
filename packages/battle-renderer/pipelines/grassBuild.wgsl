struct GrassParams {
  planes: array<vec4f, 4>,
  eye: vec4f,
  window: vec4f,
  grid: vec4u,
  density: vec4f,
  tiers: vec4f,
  shading: vec4f,
  clear: vec4f,
  counts: vec4u,
  ground: vec4f,
  sway: vec4f,
  growth: array<vec4f, 16>,
  patches: array<vec4f, 16>,
  mixes: array<vec4f, 64>,
  patchScales: vec4f,
  grain: vec4f,
  kinds: array<vec4f, 16>,
  bases: array<vec4u, 16>,
}

@group(0) @binding(0) var<uniform> params: GrassParams;

fn triangleRuleHeight(h00: f32, h10: f32, h01: f32, h11: f32, u: f32, v: f32) -> f32 {
  if (u >= v) { return h00 + u * (h10 - h00) + v * (h11 - h10); }
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}

@group(0) @binding(1) var<storage, read> heights: array<u32>;

fn readWord(at: u32) -> u32 {
  return heights[at];
}

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

fn grassDensity(dist: f32, drop: f32) -> f32 {
  let P = &params;
  let facing = clamp(drop / dist, 0.15, 1.0);
  let footprint = (*P).eye.w * dist;
  return min((*P).density.y, facing / ((*P).density.x * footprint * footprint));
}

fn pcgHash(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}

fn grassHash(a: u32, b: u32) -> vec3f {
  var s = a * 747796405u + b * 2891336453u + 12345u;
  var out = vec3f(0.0);
  for (var k = 0u; k < 3u; k++) {
    out[k] = f32(pcgHash(s) >> 8u) * (1.0 / 16777216.0);
    s = s * 747796405u + 2891336453u;
  }
  return out;
}

struct GrassClump {
  root: vec3f,
  height: f32,
  colour: u32,
  kind: u32,
  yawSin: f32,
  yawCos: f32,
  width: f32,
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

fn forestVergeInside(xy: vec2f, forest: f32) -> f32 {
  let verge = (&params_1.forestVerge);
  let wander = ((valueNoise(((xy * (*verge).z) + vec2f(71.30000305175781, 23.899999618530273))) - 0.5f) * 2f);
  let patches = (valueNoise(((xy * 0.7f) + vec2f(5.099999904632568, 91.69999694824219))) - 0.5f);
  return (((forest + ((*verge).x * 0.5f)) + (wander * (*verge).y)) + (patches * (*verge).x));
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

struct PlotRecord {
  colour: vec4f,
  rows: vec4f,
  detail: vec4f,
  grain: vec4f,
  dry: vec4f,
  tram: vec4f,
}

@group(1) @binding(2) var<storage, read> plots: array<PlotRecord>;

fn groundVerge(site: vec4f, footprint: f32) -> f32 {
  let params_2 = (&params_1);
  let vergeHalf = (*params_2).verge.w;
  return (1f - smoothstep(vergeHalf, (vergeHalf + max((*params_2).feathers.x, footprint)), site.y));
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

struct ScarSample {
  weights: vec4f,
  relief: vec4f,
  at: vec4f,
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

fn groundShoulderGrass(worn: vec2f) -> f32 {
 return mix(1.0,params_1.roads[u32(worn.y)].edge.z,worn.x);
}

@group(0) @binding(2) var<storage, read> props: array<vec4f>;

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

fn groundTint(albedo: vec3f, dry: f32) -> vec3f {
  let tinted = (albedo * (vec3f(1) + (vec3f(0.699999988079071, 0, -0.8999999761581421) * dry)));
  return (tinted * (dot(albedo, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)) / max(dot(tinted, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)), 1e-5f)));
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

fn forestFloorWeight(xy: vec2f, forest: f32, footprint: f32) -> f32 {
  let feather = max(footprint, (params_1.forestVerge.x * 0.1f));
  return smoothstep(-(feather), feather, forestVergeInside(xy, forest));
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

fn forestFloor(xy: vec2f, footprint: f32, noise: f32) -> vec3f {
  let params_2 = (&params_1);
  let detail = (&(*params_2).forestDetail);
  let at = (xy * (*detail).x);
  let turned_1 = vec2f(((at.x * 0.8f) - (at.y * 0.6f)), ((at.x * 0.6f) + (at.y * 0.8f)));
  let across = vec2f(((at.x * 0.28f) + (at.y * 0.96f)), ((at.y * 0.28f) - (at.x * 0.96f)));
  let moss = smoothstep(0.38f, 0.72f, ((valueNoise((turned_1 + vec2f(13.300000190734863, 7.699999809265137))) * 0.65f) + (valueNoise(((across * 2.1f) + vec2f(4.099999904632568, 9.199999809265137))) * 0.35f)));
  let humus = smoothstep(0.42f, 0.78f, ((valueNoise(((across * 1.3f) + vec2f(2.9000000953674316, 41.099998474121094))) * 0.6f) + (valueNoise(((turned_1 * 3.1f) + vec2f(17.299999237060547, 5.900000095367432))) * 0.4f)));
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

fn atLeast(colour: vec3f, least: f32) -> vec3f {
  let luminance = dot(colour, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023));
  return (colour * max(1f, (least / max(luminance, 1e-5f))));
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

@group(0) @binding(4) var<storage, read_write> args: array<atomic<u32>, 12>;

@group(0) @binding(3) var<storage, read_write> clumps: array<GrassClump>;

@compute @workgroup_size(64) fn buildFn(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32) {
  let P = &params;
  if (wg.x >= (*P).grid.x || wg.y >= (*P).grid.y) { return; }
  let T = (*P).window.z;
  let lo = (*P).window.xy + vec2f(f32(wg.x), f32(wg.y)) * T;
  let hi = lo + vec2f(T);
  // The tile's box: its ground at the corners and centre, a metre either
  // way, and the tallest clump above.
  var zlo = 1e9;
  var zhi = -1e9;
  for (var k = 0u; k < 5u; k++) {
    var q = (lo + hi) * 0.5;
    if (k < 4u) { q = vec2f(select(lo.x, hi.x, (k & 1u) == 1u), select(lo.y, hi.y, k >= 2u)); }
    let z = grassGround(q);
    zlo = min(zlo, z);
    zhi = max(zhi, z);
  }
  zlo -= 1.0;
  zhi += 1.0 + (*P).window.w;
  let lo2 = lo - vec2f(0.4 + (*P).clear.y);
  let hi2 = hi + vec2f(0.4 + (*P).clear.y);
  for (var i = 0u; i < 4u; i++) {
    let pl = (*P).planes[i];
    let pv = vec3f(select(lo2.x, hi2.x, pl.x >= 0.0), select(lo2.y, hi2.y, pl.y >= 0.0), select(zlo, zhi, pl.z >= 0.0));
    if (dot(pl.xyz, pv) + pl.w < 0.0) { return; }
  }
  let eye = (*P).eye.xyz;
  let nearest = clamp(eye, vec3f(lo2, zlo), vec3f(hi2, zhi));
  let dNear = max(distance(eye, nearest), 0.5);
  if ((*P).eye.w * dNear >= (*P).density.w) { return; }
  let n = u32(ceil(T * T * grassDensity(dNear, max(eye.z - zlo, 0.0))));
  // The tile's own offset of the sequence, from its place on the world grid.
  let cell = vec2i(floor(lo / T + 0.5));
  let offset = grassHash(bitcast<u32>(cell.x), bitcast<u32>(cell.y)).xy;
  for (var j = li; j < n; j += 64u) {
    // The sequence's point, jittered by a fixed amount so a sparse field
    // shows no lattice; both depend on the tile and j alone.
    let h = grassHash(bitcast<u32>(cell.x) ^ (j * 2654435761u), bitcast<u32>(cell.y) + j);
    let jitter = (vec2f(fract(h.z * 7.13), fract(h.x * 5.31)) - 0.5) * 0.8;
    let fell = lo + T * fract(offset + f32(j + 1u) * vec2f(0.7548776662466927, 0.5698402909980532)) + jitter;
    let fellZ = grassGround(fell);
    let dist = max(distance(eye, vec3f(fell, fellZ)), 0.5);
    let footprint = (*P).eye.w * dist;
    if (footprint >= (*P).density.w) { continue; }
    // GrassClump j stands once the density passes (j + 1/2) per tile.
    let rank = (f32(j) + 0.5) / (T * T);
    let rho = grassDensity(dist, max(eye.z - fellZ, 0.0));
    if (rank >= rho) { continue; }
    let cell = groundCell(fell, footprint);
    let paved = groundPaved(fell, cell);
    let site = groundSite(fell, cell, paved);
    let water = groundWater(fell, cell);
    // Bare within the margins; thinner and lower for a metre beyond them, so
    // a field meets a road or a wood without a wall of blades. Across a
    // road's shoulder it thins and lowers further as the ground is worn.
    // A wood's edge is its rect or its floor's ragged verge, whichever lies
    // farther out. A tree line is no wood: the verge's grass grows on the
    // band under it, and the field's own right up to that.
    var wood = max(site.w, forestVergeInside(fell, site.w));
    var line = vec2f(0.0, -1e9);
    if (wood > -((*P).clear.x + 1.2)) {
      line = groundLineBand(fell, cell, site.w);
      if (line.x > 0.5) { wood = -1e9; }
    }
    let margin = min(-wood, -water) - (*P).clear.x;
    if (margin < 0.0) { continue; }
    // Bare on the wet bank round water, thickening across the earth behind it.
    let shore = groundShore(fell, footprint, water).z;
    if (shore >= 1.0) { continue; }
    let worn = groundShoulder(fell, footprint, paved);
    // Bare on a road's own surface and at the foot of its shoulder, but for
    // a track's centre strip.
    if (worn.x > 0.9) { continue; }
    let edge = smoothstep(0.0, 1.2, min(margin, abs(site.z))) * (1.0 - worn.x);
    var grows = min(u32(plots[i32(site.x)].detail.y), 14u);
    if (groundVerge(site, footprint) > 0.5 || line.y > 0.0) { grows = 15u; }
    let g = (*P).growth[grows];
    let stand = (*P).patches[grows];
    // A drilled crop keeps to its plot's rows, the bright bands between the
    // furrows the ground is painted with: the clump moves across them toward
    // the nearest, but never out of its plot or into a bare margin.
    let plot = plots[i32(site.x)];
    var p = fell;
    if (g.w > 0.0 && plot.rows.z > 0.0) {
      let across = dot(fell, plot.rows.xy) / plot.rows.z;
      let toRow = (floor(across) + 0.5 - across) * plot.rows.z * g.w;
      let room = max(0.0, min(site.y - params_1.verge.w, margin) - 0.3);
      p = fell + plot.rows.xy * clamp(toRow, -room, room);
    }
    // A wheeling stays bare.
    if (groundWheeling(p, 0.0, i32(site.x)) > 0.5) { continue; }
    let root = vec3f(p, grassGround(p));
    // The side's learned scars: craters, scorch and tracks leave clumps out.
    let scar = groundScars(p, footprint);
    let S = scarParams.grass;
    let bare = max(max(scar.weights.x, scar.weights.y), scar.weights.z * S.y) * S.x;
    // Sparse patches: the stand thins there and nowhere thickens.
    let sparse = 1.0 - smoothstep(0.3, 0.55, valueNoise(p * (*P).patchScales.y + vec2f(41.7, 13.1)));
    let keep = rho * g.x * (1.0 - stand.z * sparse) * mix(0.5, 1.0, edge) * (1.0 - bare) * (1.0 - shore) * groundShoulderGrass(worn);
    if (rank >= keep) { continue; }
    if (grassUnderProp(p)) { continue; }
    // A clump nearing its rank's threshold is small: it grows as the
    // density passes it, and shrinks away as the fade takes it.
    let grow = 1.0 - smoothstep(0.7 * keep, keep, rank);
    let fade = 1.0 - smoothstep((*P).density.z, (*P).density.w, footprint);
    // Which of the row's grasses this clump is: its hash against their
    // shares, a drifting grass weighted up inside its drifts and down to
    // nothing outside them.
    var shares = array<f32, 4>();
    var total = 0.0;
    let mixed = u32(g.z);
    for (var i = 0u; i < mixed; i++) {
      let entry = (*P).mixes[grows * 4u + i];
      var share = entry.y;
      if (entry.z > 0.0) {
        let drift = smoothstep(0.4, 0.65, valueNoise(p * (*P).patchScales.w + vec2f(f32(i) * 17.3 + 3.1, f32(grows) * 7.9)));
        share *= mix(1.0, 3 * drift, entry.z);
      }
      shares[i] = share;
      total += share;
    }
    var chosen = 0u;
    if (total > 0.0) {
      chosen = mixed - 1u;
      var roll = fract(h.x * 91.37) * total;
      for (var i = 0u; i + 1u < mixed; i++) {
        if (roll < shares[i]) { chosen = i; break; }
        roll -= shares[i];
      }
    }
    let grass = (*P).mixes[grows * 4u + chosen];
    let kind = u32(grass.x);
    let row = (*P).kinds[kind];
    // Patches a few metres across stand taller or lower, as uneven grass does.
    let tall = valueNoise(p * (*P).patchScales.x);
    let height = row.x * g.y * grow * fade * mix(0.8, 1.2, h.x) * mix(stand.x, stand.y, tall) * mix(0.35, 1.0, edge);
    if (height < 0.02) { continue; }
    let centre = root + vec3f(0.0, 0.0, height * 0.5);
    let radius = height * 0.8 + 0.4;
    var shown = true;
    for (var i = 0u; i < 4u; i++) {
      let pl = (*P).planes[i];
      if (dot(pl.xyz, centre) + pl.w < -radius) { shown = false; }
    }
    if (!shown) { continue; }
    var tier = 1u;
    if (height / footprint > (*P).tiers.x * mix(0.85, 1.15, h.y)) { tier = 0u; }
    let width = max(1.0, (*P).tiers.y * footprint / max(row.z, 1e-4));
    // Dry patches, and a grass drier than its neighbours: toward straw at
    // the ground's own luminance, so no patch is darker than its field.
    let parched = smoothstep(0.55, 0.8, valueNoise(p * (*P).patchScales.z + vec2f(9.3, 77.1)));
    // What sets a clump apart from the ground under it (how dry it is, its
    // grain) goes with the clump as the fade takes it, so the field's far
    // edge meets the painted ground with the ground's own colour.
    let dryness = min(1.0, stand.w * parched + grass.w) * fade;
    // Each clump a little lighter or darker than the next, half on its own
    // and half with the tussock beneath it: the grain a field keeps when
    // it is too far to show blades. Dry grass is paler, never darker.
    let tussock = valueNoise(p * (*P).grain.x + vec2f(63.9, 27.4));
    let value = 1.0 + (*P).shading.z * fade * (fract(h.y * 57.31) + tussock - 1.0);
    let colour = value * (1.0 + (*P).grain.y * dryness) * groundTint(scarredSurface(groundColour(p, footprint, site, paved, water), scar).xyz, dryness);
    // Tracks and trampling lay the clump over (carried by the colour's alpha).
    let flat = max(scar.weights.z, scar.weights.w) * S.z;
    let slot = atomicAdd(&args[tier * 5u + 1u], 1u);
    let cap = select((*P).grid.w, (*P).grid.z, tier == 0u);
    if (slot >= cap) { continue; }
    let base = select((*P).grid.z, 0u, tier == 0u);
    clumps[base + slot] = GrassClump(
      root, height, pack4x8unorm(vec4f(sqrt(max(colour, vec3f(0.0))), 1.0 - flat)), kind,
      sin(h.z * 6.2831853), cos(h.z * 6.2831853), width);
  }
}