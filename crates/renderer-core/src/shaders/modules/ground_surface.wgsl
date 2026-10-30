// WGSL owner: ground_surface.

// @block ground_surface_fogHeight
fn fogHeight(q: vec2f) -> f32 {
  let P = fogParams;
  let nx = P.heightNx;
  let ny = P.heightNy;
  let fx = clamp(q.x / P.heightSpacing, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / P.heightSpacing, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u);
  let j = min(u32(fy), ny - 2u);
  let u = fx - f32(i);
  let v = fy - f32(j);
  return triangleRuleHeight(
    heightSample(i, j), heightSample(i + 1u, j),
    heightSample(i, j + 1u), heightSample(i + 1u, j + 1u), u, v);
}

// @block ground_surface_heightSample
fn heightSample(i: u32, j: u32) -> f32 {
    let size = readWord(0u);
    let cols = readWord(1u);
    let count = readWord(2u);
    if (count == 0u) { return 0.0; }
    let page = readWord(3u + (j / size) * cols + i / size);
    if (page == 0u) { return 0.0; }
    let at = 3u + count + (page - 1u) * size * size + (j % size) * size + i % size;
    return bitcast<f32>(readWord(at));
  }

// @block ground_surface_grassGround
fn grassGround(q: vec2f) -> f32 {
  let P = &grassParams;
  let nx = (*P).counts.x;
  let ny = (*P).counts.y;
  let fx = clamp(q.x / (*P).ground.x, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / (*P).ground.x, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u);
  let j = min(u32(fy), ny - 2u);
  return triangleRuleHeight(
    heightSample(i, j), heightSample(i + 1u, j),
    heightSample(i, j + 1u), heightSample(i + 1u, j + 1u),
    fx - f32(i), fy - f32(j));
}

// @block ground_surface_groundSite
fn groundSite(xy: vec2f, cell: vec4u, paved: GroundPaved) -> vec4f {
  let terrain = (&terrainParams);
  let warp = vec2f((valueNoise((xy * (*terrain).shape.y)) - 0.5f), (valueNoise(((xy * (*terrain).shape.y) + vec2f(19.700000762939453, 5.300000190734863))) - 0.5f));
  let q = (xy + (warp * ((*terrain).shape.x * 2f)));
  var edge = rectInside(q, (*terrain).region);
  var node = 0i;
  var leaf = 0i;
  for (var nodeStep = 0; (nodeStep < 64i); nodeStep++) {
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

// @block ground_surface_SurfaceRecord
struct SurfaceRecord {
  ends: vec4f,
  detail: vec4f,
}

// @block ground_surface_groundCell
fn groundCell(xy: vec2f, footprint: f32) -> vec4u {
 let P=terrainParams;
 var level=0u;var widest=P.field.w;
 while(footprint>widest&&level+1u<P.fieldGrid.z){widest*=2.0;level++;}
 let shift=surfaceIndex[level*2u+1u];
 let finest=clamp(vec2i(floor((xy-P.field.xy)*P.field.z)),vec2i(0),vec2i(P.fieldGrid.xy)-1);
 let cell=vec2u(finest)>>vec2u(shift);
 let cols=((P.fieldGrid.x-1u)>>shift)+1u;
 let at=surfaceIndex[level*2u]+(cell.y*cols+cell.x)*3u;
 return vec4u(surfaceIndex[at],surfaceIndex[at+1u],surfaceIndex[at+2u],surfaceIndex[at+3u]);
}

// @block ground_surface_groundForest
fn groundForest(xy: vec2f, cell: vec4u) -> f32 {
 var forest=-1e9;var distance=-1e9;var nearest=1e9;var inside=false;
 for(var i=cell.y;i<cell.z;i++){
  let entry=surfaceIndex[i];
  let record=surfaces[entry&GROUND_RECORD_MASK];
  let kind=entry>>GROUND_KIND_SHIFT;
  if((entry&GROUND_NEW_SHAPE)!=0u){
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

// @block ground_surface_groundPaved
fn groundPaved(xy: vec2f, cell: vec4u) -> GroundPaved {
 var paved=vec4f(-1e9);var member=ROAD_LOOKS_MAX;var nearest=1e9;var edge=0u;
 var lane=vec4f(0.0);var run=vec4f(0.0,0.0,0.0,-1.0);var deepest=-1e9;var bearing=-1.0;
 for(var i=cell.x;i<cell.y;i++){
  let entry=surfaceIndex[i];
  let seg=surfaces[entry&GROUND_RECORD_MASK];
  let kind=entry>>GROUND_KIND_SHIFT;
  if(kind==1u){
   let a=seg.ends.xy;let ab=seg.ends.zw-a;
   let t=clamp(dot(xy-a,ab)/max(dot(ab,ab),1e-6),0.0,1.0);
   let away=xy-(a+ab*t);let off=length(away);let own=u32(seg.detail.y);
   let inside=strokeCutInside(xy,seg.ends,seg.detail,seg.detail.x-off);
   paved[own]=max(paved[own],inside);
   let walk=terrainParams.roads[own].join.zw;
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
 if(member<ROAD_LOOKS_MAX){
  paved[member]=max(paved[member],nearest);rule=max(rule,nearest);
 }
 else if(nearest<1e9){
  paved[edge]=max(paved[edge],-nearest);rule=max(rule,-nearest);
 }
 return GroundPaved(paved,lane,run,rule,cell.xy,bearing);
}

// @block ground_surface_groundTreeLine
fn groundTreeLine(xy: vec2f, cell: vec4u) -> vec2f {
 var inside=-1e9;var band=-1e9;
 let taper=terrainParams.forestLine.x;
 for(var i=cell.y;i<cell.z;i++){
  let entry=surfaceIndex[i];
  if((entry>>GROUND_KIND_SHIFT)!=1u){continue;}
  let record=surfaces[entry&GROUND_RECORD_MASK];
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

// @block ground_surface_groundWater
fn groundWater(xy: vec2f, cell: vec4u) -> f32 {
 var bed=-1e9;
 for(var i=cell.z;i<cell.w;i++){
  let stretch=surfaces[surfaceIndex[i]&GROUND_RECORD_MASK];
  let a=stretch.ends.xy;let ab=stretch.ends.zw-a;let len2=dot(ab,ab);
  var t=0.0;if(len2>0.0){t=clamp(dot(xy-a,ab)/len2,0.0,1.0);}
  let half=stretch.detail.x+(stretch.detail.y-stretch.detail.x)*t;
  bed=max(bed,half-length(xy-(a+ab*t)));
 }
 return bed;
}

// @block ground_surface_surfaceIndex
@group(1) @binding(4) var<storage, read> surfaceIndex: array<u32>;

// @block ground_surface_surfaces
@group(1) @binding(3) var<storage, read> surfaces: array<SurfaceRecord>;
// @block ground_surface_classDistance
fn classDistance(inside: f32) -> f32 {
  return (clamp(round((128f - (inside * 8f))), 1f, 255f) / 255f);
}

// @block ground_surface_heights_grassBuildLayout
@group(0) @binding(1) var<storage, read> heights: array<u32>;

// @block ground_surface_heights_buildLayout
@group(0) @binding(2) var<storage, read> heights: array<u32>;

// @block ground_surface_readWord
fn readWord(at: u32) -> u32 {
  return heights[at];
}

// @block ground_surface_GroundCubic
struct GroundCubic {
  taps: vec4f,
  weights: vec4f,
}

// @block ground_surface_GroundPaved
struct GroundPaved {
  drawn: vec4f,
  lane: vec4f,
  run: vec4f,
  rule: f32,
  list: vec2u,
  bearing: f32,
}

// @block ground_surface_PlotNode
struct PlotNode {
  line: vec4f,
  children: vec4i,
}

// @block ground_surface_PlotRecord
struct PlotRecord {
  colour: vec4f,
  rows: vec4f,
  detail: vec4f,
  grain: vec4f,
  dry: vec4f,
  tram: vec4f,
}

// @block ground_surface_groundCubicBlend
fn groundCubicBlend(cubic: GroundCubic, a: vec4f, b: vec4f, c: vec4f, e: vec4f) -> vec4f {
  let g0 = cubic.weights.xy;
  let g1 = cubic.weights.zw;
  // Keep the original hardware cubic outer contraction explicit after inlining.
  return fma(vec4f(g0.y), g0.x * a + g1.x * b, g1.y * (g0.x * c + g1.x * e));
}

// @block ground_surface_groundCubicTap
fn groundCubicTap(cubic: GroundCubic, k: u32) -> vec2f {
  return vec2f(select(cubic.taps.x, cubic.taps.z, (k & 1u) == 1u), select(cubic.taps.y, cubic.taps.w, k >= 2u));
}

// @block ground_surface_nodes
@group(1) @binding(1) var<storage, read> nodes: array<PlotNode>;

// @block ground_surface_plots
@group(1) @binding(2) var<storage, read> plots: array<PlotRecord>;

// @block ground_surface_terrain
@group(0) @binding(6) var<storage, read_write> terrain: array<u32>;

// @block ground_surface_groundCubic
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
