// WGSL owner: ground_roads.

// @block ground_roads_areaJoint
fn areaJoint(along: f32, perMetre: f32, footprint: f32, width: f32) -> f32 {
 let drawn=max(width,footprint);
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(drawn*0.5-footprint*0.5,drawn*0.5+footprint*0.5,to))*(width/drawn);
}

// @block ground_roads_groundRoads_alternate
const GROUND_SHOULDER_GRAIN = 0.5;
const GROUND_SURFACE_JOIN_DRIFT_PER_M = 0.4;
const GROUND_ROADS_PATCH_FROM = 0.52;
const GROUND_ROADS_PATCH_TO = 0.66;
const GROUND_STRIP_COVER = 0.6;
const GROUND_STRIP_GRAIN = 0.5;
const GROUND_AREA_BORDER_M = 2;
const GROUND_ROADS_AREA_JOINT_M = 0.1;
const GROUND_ROADS_SLAB_PIXELS_FROM = 0.041666666666666664;
const GROUND_AREA_SLAB_PIXELS_TO = 0.4;
const GROUND_ROADS_AREA_EDGE_DEPTH = 1.5;
const GROUND_MARKS_WEAR_FROM = 0.4;
const GROUND_MARKS_WEAR_TO = 0.6;
const GROUND_ROADS_MARKS_WEAR_PER_M = 0.7692307692307692;
const GROUND_ROADS_SLAB_JOINT_M = 0.03;
fn groundRoads(xy: vec2f, footprint: f32, paved: GroundPaved, under: vec4f) -> vec4f {
 var surface=under;
 let plain=params.view.y==1u;
 let worn=groundShoulder(xy,footprint,paved);
 if(worn.x>0.0){
  let look=params.roads[u32(worn.y)];
  let luma=vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B);
  let lift=max(1.0,dot(under.xyz,luma)/max(dot(look.shoulder.xyz,luma),1e-5));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w)*GROUND_SHOULDER_GRAIN;
  surface=mix(under,vec4f(look.shoulder.xyz*lift*(1.0+look.shape.z*grain),look.core.w),worn.x*look.edge.w);
 }
 let lane=roadLane(paved);
 // How far the roads already laid here are carried onto the next: each
 // one's own carry, by how much of the ground it covers.
 var carried=0.0;
 for(var n=0u;n<4u;n++){
  let k=params.roadOrder[n];
  let look=params.roads[k];
  var feather=max(look.shape.x,footprint)*0.5;
  var inside=paved.drawn[k];
  if(carried>0.0){
   // The blend starts at this road's edge and runs inward, from a line
   // that wanders as far either way: gravel spilt over asphalt in drifts,
   // never a ruled end across a road.
   feather=max(feather,carried*0.5);
   inside-=feather;
   if(!plain){inside+=(wanderNoise(xy*GROUND_SURFACE_JOIN_DRIFT_PER_M+vec2f(7.9,52.3))-0.5)*2.0*carried;}
  }
  if(inside<=-feather){continue;}
  var core=look.core.xyz;
  if(!plain){
   let hue=smoothstep(GROUND_ROADS_PATCH_FROM,GROUND_ROADS_PATCH_TO,wanderNoise(xy*look.shape.y+vec2f(61.7,17.3)))*look.worn.w;
   let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
   core=mix(core,look.worn.xyz,hue)*(1.0+look.shape.z*grain);
   if(i32(k)==lane){
    core*=1.0+groundRuts(xy,footprint,paved).z;
    let strip=groundStrip(xy,footprint,paved)*GROUND_STRIP_COVER;
    core=mix(core,params.verge.xyz*(1.0+look.shape.z*grain*GROUND_STRIP_GRAIN),strip);
   }
   else if(lane<0){
    // Beside a street, its walk and the walk's slabs; past the walk, an
    // area's own slabs, on its own grid (SURFACE_AREA_BEARING), with a
    // deeper joint along the walk's outer edge (the carriageway's, where it
    // has none), so the walk or the road reads as its own strip.
    var out=1e9;
    if(paved.run.w>=0.0){
     let street=params.roads[u32(paved.run.w)];
     out=paved.lane.z-paved.lane.w-street.join.z;
     if(out<0.0&&u32(street.join.w)==k){core*=1.0-walkJoint(footprint,paved);}
    }
    if(out>=0.0&&look.stones.z>0.0&&paved.bearing>=0.0&&!plain){
     // A plain border a slab wide (at most GROUND_AREA_BORDER_M) runs along the
     // edge line before the grid starts: where the grid is not square to
     // the street, it meets the border, not the kerb.
     let border=1.0-smoothstep(0.0,footprint,out-min(1.0/look.stones.z,GROUND_AREA_BORDER_M));
     let w=GROUND_ROADS_AREA_JOINT_M;
     let along=vec2f(cos(paved.bearing),sin(paved.bearing));
     // The grid fades out before a slab is a few pixels across, so far
     // paving reads as its colour rather than a shimmering mesh.
     let near=(1.0-smoothstep(GROUND_ROADS_SLAB_PIXELS_FROM,GROUND_AREA_SLAB_PIXELS_TO,footprint*look.stones.z))*(1.0-border);
     let joint=near*max(areaJoint(dot(xy,along),look.stones.z,footprint,w),areaJoint(dot(xy,vec2f(-along.y,along.x)),look.stones.z,footprint,w));
     let drawn=max(w,footprint);
     let edge=(1.0-smoothstep(drawn-footprint*0.5,drawn+footprint*0.5,out))*(w/drawn);
     core*=1.0-max(joint,edge*GROUND_ROADS_AREA_EDGE_DEPTH)*look.stones.w;
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
  let look=params.roads[u32(paved.run.w)];
  let worn=1.0-look.marks.w*smoothstep(GROUND_MARKS_WEAR_FROM,GROUND_MARKS_WEAR_TO,wanderNoise(xy*GROUND_ROADS_MARKS_WEAR_PER_M+vec2f(33.7,71.1)));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  surface=mix(surface,vec4f(look.paint.xyz*(1.0+look.shape.z*grain),look.core.w),paint*look.paint.w*worn);
 }
 let stones=groundCurb(footprint,paved).z;
 if(stones>0.0){
  let look=params.roads[u32(paved.run.w)];
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  let joint=strokeJoint(paved.run.z,look.stones.x,footprint,GROUND_ROADS_SLAB_JOINT_M)*look.stones.y;
  surface=mix(surface,vec4f(look.curb.xyz*(1.0+look.shape.z*grain)*(1.0-joint),look.core.w),stones);
 }
 return surface;
}

// @block ground_roads_strokeJoint
const GROUND_SLAB_JOINT_PIXELS_FROM = 0.7;
const GROUND_SLAB_JOINT_PIXELS_TO = 2.5;
fn strokeJoint(along: f32, perMetre: f32, footprint: f32, width: f32) -> f32 {
 let shown=1.0-smoothstep(GROUND_SLAB_JOINT_PIXELS_FROM,GROUND_SLAB_JOINT_PIXELS_TO,footprint/width);
 if(perMetre<=0.0||shown<=0.0){return 0.0;}
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(width*0.5,width*0.5+footprint,to))*shown;
}

// @block ground_roads_walkJoint_alternate
const GROUND_WALK_SLAB_JOINT_M = 0.03;
fn walkJoint(footprint: f32, paved: GroundPaved) -> f32 {
 if(paved.run.w<0.0||params.view.y==1u){return 0.0;}
 let road=params.roads[u32(paved.run.w)];
 let beyond=paved.lane.z-paved.lane.w;
 if(beyond<=0.0||beyond>=road.join.z){return 0.0;}
 return strokeJoint(paved.run.z,road.slabs.x,footprint,GROUND_WALK_SLAB_JOINT_M)*road.slabs.y;
}
// @block ground_roads_groundCurb_alternate
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
 for(var k=0u;k<ROAD_LOOKS_MAX;k++){
  if(k!=own&&params.roads[k].track.w>0.0){open*=1.0-smoothstep(-soft,soft,paved.drawn[k]);}
 }
 let edge=smoothstep(-soft,soft,inside);
 let stones=smoothstep(-width-soft,-width+soft,inside)*(1.0-edge);
 let face=edge*(1.0-smoothstep(look.slabs.z-soft,look.slabs.z+soft,inside));
 return vec3f(paved.lane.xy*(look.slabs.w*face),stones)*(open*shown);
}

// @block ground_roads_groundMarks_alternate
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
  if((entry>>GROUND_KIND_SHIFT)!=1u){continue;}
  let seg=surfaces[entry&GROUND_RECORD_MASK];
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

// @block ground_roads_roadGrain
const GROUND_ROAD_GRAIN_FADE_FROM = 0.6;
const GROUND_ROAD_GRAIN_FADE_TO = 1.2;
const GROUND_ROAD_GRAIN_OCTAVE_1 = 2.03;
const GROUND_ROAD_GRAIN_OCTAVE_2 = 4.01;
const GROUND_ROAD_GRAIN_OCTAVE_3 = 8.3;
const GROUND_ROAD_GRAIN_WEIGHT_0 = 0.25;
const GROUND_ROAD_GRAIN_WEIGHT_1 = 0.35;
const GROUND_ROAD_GRAIN_WEIGHT_2 = 0.4;
const GROUND_ROAD_GRAIN_STONE_FROM = 0.7;
const GROUND_ROAD_GRAIN_STONE_TO = 0.76;
const GROUND_ROAD_GRAIN_WEIGHT_3 = 0.5;
fn roadGrain(p: vec2f, pixel: f32) -> f32 {
 let shown=vec4f(1.0)-smoothstep(vec4f(GROUND_ROAD_GRAIN_FADE_FROM),vec4f(GROUND_ROAD_GRAIN_FADE_TO),pixel*vec4f(1.0,GROUND_ROAD_GRAIN_OCTAVE_1,GROUND_ROAD_GRAIN_OCTAVE_2,GROUND_ROAD_GRAIN_OCTAVE_3));
 var grain=0.0;
 if(shown.x>0.0){grain+=(valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y))-0.5)*GROUND_ROAD_GRAIN_WEIGHT_0*shown.x;}
 if(shown.y>0.0){grain+=(valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*GROUND_ROAD_GRAIN_OCTAVE_1+vec2f(19.1,7.7))-0.5)*GROUND_ROAD_GRAIN_WEIGHT_1*shown.y;}
 if(shown.z>0.0){grain+=(valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*GROUND_ROAD_GRAIN_OCTAVE_2+vec2f(3.3,41.9))-0.5)*GROUND_ROAD_GRAIN_WEIGHT_2*shown.z;}
 if(shown.w>0.0){
  let stone=valueNoise(p*GROUND_ROAD_GRAIN_OCTAVE_3+vec2f(71.9,13.7));
  grain+=(smoothstep(GROUND_ROAD_GRAIN_STONE_FROM,GROUND_ROAD_GRAIN_STONE_TO,stone)-smoothstep(GROUND_ROAD_GRAIN_STONE_FROM,GROUND_ROAD_GRAIN_STONE_TO,1.0-stone))*GROUND_ROAD_GRAIN_WEIGHT_3*shown.w;
 }
 return grain;
}

// @block ground_roads_wanderNoise
const GROUND_WANDER_PUSH_SCALE = 0.61;
const GROUND_WANDER_PUSH = 1.4;
const GROUND_WANDER_OCTAVE_1 = 2.03;
const GROUND_WANDER_OCTAVE_2 = 4.01;
const GROUND_WANDER_WEIGHT_0 = 0.5;
const GROUND_WANDER_WEIGHT_1 = 0.3;
const GROUND_WANDER_WEIGHT_2 = 0.2;
fn wanderNoise(at: vec2f) -> f32 {
 let push=vec2f(valueNoise(at*GROUND_WANDER_PUSH_SCALE+vec2f(11.3,47.1)),valueNoise(at*GROUND_WANDER_PUSH_SCALE+vec2f(83.9,5.7)))-0.5;
 let p=at+push*GROUND_WANDER_PUSH;
 let a=valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y));
 let b=valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*GROUND_WANDER_OCTAVE_1+vec2f(19.1,7.7));
 let c=valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*GROUND_WANDER_OCTAVE_2+vec2f(3.3,41.9));
 return a*GROUND_WANDER_WEIGHT_0+b*GROUND_WANDER_WEIGHT_1+c*GROUND_WANDER_WEIGHT_2;
}

// @block ground_roads_groundRoadRelief
fn groundRoadRelief(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec2f {
  return (groundRuts(xy, footprint, paved).xy + groundCurb(footprint, paved).xy);
}

// @block ground_roads_groundRuts_alternate
const GROUND_RUTS_FADE_FROM = 0.2;
const GROUND_RUTS_FADE_TO = 0.4;
const GROUND_RUTS_FAINT = 0.4;
const GROUND_RUTS_BREAK_FROM = 0.4;
const GROUND_RUTS_BREAK_TO = 0.6;
const GROUND_RUTS_BREAK_PER_M = 0.16666666666666666;
fn groundRuts(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec3f {
 let kind=roadLane(paved);
 if(kind<0||params.view.y==1u){return vec3f(0.0);}
 let look=params.roads[kind];
 let width=look.ruts.z;
 if(width<=0.0){return vec3f(0.0);}
 let shown=1.0-smoothstep(GROUND_RUTS_FADE_FROM,GROUND_RUTS_FADE_TO,footprint/width);
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
 let faint=mix(GROUND_RUTS_FAINT,1.0,smoothstep(GROUND_RUTS_BREAK_FROM,GROUND_RUTS_BREAK_TO,wanderNoise(xy*GROUND_RUTS_BREAK_PER_M+vec2f(13.9,57.3))))*shown;
 let mean=ruts*width*0.5333333333333333/(2.0*half);
 return vec3f(paved.lane.xy*(look.ruts.w*rise*0.6495193556767991*faint),look.track.x*(mean-depth)*faint);
}

// @block ground_roads_groundShoulder_alternate
const GROUND_SHOULDER_WANDER = 5;
const GROUND_SHOULDER_TUFT_FROM = 0.44;
const GROUND_SHOULDER_TUFT_TO = 0.52;
const GROUND_SHOULDER_TUFT_PER_M = 0.9090909090909091;
const GROUND_SHOULDER_WHOLE = 0.35;
const GROUND_SHOULDER_PIXELS_FROM = 0.03;
const GROUND_SHOULDER_PIXELS_TO = 0.08;
const GROUND_STRIP_GRASS = 0.75;
fn groundShoulder(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec2f {
 var worn=vec2f(0.0);
 let plain=params.view.y==1u;
 for(var k=0u;k<ROAD_LOOKS_MAX;k++){
  let look=params.roads[k];
  let width=look.shoulder.w;
  let out=-paved.drawn[k];
  if(out<=0.0){worn=vec2f(1.0,f32(k));continue;}
  if(plain||out>=width){continue;}
  let wander=saturate((wanderNoise(xy*look.edge.y+vec2f(27.3,88.1))-0.5)*GROUND_SHOULDER_WANDER+0.5);
  let reach=width*(1.0-look.edge.x*wander);
  let across=saturate(out/reach);
  let tufts=1.0-across*(1.0-smoothstep(GROUND_SHOULDER_TUFT_FROM,GROUND_SHOULDER_TUFT_TO,wanderNoise(xy*GROUND_SHOULDER_TUFT_PER_M+vec2f(5.9,63.1))));
  let wear=(1.0-smoothstep(GROUND_SHOULDER_WHOLE,1.0,out/reach))*tufts*(1.0-smoothstep(GROUND_SHOULDER_PIXELS_FROM,GROUND_SHOULDER_PIXELS_TO,footprint/width));
  if(wear>worn.x){worn=vec2f(wear,f32(k));}
 }
 let strip=groundStrip(xy,footprint,paved);
 if(strip>0.0){worn=vec2f(1.0-strip*GROUND_STRIP_GRASS,f32(roadLane(paved)));}
 return worn;
}

// @block ground_roads_groundStrip_alternate
const GROUND_STRIP_WANDER_PER_M = 0.4;
const GROUND_STRIP_WANDER = 3;
const GROUND_STRIP_EDGE_M = 0.3;
const GROUND_STRIP_PIXELS_FROM = 0.2;
const GROUND_STRIP_PIXELS_TO = 0.4;
fn groundStrip(xy: vec2f, footprint: f32, paved: GroundPaved) -> f32 {
 let kind=roadLane(paved);
 if(kind<0||params.view.y==1u){return 0.0;}
 let look=params.roads[kind];
 let half=look.track.y;
 if(half<=0.0||paved.lane.w>look.track.z){return 0.0;}
 let reach=half*(1.0+(wanderNoise(xy*GROUND_STRIP_WANDER_PER_M+vec2f(41.3,9.7))-0.5)*GROUND_STRIP_WANDER);
 let soft=max(footprint,GROUND_STRIP_EDGE_M);
 let shown=1.0-smoothstep(GROUND_STRIP_PIXELS_FROM,GROUND_STRIP_PIXELS_TO,footprint/(2.0*half));
 return (1.0-smoothstep(reach-soft,reach+soft,paved.lane.z))*shown;
}

// @block ground_roads_roadLane_alternate
fn roadLane(paved: GroundPaved) -> i32 {
 if(paved.run.w<0.0){return -1;}
 let own=u32(paved.run.w);
 if(paved.drawn[own]<=0.0){return -1;}
 for(var n=ROAD_LOOKS_MAX;n>0u;n--){
  let k=params.roadOrder[n-1u];
  if(k==own){break;}
  if(paved.drawn[k]>0.0){return -1;}
 }
 return i32(own);
}
