// WGSL owner: models_geometry.

// @block models_geometry_Joint
struct Joint {
  x: vec4f,
  y: vec4f,
  z: vec4f,
}

// @block models_geometry_areaJoint
fn areaJoint(along: f32, perMetre: f32, footprint: f32, width: f32) -> f32 {
 let drawn=max(width,footprint);
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(drawn*0.5-footprint*0.5,drawn*0.5+footprint*0.5,to))*(width/drawn);
}

// @block models_geometry_cardDirection
fn cardDirection(direction: vec3f, scale: vec3f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  return (vec3f(((direction.x * c) + (direction.y * s)), ((-(direction.x) * s) + (direction.y * c)), direction.z) / scale);
}

// @block models_geometry_cardNormal
fn cardNormal(normal_1: vec3f, scale: vec3f, yaw: f32) -> vec3f {
  return normalize(cardPoint((normal_1 / scale), vec3f(1), yaw, vec3f()));
}

// @block models_geometry_cardPoint
fn cardPoint(point: vec3f, scale: vec3f, yaw: f32, at: vec3f) -> vec3f {
  let p = (point * scale);
  let c = cos(yaw);
  let s = sin(yaw);
  return (at + vec3f(((p.x * c) - (p.y * s)), ((p.x * s) + (p.y * c)), p.z));
}

// @block models_geometry_cardVertex_Output
struct cardVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) uv: vec2f,
  @location(2) @interpolate(flat) layer: u32,
  @location(3) @interpolate(flat) yaw: f32,
  @location(4) @interpolate(flat) scale: vec3f,
  @location(5) tint: vec3f,
  @location(6) anchor: vec3f,
}

// @block models_geometry_countCoverage
fn countCoverage(clip: vec4f, instance: u32, xray: vec4f) -> vec4f {
    if (xray.w <= 0.0) { discard; }
    atomicAdd(&counts[instance].total, 1u);
    // Same float reverse-Z offset as the hardware depth bias in the draw.
    let biased = bitcast<f32>(bitcast<u32>(clip.z) + 65536u);
    let world = textureLoad(depth, vec2i(clip.xy), 0);
    if (biased < world) { atomicAdd(&counts[instance].hidden, 1u); }
    return vec4f(0.0);
  }

// @block models_geometry_drawCoverage
fn drawCoverage(instance: u32, xray: vec4f) -> vec4f {
    let coverage = counts[instance];
    if (xray.w <= 0.0 || f32(coverage.y) < f32(coverage.x) * minimum) { discard; }
    return vec4f(xray.rgb * xray.a, xray.a);
  }

// @block models_geometry_groundRoads_alternate
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
  let luma=vec3f(0.2126,0.7152,0.0722);
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

// @block models_geometry_impostor_Input
struct impostor_Input {
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

// @block models_geometry_impostor_Output
struct impostor_Output {
  @location(0) albedo: vec4f,
  @location(1) normal: vec4f,
  @location(2) mask: vec4f,
}

// @block models_geometry_modelVertex_Output
struct modelVertex_Output {
  @invariant @builtin(position) clip: vec4f,
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

// @block models_geometry_moved
fn moved(j: Joint, p: vec4f) -> vec4f {
  return vec4f(dot(j.x, p), dot(j.y, p), dot(j.z, p), p.w);
}

// @block models_geometry_palette
@group(1) @binding(0) var<storage, read> palette: array<Joint>;

// @block models_geometry_strokeJoint
const GROUND_SLAB_JOINT_PIXELS_FROM = 0.7;
const GROUND_SLAB_JOINT_PIXELS_TO = 2.5;
fn strokeJoint(along: f32, perMetre: f32, footprint: f32, width: f32) -> f32 {
 let shown=1.0-smoothstep(GROUND_SLAB_JOINT_PIXELS_FROM,GROUND_SLAB_JOINT_PIXELS_TO,footprint/width);
 if(perMetre<=0.0||shown<=0.0){return 0.0;}
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(width*0.5,width*0.5+footprint,to))*shown;
}

// @block models_geometry_walkJoint_alternate
const GROUND_WALK_SLAB_JOINT_M = 0.03;
fn walkJoint(footprint: f32, paved: GroundPaved) -> f32 {
 if(paved.run.w<0.0||params.view.y==1u){return 0.0;}
 let road=params.roads[u32(paved.run.w)];
 let beyond=paved.lane.z-paved.lane.w;
 if(beyond<=0.0||beyond>=road.join.z){return 0.0;}
 return strokeJoint(paved.run.z,road.slabs.x,footprint,GROUND_WALK_SLAB_JOINT_M)*road.slabs.y;
}
