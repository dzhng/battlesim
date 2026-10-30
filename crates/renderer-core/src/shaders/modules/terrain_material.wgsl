// WGSL owner: terrain_material.

// @block terrain_material_GroundCubic
struct GroundCubic {
  taps: vec4f,
  weights: vec4f,
}

// @block terrain_material_GroundPaved
struct GroundPaved {
  drawn: vec4f,
  lane: vec4f,
  run: vec4f,
  rule: f32,
  list: vec2u,
  bearing: f32,
}

// @block terrain_material_PlotNode
struct PlotNode {
  line: vec4f,
  children: vec4i,
}

// @block terrain_material_PlotRecord
struct PlotRecord {
  colour: vec4f,
  rows: vec4f,
  detail: vec4f,
  grain: vec4f,
  dry: vec4f,
  tram: vec4f,
}

// @block terrain_material_SkyLight
struct SkyLight {
  sunDirection: vec3f,
  overcast: f32,
  sunTransmittance: vec3f,
  radiance: f32,
  groundTint: vec3f,
  mieScatter: f32,
  mieExtinction: f32,
  multipleScatter: f32,
  aureole: f32,
  overcastScale: f32,
  discRadiance: f32,
  cloudReach: f32,
  cloudEdge: vec2f,
  cloudToSun: vec2f,
  cloudLow: f32,
  cloudShadowTop: f32,
  cloudCoreTop: f32,
  cloudOpacity: f32,
  cloudLit: vec3f,
  cloudSilver: f32,
  cloudBase: vec3f,
  skySaturation: f32,
  skyTint: vec3f,
}

// @block terrain_material_backgroundColor
fn backgroundColor(uv: vec2f) -> vec4f {
    // Alpha 0 marks the sky for the post pass (its grade share).
    return max(vec4f(sample(rays.origin + rays.dx * uv.x + rays.dy * uv.y), 0.0), vec4f(0.0));
  }

// @block terrain_material_distanceRows
fn distanceRows(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let width = i32(textureDimensions(mask).x);
  let reach = i32(look.reachPx);
  var best = vec2f(16.0);
  for (var k = -reach; k <= reach; k++) {
    let x = p.x + k;
    if (x < 0 || x >= width) { continue; }
    let side = groundSide(textureLoad(mask, vec2i(x, p.y), 0));
    best = min(best, select(vec2f(16.0), vec2f(f32(abs(k))), side > vec2f(0.5)));
  }
  return vec4f(best / 16.0, 0.0, 1.0);
}

// @block terrain_material_fieldTexture
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

// @block terrain_material_fragment_Input
struct fragment_Input {
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}

// @block terrain_material_groundClassView_alternate
fn groundClassView() -> bool {
  return (params_1.view.x == 1u);
}

// @block terrain_material_groundCubicBlend
fn groundCubicBlend(cubic: GroundCubic, a: vec4f, b: vec4f, c: vec4f, e: vec4f) -> vec4f {
  let g0 = cubic.weights.xy;
  let g1 = cubic.weights.zw;
  // Keep the original hardware cubic outer contraction explicit after inlining.
  return fma(vec4f(g0.y), g0.x * a + g1.x * b, g1.y * (g0.x * c + g1.x * e));
}

// @block terrain_material_groundCubicTap
fn groundCubicTap(cubic: GroundCubic, k: u32) -> vec2f {
  return vec2f(select(cubic.taps.x, cubic.taps.z, (k & 1u) == 1u), select(cubic.taps.y, cubic.taps.w, k >= 2u));
}

// @block terrain_material_groundPaint
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

// @block terrain_material_groundPaintAtPixel
fn groundPaintAtPixel(pixel: vec2f) -> vec4f {
  if (layer.painted == 0u) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(paint));
  let p = textureLoad(paint, clamp(vec2i(pixel), vec2i(0), size - 1), 0);
  return vec4f(p.xyz * 2, p.w);
}

// @block terrain_material_groundRoadRelief
fn groundRoadRelief(xy: vec2f, footprint: f32, paved: GroundPaved) -> vec2f {
  return (groundRuts(xy, footprint, paved).xy + groundCurb(footprint, paved).xy);
}

// @block terrain_material_groundRuts_alternate
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

// @block terrain_material_groundShore_alternate
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

// @block terrain_material_groundShoulder_alternate
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

// @block terrain_material_groundSide
fn groundSide(m: vec4f) -> vec2f {
  if (m.z <= 0.5) { return vec2f(0.0); }
  return vec2f(f32(m.x > 0.5 && m.y <= 0.0), f32(m.y > 0.5 && m.x <= 0.0));
}

// @block terrain_material_groundStrip_alternate
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

// @block terrain_material_groundTint
const GROUND_DRY_RED = 0.699999988079071;
const GROUND_DRY_GREEN = 0;
const GROUND_DRY_BLUE_ABS = 0.8999999761581421;
fn groundTint(albedo: vec3f, dry: f32) -> vec3f {
  let tinted = (albedo * (vec3f(1) + (vec3f(GROUND_DRY_RED, GROUND_DRY_GREEN, -GROUND_DRY_BLUE_ABS) * dry)));
  return (tinted * (dot(albedo, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B)) / max(dot(tinted, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B)), 1e-5f)));
}

// @block terrain_material_groundVerge_alternate
fn groundVerge(site: vec4f, footprint: f32) -> f32 {
  let params_1 = (&params);
  let vergeHalf = (*params_1).verge.w;
  return (1f - smoothstep(vergeHalf, (vergeHalf + max((*params_1).feathers.x, footprint)), site.y));
}

// @block terrain_material_groundWheeling
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

// @block terrain_material_mottle_alternate
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

// @block terrain_material_nodes
@group(1) @binding(1) var<storage, read> nodes: array<PlotNode>;

// @block terrain_material_paintStyle_primary
@group(2) @binding(6) var<uniform> paintStyle: GroundPaintStyle;

// @block terrain_material_params
@group(1) @binding(0) var<uniform> params: TerrainParams;

// @block terrain_material_plots
@group(1) @binding(2) var<storage, read> plots: array<PlotRecord>;

// @block terrain_material_radiance
fn radiance(dirIn: vec3f) -> vec3f {
  let betaR = vec3f(5.8019999999999999e-3f, 1.3558000000000001e-2f, 3.3099999999999997e-2f);
  let betaMScatter = light.mieScatter;
  let betaMExtinction = light.mieExtinction;
  let sunDir = light.sunDirection;
  let mu = max(dirIn.z, 0.004);
  let dir = normalize(vec3f(dirIn.xy, mu));
  let tTop = sqrt(4.0452144039999999e+7f * (mu * mu - 1.0) + 4.1731600000000000e+7f) - mu * 6.3601999999999998e+3f;
  let dt = tTop / 32.0;
  let cosTheta = dot(dir, sunDir);
  let phaseR = (cosTheta * cosTheta + 1.0) * 5.9683103659460751e-2f;
  let phaseM = 2.8647889756541152e-2f / pow(1.6400000000000001e+0f - cosTheta * 1.6000000000000001e+0f, 1.5);
  var radiance = vec3f(0.0);
  var odView = vec3f(0.0);
  for (var i = 0; i < 32; i++) {
    let t = (f32(i) + 0.5) * dt;
    let px = dir.x * t;
    let py = dir.y * t;
    let pz = dir.z * t + 6.3601999999999998e+3f;
    let rp = sqrt(px * px + py * py + pz * pz);
    let h = rp - 6.3600000000000000e+3f;
    let rhoR = exp(h / -8.0000000000000000e+0f);
    let rhoM = exp(h / -1.2000000000000000e+0f);
    odView += (betaR * rhoR + rhoM * betaMExtinction) * dt;
    let tView = exp(-odView);
    let muS = (px * sunDir.x + py * sunDir.y + pz * sunDir.z) / rp;
    let tSunTop = sqrt(rp * rp * (muS * muS - 1.0) + 4.1731600000000000e+7f) - muS * rp;
    let dts = tSunTop / 6.0;
    var odSun = vec3f(0.0);
    for (var j = 0; j < 6; j++) {
      let ts = (f32(j) + 0.5) * dts;
      let sx = px + sunDir.x * ts;
      let sy = py + sunDir.y * ts;
      let sz = pz + sunDir.z * ts;
      let hs = sqrt(sx * sx + sy * sy + sz * sz) - 6.3600000000000000e+3f;
      odSun += (betaR * exp(hs / -8.0000000000000000e+0f) + exp(hs / -1.2000000000000000e+0f) * betaMExtinction) * dts;
    }
    let horizonMu = -sqrt(max(1.0 - 4.0449600000000000e+7f / (rp * rp), 0.0));
    let shadow = smoothstep(horizonMu - 0.008, horizonMu + 0.004, muS);
    let tSun = exp(-odSun) * shadow;
    let scatterR = betaR * rhoR;
    let scatterM = rhoM * betaMScatter;
    let single = tSun * (scatterR * phaseR + scatterM * phaseM);
    let multiple = (scatterR + scatterM) * vec3f(5.5000000000000004e-1f, 7.8000000000000003e-1f, 1.1499999999999999e+0f) * light.multipleScatter;
    radiance += tView * (single + multiple) * dt;
  }
  let aureole = smoothstep(-1.2000000000000000e-1f, 7.6000000000000001e-1f, cosTheta) * light.aureole;
  let clearSkyRaw = radiance * light.radiance + light.sunTransmittance * aureole;
  let clearSky = mix(vec3f(dot(clearSkyRaw, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B))), clearSkyRaw, light.skySaturation) * light.skyTint;
  let overcastGradient = 1.05 - max(dirIn.z, 0.0) * 0.22;
  let overcastSky = vec3f(1.0400000000000000e+0f, 1.0800000000000001e+0f, 1.1399999999999999e+0f) * overcastGradient * light.overcastScale;
  let result = mix(clearSky, overcastSky, light.overcast);
  let ground = smoothstep(0.0, 0.35, -dirIn.z);
  return mix(result, result * light.groundTint, ground);
}

// @block terrain_material_roadLane_alternate
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

// @block terrain_material_sample
fn sample(ray: vec3f) -> vec3f {
    let dir = normalize(ray);
    // Below the horizon the view shows the horizon's haze, not the LUT's
    // ground-bounce hemisphere (that stays for the environment light only):
    // the backdrop land dissolves into it rather than into a khaki band.
    let shown = normalize(vec3f(dir.xy, max(dir.z, 0.004)));
    // Clouds pass in front of the sun's disc; the LUT stays clear-sky.
    return clouds(dir, textureSampleLevel(lutView, sampler_1, equirectUv(shown), 0.0).rgb + disc(dir));
  }

// @block terrain_material_terrain
@group(0) @binding(6) var<storage, read_write> terrain: array<u32>;

// @block terrain_material_terrainFragment_Input
struct terrainFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block terrain_material_waterFragment_Input
struct waterFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block terrain_material_waterNormal
fn waterNormal(xy: vec2f, footprint: f32, look: u32) -> vec3f {
  let w = (&params.waterLooks[look]);
  let carried = waterCarried();
  let primary = waterLayer(xy, (*w).normals.x, vec2f(1, 0), (carried * (*w).drifts.x), (*w).normals.w, footprint);
  let macro_1 = waterLayer(xy, (*w).normals.y, vec2f(0.824999988079071, 0.5649999976158142), (carried * (*w).drifts.y), (*w).normals.w, footprint);
  let detail = waterLayer(xy, (*w).normals.z, vec2f(0.45399999618530273, -0.890999972820282), (carried * (*w).drifts.z), (*w).normals.w, footprint);
  let n = blendRnm(blendRnm(macro_1, primary), detail);
  let shown = (1f - smoothstep((*w).fades.y, (*w).fades.z, footprint));
  return normalize(mix(vec3f(0, 0, 1), n, shown));
}
