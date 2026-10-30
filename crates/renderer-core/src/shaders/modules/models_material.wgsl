// WGSL owner: models_material.

// @block models_material_ModelSurface
struct ModelSurface {
  albedo: vec3f,
  normal: vec3f,
  roughness: f32,
  metallic: f32,
  occlusion: f32,
  tint: f32,
}

// @block models_material_SurfaceRecord
struct SurfaceRecord {
  ends: vec4f,
  detail: vec4f,
}

// @block models_material__Input
struct _Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

// @block models_material_applyAerial
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

// @block models_material_coverageValue
fn coverageValue(material: u32, uv: vec2f) -> f32 {
  let row = (material * 5u);
  let layer = materials[(row + 4u)].y;
  let image = textureSample(surface, tiled, uv, i32(max(layer, 0f)));
  var value = materials[row].w;
  if ((layer >= 0f)) {
    value = (value * image.w);
  }
  return value;
}

// @block models_material_cutoutCoverage
fn cutoutCoverage(material: u32, uv: vec2f) -> f32 {
  let cutoff = materials[((material * 5u) + 4u)].x;
  return saturate(((coverageValue(material, uv) * 0.5f) / max(cutoff, 1e-3f)));
}

// @block models_material_groundAlbedo
fn groundAlbedo(world: vec3f, tint: vec4f, cell: vec4u, paved: GroundPaved) -> vec4f {
  let footprint = length(fwidth(world.xy));
  let xy = world.xy;
  let site = groundSite(xy, cell, paved);
  let surface = groundColour(xy, footprint, site, paved, groundWater(xy, cell));
  let albedo = mix(surface.xyz, srgbToLinear(tint.xyz), tint.w);
  return vec4f(albedo, mix(surface.w, 0.85f, tint.w));
}

// @block models_material_groundBank
const GROUND_BANK_BLEND_REACH_M = 50;
const GROUND_BANK_BLEND_M = 0.7;
const GROUND_BANK_MAX_SLOPE = 0.84;
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
  let ease=max(footprint,GROUND_BANK_EASE_M);
  let sloped=(1.0-smoothstep(run-ease,run+ease,-inside))*(1.0-smoothstep(0.0,ease,inside));
  let top=run+params_1.waterBed.w;
  let weight=exp(clamp(inside,-GROUND_BANK_BLEND_REACH_M,GROUND_BANK_BLEND_REACH_M)/GROUND_BANK_BLEND_M);
  let shown=min(grade*params_1.shoreEdge.w,GROUND_BANK_MAX_SLOPE);
  sum+=vec4f(away/max(off,1e-4)*shown*sloped,1.0-smoothstep(top,top+2.0*ease,-inside),1.0)*weight;
 }
 if(sum.w<=0.0){return vec3f(0.0);}
 return sum.xyz/sum.w;
}

// @block models_material_groundCell_alternate
fn groundCell(xy: vec2f, footprint: f32) -> vec4u {
 let P=params;
 var level=0u;var widest=P.field.w;
 while(footprint>widest&&level+1u<P.fieldGrid.z){widest*=2.0;level++;}
 let shift=surfaceIndex[level*2u+1u];
 let finest=clamp(vec2i(floor((xy-P.field.xy)*P.field.z)),vec2i(0),vec2i(P.fieldGrid.xy)-1);
 let cell=vec2u(finest)>>vec2u(shift);
 let cols=((P.fieldGrid.x-1u)>>shift)+1u;
 let at=surfaceIndex[level*2u]+(cell.y*cols+cell.x)*3u;
 return vec4u(surfaceIndex[at],surfaceIndex[at+1u],surfaceIndex[at+2u],surfaceIndex[at+3u]);
}

// @block models_material_groundCurb_alternate
const GROUND_CURB_PIXELS_FROM = 0.25;
const GROUND_CURB_PIXELS_TO = 0.6;
const GROUND_CURB_EDGE_M = 0.03;
fn groundCurb(footprint: f32, paved: GroundPaved) -> vec3f {
 if(paved.run.w<0.0||params.view.y==1u){return vec3f(0.0);}
 let own=u32(paved.run.w);
 let look=params.roads[own];
 let width=look.curb.w;
 if(width<=0.0){return vec3f(0.0);}
 let inside=paved.drawn[own];
 if(inside<-2.0*width||inside>2.0*look.slabs.z){return vec3f(0.0);}
 let shown=1.0-smoothstep(GROUND_CURB_PIXELS_FROM,GROUND_CURB_PIXELS_TO,footprint/width);
 if(shown<=0.0){return vec3f(0.0);}
 let soft=max(footprint,GROUND_CURB_EDGE_M)*0.5;
 // Another carriageway's surface over the edge leaves no curb there.
 var open=1.0;
 for(var k=0u;k<4u;k++){
  if(k!=own&&params.roads[k].track.w>0.0){open*=1.0-smoothstep(-soft,soft,paved.drawn[k]);}
 }
 let edge=smoothstep(-soft,soft,inside);
 let stones=smoothstep(-width-soft,-width+soft,inside)*(1.0-edge);
 let face=edge*(1.0-smoothstep(look.slabs.z-soft,look.slabs.z+soft,inside));
 return vec3f(paved.lane.xy*(look.slabs.w*face),stones)*(open*shown);
}

// @block models_material_groundForest
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

// @block models_material_groundMarks_alternate
const GROUND_MARKS_PIXELS_FROM = 0.35;
const GROUND_MARKS_PIXELS_TO = 0.85;
const GROUND_MARKS_CROSSING_COS = 0.8;
const GROUND_MARKS_EDGE_M = 0.03;
fn groundMarks(xy: vec2f, footprint: f32, paved: GroundPaved) -> f32 {
 let kind=roadLane(paved);
 if(kind<0||params.view.y==1u){return 0.0;}
 let look=params.roads[kind];
 let line=look.marks.x;
 if(line<=0.0){return 0.0;}
 let bars=look.crossing;
 let shown=vec2f(1.0)-smoothstep(vec2f(GROUND_MARKS_PIXELS_FROM),vec2f(GROUND_MARKS_PIXELS_TO),footprint/vec2f(2.0*line,bars.x));
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
  if(params.roads[u32(seg.detail.y)].track.w<=0.0){continue;}
  let a=seg.ends.xy;let ab=seg.ends.zw-a;let len=max(length(ab),1e-6);
  if(abs(dot(ab,paved.run.xy))>GROUND_MARKS_CROSSING_COS*len){continue;}
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
 let soft=max(footprint,GROUND_MARKS_EDGE_M)*0.5;
 let centre=(1.0-smoothstep(line-soft,line+soft,paved.lane.z))
  *(1.0-smoothstep(look.marks.y*0.5-soft,look.marks.y*0.5+soft,abs(dash)))
  *(1.0-step(-far-GROUND_MARKS_CLEAR_M,near));
 let zone=smoothstep(-far-soft,-far+soft,cross)*(1.0-smoothstep(-bars.z-soft,-bars.z+soft,cross));
 let across=abs(fract(paved.lane.z/(2.0*bars.x))-0.5)*2.0*bars.x;
 let bar=1.0-smoothstep(bars.x*0.5-soft,bars.x*0.5+soft,across);
 let kept=1.0-smoothstep(paved.lane.w-bars.w-soft,paved.lane.w-bars.w+soft,paved.lane.z);
 return max(centre*shown.x,zone*bar*kept*shown.y);
}

// @block models_material_groundPaved_alternate
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
   let walk=params.roads[own].join.zw;
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

// @block models_material_groundTreeLine_alternate
fn groundTreeLine(xy: vec2f, cell: vec4u) -> vec2f {
 var inside=-1e9;var band=-1e9;
 let taper=params.forestLine.x;
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

// @block models_material_groundWater
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

// @block models_material_importanceGGX
fn importanceGGX(xi: vec2f, r: f32) -> vec3f {
  let alpha=r*r;
  let radius=sqrt(xi.x);
  let phi=(2.0*3.14159265359)*xi.y;
  let t1=radius*cos(phi);
  // PMREM views along the surface normal: tangent-space V=(0,0,1), s=1.
  let t2=radius*sin(phi);
  let nh=vec3f(t1,t2,sqrt(max(0.0,1.0-(t1*t1+t2*t2))));
  return normalize(vec3f(alpha*nh.x,alpha*nh.y,max(0.0,nh.z)));
}

// @block models_material_lit_Input
struct lit_Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

// @block models_material_materials
@group(1) @binding(1) var<storage, read> materials: array<vec4f>;

// @block models_material_modelCutoutCaster_Input
struct modelCutoutCaster_Input {
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
}

// @block models_material_modelCutoutDepth_Input
struct modelCutoutDepth_Input {
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
}

// @block models_material_modelSurface
fn modelSurface(color: vec4f, material: u32, track: f32, uv: vec2f, normal: vec3f, tangent: vec4f) -> ModelSurface {
  let row = (material * 5u);
  let base = (&materials[row]);
  let factors = (&materials[(row + 1u)]);
  let layers = (&materials[(row + 2u)]);
  let worn = (&materials[(row + 3u)]);
  let a = textureSample(albedo, tiled, uv, i32(max((*layers).x, 0f)));
  let nm = textureSample(surface, tiled, uv, i32(max((*layers).y, 0f)));
  let orm = textureSample(surface, tiled, uv, i32(max((*layers).z, 0f)));
  var albedo_1 = ((color.xyz * (*factors).w) * (*base).xyz);
  var threshold = 0.5f;
  if (((*layers).x >= 0f)) {
    albedo_1 = (albedo_1 * a.xyz);
    threshold = a.w;
  }
  else {
    if (((*layers).x <= -2f)) {
      let mean = textureSampleLevel(albedo, tiled, vec2f(0.5), i32((-2f - (*layers).x)), 16);
      albedo_1 = (albedo_1 * mean.xyz);
    }
    else {
      if (((track >= 1f) && ((track - 1f) < 0.5f))) {
        albedo_1 = (albedo_1 * 0.55f);
      }
    }
  }
  var roughness = (*factors).y;
  var metallic = (*factors).x;
  var occlusion = 1f;
  var tint = (*factors).z;
  if (((*layers).z >= 0f)) {
    occlusion = orm.x;
    roughness = (roughness * orm.y);
    metallic = (metallic * orm.z);
    tint = (tint * orm.w);
  }
  var n = normal;
  let tangentLength = length(tangent.xyz);
  if ((((*layers).y >= 0f) && (tangentLength > 0.5f))) {
    let t = normalize((tangent.xyz - (n * dot(n, tangent.xyz))));
    let b = (cross(n, t) * tangent.w);
    let m = ((nm.xyz * 2f) - vec3f(1));
    n = normalize((((t * m.x) + (b * m.y)) + (n * m.z)));
  }
  if (((*layers).w > 0f)) {
    let w = smoothstep((threshold - 0.04f), (threshold + 0.04f), color.w);
    albedo_1 = mix(albedo_1, (*worn).xyz, w);
    roughness = mix(roughness, (*worn).w, w);
    metallic = mix(metallic, 0f, w);
  }
  return ModelSurface(albedo_1, n, roughness, metallic, occlusion, tint);
}

// @block models_material_modelXrayCountFragment_Input
struct modelXrayCountFragment_Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

// @block models_material_modelXrayFragment_Input
struct modelXrayFragment_Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

// @block models_material_roomLookup
fn roomLookup(uv: vec2f, pick: u32, cellTexels: f32) -> vec3f {
    let over = max(max(-uv, uv - vec2f(1.0)), vec2f(0.0));
    let depth = 1.0 - max(over.x, over.y);
    let across = clamp(uv, vec2f(0.0), vec2f(1.0)) - vec2f(0.5);
    let shrink = 16.0 / (16.0 + 4.5 * depth);
    var at = vec2f(0.5) + across * shrink;
    if (((pick >> 16u) & 1u) == 1u) { at.x = 1.0 - at.x; }
    let footprint = max(length(dpdx(at)), length(dpdy(at))) * cellTexels;
    let level = clamp(log2(max(footprint, 0.000001)), 0.0, log2(cellTexels));
    let inset = 0.5 * exp2(ceil(level)) / cellTexels;
    at = clamp(at, vec2f(inset), vec2f(1.0 - inset));
    let cell = pick % 10u;
    let corner = vec2f(f32(cell % 4u), f32(cell / 4u));
    return vec3f((corner + at) / 4.0, level);
  }

// @block models_material_roomPick
fn roomPick(anchor: vec3f) -> u32 {
    let bits = bitcast<vec3u>(anchor);
    var h = 0u;
    for (var i = 0; i < 3; i++) {
      // PCG's output permutation, chained over the three coordinates.
      let state = (h ^ bits[i]) * 747796405u + 2891336453u;
      let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
      h = (word >> 22u) ^ word;
    }
    return h;
  }

// @block models_material_scarredSurface
const GROUND_SCAR_SURFACE_GRAIN_SCALE = 1.7;
const GROUND_SCAR_SURFACE_FLECK_SCALE = 6.3;
const GROUND_SCAR_SURFACE_TRAMPLED_FROM = 0.9;
const GROUND_SCAR_SURFACE_TRAMPLED_TO = 1.1;
const GROUND_SCAR_SURFACE_TRACKS_FROM = 0.85;
const GROUND_SCAR_SURFACE_TRACKS_TO = 1.1;
const GROUND_SCAR_SURFACE_ASH_FROM = 0.6;
const GROUND_SCAR_SURFACE_ASH_TO = 2.4;
const GROUND_SCAR_SURFACE_ASH_BROAD_WEIGHT = 0.6;
const GROUND_SCAR_SURFACE_ASH_DRIFT_SCALE = 0.7;
const GROUND_SCAR_SURFACE_ASH_FINE_WEIGHT = 0.4;
const GROUND_SCAR_SURFACE_SOIL_FROM = 0.8;
const GROUND_SCAR_SURFACE_SOIL_TO = 1.15;
const GROUND_SCAR_SURFACE_CAVITY = 0.7;
fn scarredSurface(surface: vec4f, scar: ScarSample) -> vec4f {
  let P = scarParams;
  let w = scar.weights;
  let xy = scar.at.xy;
  var albedo = surface.xyz;
  var roughness = surface.w;
  let grain = valueNoise(xy * GROUND_SCAR_SURFACE_GRAIN_SCALE);
  let fleck = valueNoise(xy * GROUND_SCAR_SURFACE_FLECK_SCALE + vec2f(2.2, 9.1));
  albedo = mix(albedo, P.trampled.xyz * mix(GROUND_SCAR_SURFACE_TRAMPLED_FROM, GROUND_SCAR_SURFACE_TRAMPLED_TO, grain), w.w * P.trampled.w);
  albedo = mix(albedo, P.tracks.xyz * mix(GROUND_SCAR_SURFACE_TRACKS_FROM, GROUND_SCAR_SURFACE_TRACKS_TO, grain), w.z * P.tracks.w);
  // Soot is uneven: ash grey and char black in flecks and drifts.
  let ash = mix(GROUND_SCAR_SURFACE_ASH_FROM, GROUND_SCAR_SURFACE_ASH_TO, fleck * GROUND_SCAR_SURFACE_ASH_BROAD_WEIGHT + valueNoise(xy * GROUND_SCAR_SURFACE_ASH_DRIFT_SCALE + vec2f(4.4, 0.3)) * GROUND_SCAR_SURFACE_ASH_FINE_WEIGHT);
  albedo = mix(albedo, P.scorch.xyz * ash, w.y * P.scorch.w);
  let ring = scar.relief.z * P.ejecta.w;
  albedo = mix(albedo, P.ejecta.xyz * mix(GROUND_SCAR_SURFACE_SOIL_FROM, GROUND_SCAR_SURFACE_SOIL_TO, fleck), ring);
  let bowl = w.x * P.crater.w;
  // The deeper, the darker: the floor sees less sky than the lip.
  let cavity = mix(1.0, GROUND_SCAR_SURFACE_CAVITY, saturate(scar.relief.w));
  albedo = mix(albedo, P.crater.xyz * mix(GROUND_SCAR_SURFACE_SOIL_FROM, GROUND_SCAR_SURFACE_SOIL_TO, fleck) * cavity, bowl);
  // Churned soil and ash are fully rough: a tilted wall never catches sky.
  roughness = mix(roughness, 1.0, max(w.y * P.scorch.w, max(bowl, ring)));
  return vec4f(albedo, roughness);
}

// @block models_material_surface
@group(1) @binding(3) var surface: texture_2d_array<f32>;

// @block models_material_surfaceIndex
@group(1) @binding(4) var<storage, read> surfaceIndex: array<u32>;

// @block models_material_surfaces
@group(1) @binding(3) var<storage, read> surfaces: array<SurfaceRecord>;
