// WGSL owner: scenery.

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
  water: vec4f,
  waterEdge: vec4f,
  waterLook: vec4f,
  shore: vec4f,
  shoreEarth: vec4f,
  shoreEdge: vec4f,
  distant: vec4f,
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
  let feather = max(footprint, (params.forestVerge.x * 0.1f));
  return smoothstep(-(feather), feather, forestVergeInside(xy, forest));
}

// @block scenery_forestFloor_alternate
fn forestFloor(xy: vec2f, footprint: f32, noise: f32) -> vec3f {
  let params_1 = (&params);
  let detail = (&(*params_1).forestDetail);
  let at = (xy * (*detail).x);
  let turned = vec2f(((at.x * 0.8f) - (at.y * 0.6f)), ((at.x * 0.6f) + (at.y * 0.8f)));
  let across = vec2f(((at.x * 0.28f) + (at.y * 0.96f)), ((at.y * 0.28f) - (at.x * 0.96f)));
  let moss = smoothstep(0.38f, 0.72f, ((valueNoise((turned + vec2f(13.300000190734863, 7.699999809265137))) * 0.65f) + (valueNoise(((across * 2.1f) + vec2f(4.099999904632568, 9.199999809265137))) * 0.35f)));
  let humus = smoothstep(0.42f, 0.78f, ((valueNoise(((across * 1.3f) + vec2f(2.9000000953674316, 41.099998474121094))) * 0.6f) + (valueNoise(((turned * 3.1f) + vec2f(17.299999237060547, 5.900000095367432))) * 0.4f)));
  var floor_1 = mix((*params_1).forestLitter.xyz, (*params_1).forestMoss.xyz, (moss * (*params_1).forestMoss.w));
  floor_1 = mix(floor_1, (*params_1).forestHumus.xyz, (humus * (*params_1).forestHumus.w));
  let rootsAt = (xy * (*params_1).forestVerge.w);
  let ridge = abs((valueNoise((rootsAt + vec2f(29.100000381469727, 3.299999952316284))) - 0.5f));
  let runs = smoothstep(0.55f, 0.75f, valueNoise(((rootsAt * 1.7f) + vec2f(8.300000190734863, 17.899999618530273))));
  let shown = (1f - smoothstep(0.02f, 0.08f, (footprint * (*params_1).forestVerge.w)));
  let line = ((1f - smoothstep(0.015f, 0.035f, ridge)) * runs);
  let root = ((line * shown) + (0.02f * (1f - shown)));
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
fn forestVergeInside(xy: vec2f, forest: f32) -> f32 {
  let verge = (&params.forestVerge);
  let wander = ((valueNoise(((xy * (*verge).z) + vec2f(71.30000305175781, 23.899999618530273))) - 0.5f) * 2f);
  let patches = (valueNoise(((xy * 0.7f) + vec2f(5.099999904632568, 91.69999694824219))) - 0.5f);
  return (((forest + ((*verge).x * 0.5f)) + (wander * (*verge).y)) + (patches * (*verge).x));
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
fn groundColour(xy: vec2f, footprint: f32, site: vec4f, paved: GroundPaved, water: f32) -> vec4f {
  let params_1 = (&params);
  let aa = (footprint * 0.5f);
  let plot = (&plots[i32(site.x)]);
  let variation = mottle(xy, footprint, (*plot).rows.xy, site.x, (*plot).dry.xy);
  let noise = variation.x;
  var albedo = groundTint(((*plot).colour.xyz * fieldTexture(xy, footprint, i32(site.x))), ((*plot).detail.x * variation.y));
  var roughness = (*plot).colour.w;
  let verge = groundVerge(site, footprint);
  let vergeColour = ((*params_1).verge.xyz * (1f + (0.18f * noise)));
  albedo = mix(albedo, vergeColour, verge);
  roughness = mix(roughness, 0.95f, verge);
  let inRegion = rectInside(xy, (*params_1).region);
  let distant = max((1f - smoothstep(0f, 600f, inRegion)), smoothstep((*params_1).feathers.y, (*params_1).feathers.z, footprint));
  albedo = mix(albedo, (*params_1).distant.xyz, distant);
  let floor_1 = groundFloor(xy, footprint, site.w);
  if ((floor_1.x > 0f)) {
    albedo = mix(albedo, forestFloor(xy, footprint, noise), floor_1.x);
    roughness = mix(roughness, (*params_1).forestDetail.z, floor_1.x);
  }
  if ((floor_1.y > 0f)) {
    albedo = mix(albedo, vergeColour, floor_1.y);
    roughness = mix(roughness, 0.95f, floor_1.y);
  }
  let shore = groundShore(xy, footprint, water);
  if ((shore.y > 0f)) {
    let field = mix(dot((*params_1).verge.xyz, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)), (dot((*plot).colour.xyz, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023)) * (1f - (0.5f * (*plot).rows.w))), smoothstep(0f, 10f, site.y));
    let least = (field * (*params_1).shoreEdge.z);
    let earth = ((*params_1).shoreEarth.xyz * (1f + (0.6f * noise)));
    albedo = mix(albedo, atLeast(earth, least), shore.y);
    roughness = mix(roughness, 0.92f, shore.y);
    albedo = mix(albedo, atLeast((*params_1).shore.xyz, least), shore.x);
    roughness = mix(roughness, 0.8f, shore.x);
  }
  let paving = groundRoads(xy, footprint, paved, vec4f(albedo, roughness));
  let bed = smoothstep(-(aa), max(aa, 0.3f), water);
  albedo = mix(paving.xyz, (*params_1).waterBed.xyz, bed);
  return vec4f(max(albedo, vec3f()), paving.w);
}

// @block scenery_groundDapple
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

// @block scenery_groundFloor_alternate
fn groundFloor(xy: vec2f, footprint: f32, forest: f32) -> vec2f {
  let params_1 = (&params);
  let wood = forestFloorWeight(xy, forest, footprint);
  if (((wood > 0f) || (forest > (-((*params_1).forestVerge.x) - (*params_1).forestVerge.y)))) {
    let band = groundLineBand(xy, groundCell(xy, footprint), forest);
    if ((band.x > 0.5f)) {
      let feather = max(footprint, ((*params_1).forestVerge.x * 0.1f));
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
