// WGSL owner: ground_material.

// @block ground_material_groundShoulderGrass
fn groundShoulderGrass(worn: vec2f) -> f32 {
 return mix(1.0,params_1.roads[u32(worn.y)].edge.z,worn.x);
}

// @block ground_material_groundAlbedo
fn groundAlbedo(world: vec3f, tint: vec4f, cell: vec4u, paved: GroundPaved) -> vec4f {
  let footprint = length(fwidth(world.xy));
  let xy = world.xy;
  let site = groundSite(xy, cell, paved);
  let surface = groundColour(xy, footprint, site, paved, groundWater(xy, cell));
  let albedo = mix(surface.xyz, srgbToLinear(tint.xyz), tint.w);
  return vec4f(albedo, mix(surface.w, 0.85f, tint.w));
}

// @block ground_material_groundBank
const GROUND_BANK_BLEND_REACH_M = 50;
const GROUND_BANK_BLEND_M = 0.7;
const GROUND_BANK_MAX_SLOPE = 0.84;
fn groundBank(xy: vec2f, cell: vec4u, footprint: f32) -> vec3f {
 var sum=vec4f(0.0);
 for(var i=cell.z;i<cell.w;i++){
  let at=surfaceIndex[i]&GROUND_RECORD_MASK;
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

// @block ground_material_forestFloorWeight_alternate
fn forestFloorWeight(xy: vec2f, forest: f32, footprint: f32) -> f32 {
  let feather = max(footprint, (params.forestVerge.x * GROUND_FOREST_VERGE_FEATHER));
  return smoothstep(-(feather), feather, forestVergeInside(xy, forest));
}

// @block ground_material_forestFloor_alternate
const GROUND_FOREST_FLOOR_MOSS_FROM = 0.38f;
const GROUND_FOREST_FLOOR_MOSS_TO = 0.72f;
const GROUND_FOREST_FLOOR_MOSS_BROAD_WEIGHT = 0.65f;
const GROUND_FOREST_FLOOR_MOSS_FINE_SCALE = 2.1f;
const GROUND_FOREST_FLOOR_MOSS_FINE_WEIGHT = 0.35f;
const GROUND_FOREST_FLOOR_HUMUS_FROM = 0.42f;
const GROUND_FOREST_FLOOR_HUMUS_TO = 0.78f;
const GROUND_FOREST_FLOOR_HUMUS_BROAD_SCALE = 1.3f;
const GROUND_FOREST_FLOOR_HUMUS_BROAD_WEIGHT = 0.6f;
const GROUND_FOREST_FLOOR_HUMUS_FINE_SCALE = 3.1f;
const GROUND_FOREST_FLOOR_HUMUS_FINE_WEIGHT = 0.4f;
const GROUND_FOREST_FLOOR_ROOT_RUN_FROM = 0.55f;
const GROUND_FOREST_FLOOR_ROOT_RUN_TO = 0.75f;
const GROUND_FOREST_FLOOR_ROOT_RUN_SCALE = 1.7f;
const GROUND_FOREST_FLOOR_ROOT_FADE_FROM = 0.02f;
const GROUND_FOREST_FLOOR_ROOT_FADE_TO = 0.08f;
const GROUND_FOREST_FLOOR_ROOT_LINE_FROM = 0.015f;
const GROUND_FOREST_FLOOR_ROOT_LINE_TO = 0.035f;
const GROUND_FOREST_FLOOR_ROOT_MEAN = 0.02f;
fn forestFloor(xy: vec2f, footprint: f32, noise: f32) -> vec3f {
  let params_1 = (&params);
  let detail = (&(*params_1).forestDetail);
  let at = (xy * (*detail).x);
  let turned = vec2f(((at.x * 0.8f) - (at.y * 0.6f)), ((at.x * 0.6f) + (at.y * 0.8f)));
  let across = vec2f(((at.x * 0.28f) + (at.y * 0.96f)), ((at.y * 0.28f) - (at.x * 0.96f)));
  let moss = smoothstep(GROUND_FOREST_FLOOR_MOSS_FROM, GROUND_FOREST_FLOOR_MOSS_TO, ((valueNoise((turned + vec2f(13.300000190734863, 7.699999809265137))) * GROUND_FOREST_FLOOR_MOSS_BROAD_WEIGHT) + (valueNoise(((across * GROUND_FOREST_FLOOR_MOSS_FINE_SCALE) + vec2f(4.099999904632568, 9.199999809265137))) * GROUND_FOREST_FLOOR_MOSS_FINE_WEIGHT)));
  let humus = smoothstep(GROUND_FOREST_FLOOR_HUMUS_FROM, GROUND_FOREST_FLOOR_HUMUS_TO, ((valueNoise(((across * GROUND_FOREST_FLOOR_HUMUS_BROAD_SCALE) + vec2f(2.9000000953674316, 41.099998474121094))) * GROUND_FOREST_FLOOR_HUMUS_BROAD_WEIGHT) + (valueNoise(((turned * GROUND_FOREST_FLOOR_HUMUS_FINE_SCALE) + vec2f(17.299999237060547, 5.900000095367432))) * GROUND_FOREST_FLOOR_HUMUS_FINE_WEIGHT)));
  var floor_1 = mix((*params_1).forestLitter.xyz, (*params_1).forestMoss.xyz, (moss * (*params_1).forestMoss.w));
  floor_1 = mix(floor_1, (*params_1).forestHumus.xyz, (humus * (*params_1).forestHumus.w));
  let rootsAt = (xy * (*params_1).forestVerge.w);
  let ridge = abs((valueNoise((rootsAt + vec2f(29.100000381469727, 3.299999952316284))) - 0.5f));
  let runs = smoothstep(GROUND_FOREST_FLOOR_ROOT_RUN_FROM, GROUND_FOREST_FLOOR_ROOT_RUN_TO, valueNoise(((rootsAt * GROUND_FOREST_FLOOR_ROOT_RUN_SCALE) + vec2f(8.300000190734863, 17.899999618530273))));
  let shown = (1f - smoothstep(GROUND_FOREST_FLOOR_ROOT_FADE_FROM, GROUND_FOREST_FLOOR_ROOT_FADE_TO, (footprint * (*params_1).forestVerge.w)));
  let line = ((1f - smoothstep(GROUND_FOREST_FLOOR_ROOT_LINE_FROM, GROUND_FOREST_FLOOR_ROOT_LINE_TO, ridge)) * runs);
  let root = ((line * shown) + (GROUND_FOREST_FLOOR_ROOT_MEAN * (1f - shown)));
  floor_1 = (floor_1 * (1f - ((*detail).w * root)));
  return (floor_1 * (1f + ((*detail).y * noise)));
}

// @block ground_material_forestVergeInside_alternate
const GROUND_FOREST_VERGE_PATCH_SCALE = 0.7f;
const GROUND_FOREST_VERGE_INSET = 0.5f;
fn forestVergeInside(xy: vec2f, forest: f32) -> f32 {
  let verge = (&params.forestVerge);
  let wander = ((valueNoise(((xy * (*verge).z) + vec2f(71.30000305175781, 23.899999618530273))) - 0.5f) * 2f);
  let patches = (valueNoise(((xy * GROUND_FOREST_VERGE_PATCH_SCALE) + vec2f(5.099999904632568, 91.69999694824219))) - 0.5f);
  return (((forest + ((*verge).x * GROUND_FOREST_VERGE_INSET)) + (wander * (*verge).y)) + (patches * (*verge).x));
}

// @block ground_material_groundClasses
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

// @block ground_material_groundColour_alternate
const GROUND_COLOUR_VERGE_MOTTLE = 0.18f;
const GROUND_COLOUR_VERGE_ROUGHNESS = 0.95f;
const GROUND_COLOUR_DISTANT_FADE_M = 600f;
fn groundColour(xy: vec2f, footprint: f32, site: vec4f, paved: GroundPaved, water: f32) -> vec4f {
  let params_1 = (&params);
  let aa = (footprint * 0.5f);
  let plot = (&plots[i32(site.x)]);
  let variation = mottle(xy, footprint, (*plot).rows.xy, site.x, (*plot).dry.xy);
  let noise = variation.x;
  var albedo = groundTint(((*plot).colour.xyz * fieldTexture(xy, footprint, i32(site.x))), ((*plot).detail.x * variation.y));
  var roughness = (*plot).colour.w;
  let verge = groundVerge(site, footprint);
  let vergeColour = ((*params_1).verge.xyz * (1f + (GROUND_COLOUR_VERGE_MOTTLE * noise)));
  albedo = mix(albedo, vergeColour, verge);
  roughness = mix(roughness, GROUND_COLOUR_VERGE_ROUGHNESS, verge);
  let inRegion = rectInside(xy, (*params_1).region);
  let distant = max((1f - smoothstep(0f, GROUND_COLOUR_DISTANT_FADE_M, inRegion)), smoothstep((*params_1).feathers.y, (*params_1).feathers.z, footprint));
  albedo = mix(albedo, (*params_1).distant.xyz, distant);
  let floor_1 = groundFloor(xy, footprint, site.w);
  if ((floor_1.x > 0f)) {
    albedo = mix(albedo, forestFloor(xy, footprint, noise), floor_1.x);
    roughness = mix(roughness, (*params_1).forestDetail.z, floor_1.x);
  }
  if ((floor_1.y > 0f)) {
    albedo = mix(albedo, vergeColour, floor_1.y);
    roughness = mix(roughness, GROUND_COLOUR_VERGE_ROUGHNESS, floor_1.y);
  }
  let shore = groundShore(xy, footprint, water);
  if ((shore.y > 0f)) {
    let field = mix(dot((*params_1).verge.xyz, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B)), (dot((*plot).colour.xyz, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B)) * (1f - (0.5f * (*plot).rows.w))), smoothstep(0f, (*params_1).shoreTuning.ground.y, site.y));
    let least = (field * (*params_1).shoreEdge.z);
    let earth = ((*params_1).shoreEarth.xyz * (1f + ((*params_1).shoreTuning.ground.x * noise)));
    albedo = mix(albedo, atLeast(earth, least), shore.y);
    roughness = mix(roughness, (*params_1).shoreTuning.ground.z, shore.y);
    albedo = mix(albedo, atLeast((*params_1).shore.xyz, least), shore.x);
    roughness = mix(roughness, (*params_1).shoreTuning.ground.w, shore.x);
  }
  let paving = groundRoads(xy, footprint, paved, vec4f(albedo, roughness));
  let bed = smoothstep(-(aa), max(aa, (*params_1).shoreTuning.fade.z), water);
  albedo = mix(paving.xyz, (*params_1).waterBed.xyz, bed);
  return vec4f(max(albedo, vec3f()), paving.w);
}

// @block ground_material_groundDapple
const GROUND_DAPPLE_BROAD_WEIGHT = 0.65f;
const GROUND_DAPPLE_OCTAVE = 2.7f;
const GROUND_DAPPLE_FINE_WEIGHT = 0.35f;
const GROUND_DAPPLE_EDGE = 0.06f;
const GROUND_DAPPLE_FADE_FROM = 0.1f;
const GROUND_DAPPLE_FADE_TO = 0.5f;
fn groundDapple(xy: vec2f, footprint: f32, cell: vec4u) -> f32 {
  let params_2 = (&params_1);
  let forest = groundForest(xy, cell);
  if ((forest < (-((*params_2).forestVerge.x) - (*params_2).forestVerge.y))) {
    return 0f;
  }
  let dapple = (&(*params_2).forestDapple);
  let at = (xy * (*dapple).x);
  let n = ((valueNoise(at) * GROUND_DAPPLE_BROAD_WEIGHT) + (valueNoise(((at * GROUND_DAPPLE_OCTAVE) + vec2f(3.700000047683716, 8.100000381469727))) * GROUND_DAPPLE_FINE_WEIGHT));
  let cut = (1f - (*dapple).y);
  let fleck = smoothstep((cut - GROUND_DAPPLE_EDGE), (cut + GROUND_DAPPLE_EDGE), n);
  let shown = (1f - smoothstep(GROUND_DAPPLE_FADE_FROM, GROUND_DAPPLE_FADE_TO, (footprint * (*dapple).x)));
  let flecks = mix((*dapple).y, fleck, shown);
  return ((flecks * (*dapple).z) * forestFloorWeight(xy, forest, footprint));
}

// @block ground_material_groundFloor_alternate
fn groundFloor(xy: vec2f, footprint: f32, forest: f32) -> vec2f {
  let params_1 = (&params);
  let wood = forestFloorWeight(xy, forest, footprint);
  if (((wood > 0f) || (forest > (-((*params_1).forestVerge.x) - (*params_1).forestVerge.y)))) {
    let band = groundLineBand(xy, groundCell(xy, footprint), forest);
    if ((band.x > 0.5f)) {
      let feather = max(footprint, ((*params_1).forestVerge.x * GROUND_FOREST_VERGE_FEATHER));
      return vec2f(0f, smoothstep(-(feather), feather, band.y));
    }
  }
  return vec2f(wood, 0f);
}

// @block ground_material_groundLineBand_alternate
fn groundLineBand(xy: vec2f, cell: vec4u, forest: f32) -> vec2f {
  let params_1 = (&params);
  let line = groundTreeLine(xy, cell);
  if ((line.x < forest)) {
    return vec2f(0, -1000000000);
  }
  let at = ((xy * (*params_1).forestVerge.z) + vec2f(71.30000305175781, 23.899999618530273));
  return vec2f(1f, (line.y + (((valueNoise(at) - 0.5f) * 2f) * (*params_1).forestLine.y)));
}

// @block ground_material_fieldTexture
const GROUND_ROWS_FADE_FROM = 0.15;
const GROUND_ROWS_FADE_TO = 0.45;
const GROUND_ROWS_BREAK_GRAINS = 2.0;
const GROUND_GRAIN_FADE_FROM = 0.3;
const GROUND_GRAIN_FADE_TO = 0.9;
const GROUND_ROWS_MEAN = 0.3125;
const GROUND_GRAIN_BASE_DIVISOR = 7.290000000000001;
const GROUND_GRAIN_WEIGHT_0 = 0.15;
const GROUND_GRAIN_WEIGHT_1 = 0.25;
const GROUND_GRAIN_WEIGHT_2 = 0.35;
const GROUND_GRAIN_WEIGHT_3 = 0.25;
const GROUND_GRAIN_TURN_0_COS = 0.8525245220595057;
const GROUND_GRAIN_TURN_0_SIN = 0.5226872289306592;
const GROUND_GRAIN_TURN_1_COS_ABS = 0.32328956686350335;
const GROUND_GRAIN_TURN_1_SIN = 0.9463000876874145;
const GROUND_GRAIN_TURN_2_COS_ABS = 0.9709581651495905;
const GROUND_GRAIN_TURN_2_SIN = 0.23924932921398243;
const GROUND_GRAIN_TURN_3_COS_ABS = 0.40079917207997545;
const GROUND_GRAIN_TURN_3_SIN_ABS = 0.9161659367494549;
const GROUND_GRAIN_CRISP_SPAN = 4.0;
const GROUND_GRAIN_WARP = 0.45;
const GROUND_GRAIN_OCTAVE = 2.7;
fn fieldTexture(xy: vec2f, footprint: f32, plot: i32) -> f32 {
  let rows = plots[plot].rows;
  let grain = plots[plot].grain;
  let breaks = plots[plot].detail.z;
  let u = dot(xy, vec2f(-rows.y, rows.x));
  let v = dot(xy, rows.xy);
  var value = 1.0;
  if (rows.z > 0.0) {
    let phase = v / rows.z;
    let shown = 1.0 - smoothstep(GROUND_ROWS_FADE_FROM, GROUND_ROWS_FADE_TO, footprint / rows.z);
    let stripe = 0.5 + 0.5 * cos(phase * 6.2831853);
    var furrow = stripe * stripe * stripe;
    let along = grain.x * grain.z / GROUND_ROWS_BREAK_GRAINS;
    let seen = shown * (1.0 - smoothstep(GROUND_GRAIN_FADE_FROM, GROUND_GRAIN_FADE_TO, footprint * along));
    if (breaks > 0.0 && seen > 0.0) {
      // Each furrow has its own noise along its length; two meet on the bed
      // between them, where neither darkens anything, so nothing steps.
      let gap = valueNoise(vec2f(u * along, floor(phase + 0.5) * 7.31 + 0.5));
      furrow *= 1.0 + breaks * (gap * 2.0 - 1.0) * seen;
    }
    let bare = plots[plot].tram.z * groundWheeling(xy, footprint, plot);
    value = (1.0 - rows.w * (0.5 + (furrow - GROUND_ROWS_MEAN) * shown)) * (1.0 - bare);
  }
  if (grain.y > 0.0) {
    var size = grain.x / GROUND_GRAIN_BASE_DIVISOR;
    var sum = 0.0;
    var push = vec2f(0.0);
    var weights = array<f32, 4>(GROUND_GRAIN_WEIGHT_0, GROUND_GRAIN_WEIGHT_1, GROUND_GRAIN_WEIGHT_2, GROUND_GRAIN_WEIGHT_3);
    var turns = array<vec2f, 4>(vec2f(GROUND_GRAIN_TURN_0_COS, GROUND_GRAIN_TURN_0_SIN), vec2f(-GROUND_GRAIN_TURN_1_COS_ABS, GROUND_GRAIN_TURN_1_SIN), vec2f(-GROUND_GRAIN_TURN_2_COS_ABS, GROUND_GRAIN_TURN_2_SIN), vec2f(-GROUND_GRAIN_TURN_3_COS_ABS, -GROUND_GRAIN_TURN_3_SIN_ABS));
    for (var i = 0u; i < 4u; i++) {
      let shown = 1.0 - smoothstep(GROUND_GRAIN_FADE_FROM, GROUND_GRAIN_FADE_TO, footprint * size);
      if (shown > 0.0) {
        let turn = turns[i];
        let p = vec2f(u * grain.z, v) * size;
        let seed = vec2f(f32(i) * 17.31 + 3.7, f32(plot % 64) * 1.37);
        let n = valueNoise(vec2f(dot(p, turn), dot(p, vec2f(-turn.y, turn.x))) + seed + push);
        sum += weights[i] * shown * clamp((n - 0.5) * GROUND_GRAIN_CRISP_SPAN, -1.0, 1.0);
        push = vec2f(cos(n * 6.2831853), sin(n * 6.2831853)) * GROUND_GRAIN_WARP;
      }
      size *= GROUND_GRAIN_OCTAVE;
    }
    value *= 1.0 + grain.y * sum;
  }
  return value;
}

// @block ground_material_groundClassView_alternate
fn groundClassView() -> bool {
  return (params_1.view.x == 1u);
}

// @block ground_material_groundShore_alternate
fn groundShore(xy: vec2f, footprint: f32, water: f32) -> vec3f {
  let params_1 = (&params);
  let out = -(water);
  let wet = (*params_1).shore.w;
  let reach = (*params_1).shoreEarth.w;
  if ((out >= reach)) {
    return vec3f();
  }
  let t = (&(*params_1).shoreTuning);
  let broad = valueNoise((turned(xy, (*t).turns.xy) * (*params_1).shoreEdge.y));
  let scale = ((*params_1).shoreEdge.y * (*t).bands.z);
  let fine = mix(valueNoise(((turned(xy, (*t).turns.zw) * scale) + vec2f(41.70000076293945, 13.100000381469727))), 0.5f, smoothstep((*t).fade.x, (*t).fade.y, (footprint * scale)));
  let line = (reach * (1f - ((*params_1).shoreEdge.x * mix(broad, fine, (*t).bands.w))));
  return vec3f((1f - smoothstep((wet * (*t).bands.x), wet, out)), (1f - smoothstep(mix(wet, line, (*t).bands.y), line, out)), (1f - smoothstep(wet, line, out)));
}

// @block ground_material_groundTint
const GROUND_DRY_RED = 0.699999988079071;
const GROUND_DRY_GREEN = 0;
const GROUND_DRY_BLUE_ABS = 0.8999999761581421;
fn groundTint(albedo: vec3f, dry: f32) -> vec3f {
  let tinted = (albedo * (vec3f(1) + (vec3f(GROUND_DRY_RED, GROUND_DRY_GREEN, -GROUND_DRY_BLUE_ABS) * dry)));
  return (tinted * (dot(albedo, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B)) / max(dot(tinted, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B)), 1e-5f)));
}

// @block ground_material_groundVerge_alternate
fn groundVerge(site: vec4f, footprint: f32) -> f32 {
  let params_1 = (&params);
  let vergeHalf = (*params_1).verge.w;
  return (1f - smoothstep(vergeHalf, (vergeHalf + max((*params_1).feathers.x, footprint)), site.y));
}

// @block ground_material_groundWheeling
const GROUND_WHEELING_GAUGE_ROWS = 2.0;
const GROUND_WHEELING_LEAST_PIXEL_M = 0.02;
fn groundWheeling(xy: vec2f, footprint: f32, plot: i32) -> f32 {
  let rows = plots[plot].rows;
  let tram = plots[plot].tram;
  if (tram.x <= 0.0 || rows.z <= 0.0) { return 0.0; }
  let phase = dot(xy, rows.xy) / rows.z;
  let furrow = round(phase);
  let nth = furrow - tram.x * floor(furrow / tram.x);
  if (nth != 0.0 && nth != GROUND_WHEELING_GAUGE_ROWS) { return 0.0; }
  let off = abs(phase - furrow) * rows.z;
  let pixel = max(footprint, GROUND_WHEELING_LEAST_PIXEL_M);
  return clamp((tram.y - off) / pixel + 0.5, 0.0, 1.0) * min(1.0, 2.0 * tram.y / pixel);
}

// @block ground_material_mottle_alternate
const GROUND_MOTTLE_SLOPE_TAP = 0.05000000074505806;
const GROUND_MOTTLE_SLOPE_STEP = 0.05f;
const GROUND_MOTTLE_PATCH_CUT = 0.58f;
const GROUND_MOTTLE_PATCH_EDGE_M = 0.3f;
const GROUND_MOTTLE_FINE_FADE_FROM = 0.25f;
const GROUND_MOTTLE_FINE_FADE_TO = 1f;
const GROUND_MOTTLE_FINE_VALUE = 0.45f;
const GROUND_MOTTLE_PATCH_HUE = 0.6f;
const GROUND_MOTTLE_FINE_HUE = 0.4f;
fn mottle(xy: vec2f, footprint: f32, across: vec2f, plot: f32, reach: vec2f) -> vec2f {
  let shape = (&params.shape);
  let along = vec2f(-(across.y), across.x);
  let lane = vec2f((dot(xy, along) * reach.x), (dot(xy, across) * reach.y));
  let seed = vec2f((fract((plot * 0.6180339f)) * 997f), (fract((plot * 0.7548776f)) * 991f));
  let at = (lane + seed);
  let streak = valueNoise(at);
  let slope = vec2f((((valueNoise((at + vec2f(GROUND_MOTTLE_SLOPE_TAP, 0))) - streak) / GROUND_MOTTLE_SLOPE_STEP) * reach.x), (((valueNoise((at + vec2f(0, GROUND_MOTTLE_SLOPE_TAP))) - streak) / GROUND_MOTTLE_SLOPE_STEP) * reach.y));
  let inside = ((streak - GROUND_MOTTLE_PATCH_CUT) / max(length(slope), 1e-4f));
  let edge = (max(footprint, GROUND_MOTTLE_PATCH_EDGE_M) * 0.5f);
  let parched = smoothstep(-(edge), edge, inside);
  let fine = (valueNoise(((xy * (*shape).z) + vec2f(37.099998474121094, 11.300000190734863))) - 0.5f);
  let fineShown = (1f - smoothstep(GROUND_MOTTLE_FINE_FADE_FROM, GROUND_MOTTLE_FINE_FADE_TO, (footprint * (*shape).z)));
  return vec2f(((fine * GROUND_MOTTLE_FINE_VALUE) * fineShown), ((parched * GROUND_MOTTLE_PATCH_HUE) + ((max(fine, 0f) * GROUND_MOTTLE_FINE_HUE) * fineShown)));
}
