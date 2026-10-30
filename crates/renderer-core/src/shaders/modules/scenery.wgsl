// WGSL owner: scenery.

// @block scenery_ShoreTuning
struct ShoreTuning {
  bands: vec4f,
  turns: vec4f,
  ground: vec4f,
  fade: vec4f,
}

// @block scenery_WaterLook
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

// @block scenery_TerrainParams
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

// @block scenery_dressingVertex_Output
struct dressingVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block scenery_felledVertex_Output
struct felledVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block scenery_forestFloorWeight_alternate
fn forestFloorWeight(xy: vec2f, forest: f32, footprint: f32) -> f32 {
  let feather = max(footprint, (params.forestVerge.x * GROUND_FOREST_VERGE_FEATHER));
  return smoothstep(-(feather), feather, forestVergeInside(xy, forest));
}

// @block scenery_forestFloor_alternate
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

// @block scenery_forestFragment_Input
struct forestFragment_Input {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block scenery_forestVergeInside_alternate
const GROUND_FOREST_VERGE_PATCH_SCALE = 0.7f;
const GROUND_FOREST_VERGE_INSET = 0.5f;
fn forestVergeInside(xy: vec2f, forest: f32) -> f32 {
  let verge = (&params.forestVerge);
  let wander = ((valueNoise(((xy * (*verge).z) + vec2f(71.30000305175781, 23.899999618530273))) - 0.5f) * 2f);
  let patches = (valueNoise(((xy * GROUND_FOREST_VERGE_PATCH_SCALE) + vec2f(5.099999904632568, 91.69999694824219))) - 0.5f);
  return (((forest + ((*verge).x * GROUND_FOREST_VERGE_INSET)) + (wander * (*verge).y)) + (patches * (*verge).x));
}

// @block scenery_groundClasses
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

// @block scenery_groundColour_alternate
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

// @block scenery_groundDapple
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

// @block scenery_groundFloor_alternate
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

// @block scenery_groundLineBand_alternate
fn groundLineBand(xy: vec2f, cell: vec4u, forest: f32) -> vec2f {
  let params_1 = (&params);
  let line = groundTreeLine(xy, cell);
  if ((line.x < forest)) {
    return vec2f(0, -1000000000);
  }
  let at = ((xy * (*params_1).forestVerge.z) + vec2f(71.30000305175781, 23.899999618530273));
  return vec2f(1f, (line.y + (((valueNoise(at) - 0.5f) * 2f) * (*params_1).forestLine.y)));
}

// @block scenery_sway
fn sway(foot: vec3f, point: vec3f, up: f32, height: f32, leaf: f32) -> vec3f {
  let wind = cameraWind();
  let t = cam.time;
  let dir = wind.toward.xy;
  let strength = clamp(wind.gustMps / SWAY_FULL_GUST_MPS, 0.0, 1.0);
  let along = dot(foot.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(0.2, 1.0, front);
  let seed = pcgHash(bitcast<u32>(floor(foot.x * 4.0)) ^ (bitcast<u32>(floor(foot.y * 4.0)) * 2654435761u));
  let phase = f32(seed >> 8u) * (6.2831853 / 16777216.0);
  let swing = sin(t * SWAY_RATE + phase) + 0.25 * sin(1.5 * t * SWAY_RATE + 1.8 * phase);
  let h = clamp(up, 0.0, 1.0);
  let reach = h * h * h;
  let carried = SWAY_LEAN + SWAY_GUST * gust + SWAY_SWING * swing * 0.5 * (1.0 + gust);
  let bend = height * SWAY_BEND * strength * carried * reach;
  // Bent, the point keeps its distance from the foot: it drops a little.
  var offset = vec3f(dir * bend, -0.5 * bend * bend / max(height * h, 1.0));
  let q = point - foot;
  let shiver = vec3f(
    sin(t * SWAY_SHIVER_RATE + q.x * 2.1 + q.y * 0.7 + phase),
    sin(t * SWAY_SHIVER_RATE * 1.3 + q.y * 1.9 + q.z * 0.8 + phase),
    sin(t * SWAY_SHIVER_RATE * 0.9 + q.z * 2.3 + q.x * 0.6 + phase),
  );
  offset += shiver * (SWAY_SHIVER_M * strength * 0.5 * (1.0 + gust) * leaf * h);
  return offset;
}

// @block scenery_standing
fn standing(local: vec3f, pose: vec4f, shape: vec4f, leaf: f32) -> vec3f {
  let at = placedPoint(local, pose);
  let height = max((shape.w / SWAY_HEART), 1f);
  return (at + sway(pose.xyz, at, (local.z / height), height, leaf));
}

// @block scenery_casterVertex_Output
struct casterVertex_Output {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block scenery_casterVertex
@vertex fn casterVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) pose: vec4f, @location(4) shape: vec4f, @location(5) tint: vec4f) -> casterVertex_Output {
  let local = vec3f((position.x * shape.x), (position.y * shape.y), (position.z * shape.z));
  let world = standing(local, pose, shape, 0f);
  return casterVertex_Output((cam.viewProj * vec4f(world, 1f)), world, placedNormal(normal, shape, pose.w), vec4f((color.xyz * tint.xyz), color.w), local, vec3f(pose.x, pose.y, (pose.z + shape.w)));
}
