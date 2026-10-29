// WGSL owner: fog_sight.

// @block fog_sight_FogBox
struct FogBox {
  center: vec2f,
  half: vec2f,
  yaw: f32,
  top: f32,
  base: f32,
}

// @block fog_sight_FogEye
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

// @block fog_sight_FogLayer
struct FogLayer {
  ground: u32,
  seen: u32,
  painted: u32,
  pad: u32,
}

// @block fog_sight_FogParams
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

// @block fog_sight_FogProbe
struct FogProbe {
  position: vec3f,
  ground: u32,
  normal: vec3f,
  pad: u32,
}

// @block fog_sight_Frag
struct Frag {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
};

// @block fog_sight_GroundPaintStyle
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

// @block fog_sight_Out
struct Out {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) color: vec4f,
  @location(2) @interpolate(flat) misc: vec4f,
  @location(3) @interpolate(flat) extra: vec4f,
  @location(4) along: f32,
  @location(5) viewDepth: f32,
  /** A lit sprite's sun in its own frame (right, up, toward the eye), and
   *  the sky's light on its top and on its side. */
  @location(6) @interpolate(flat) sun: vec3f,
  @location(8) @interpolate(flat) skyTop: vec3f,
  @location(9) @interpolate(flat) skySide: vec3f,
  /** Where on the sprite, screen-aligned, -1..1. */
  @location(7) offset: vec2f,
  /** A lit sprite's cast light (castAt), along a ribbon from end to end. */
  @location(10) castLit: vec3f,
  /** A ground shadow's view ray (from the eye to its quad), in the shadow's
   *  own frame: along its nose, across it, and up from its ground. */
  @location(11) ray: vec3f,
};

// @block fog_sight_SpriteLight
/** The world's light on a lit sprite: the sun in the sprite's own frame
 *  (the camera's right, up, and toward the eye), and the sky's diffuse
 *  light (its PMREM at full roughness, times the fill, as world materials
 *  take it) on its top and on its side. */
struct SpriteLight {
  sun: vec3f,
  skyTop: vec3f,
  skySide: vec3f,
};

// @block fog_sight_backdropFragment_Output
struct backdropFragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_boxes
@group(0) @binding(0) var<storage, read> boxes: array<FogBox>;

// @block fog_sight_clipOf
fn clipOf(position: vec3f) -> vec4f {
  let toEye = normalize((cam.eye - position));
  let drawn = (position + (toEye * 1f));
  return (cam.viewProj * vec4f(drawn, 1f));
}

// @block fog_sight_eyes_alternate
@group(0) @binding(1) var<storage, read> eyes: array<FogEye>;

// @block fog_sight_eyes_extended
@group(0) @binding(2) var<storage, read> eyes: array<FogEye>;

// @block fog_sight_eyes_layered
@group(3) @binding(2) var<storage, read> eyes: array<FogEye>;

// @block fog_sight_eyes_masked
@group(2) @binding(2) var<storage, read> eyes: array<FogEye>;

// @block fog_sight_eyes_primary
@group(1) @binding(2) var<storage, read> eyes: array<FogEye>;

// @block fog_sight_fogBinEdge
fn fogBinEdge(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}

// @block fog_sight_fogCoverage_alternate
fn fogCoverage(seen: f32, alpha: f32) -> vec4f {
  if (params_1.enabled == 0u || layer.seen == 1u) {
    return vec4f(0.0, 0.0, 0.0, alpha);
  }
  return vec4f(1.0 - seen, seen, f32(layer.ground), alpha);
}

// @block fog_sight_fogHeight
fn fogHeight(q: vec2f) -> f32 {
  let P = params;
  let nx = P.heightNx;
  let ny = P.heightNy;
  let fx = clamp(q.x / P.heightSpacing, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / P.heightSpacing, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u);
  let j = min(u32(fy), ny - 2u);
  let u = fx - f32(i);
  let v = fy - f32(j);
  return triangleRuleHeight(
    fogSample(i, j), fogSample(i + 1u, j),
    fogSample(i, j + 1u), fogSample(i + 1u, j + 1u), u, v);
}

// @block fog_sight_fogIsGround_alternate
fn fogIsGround() -> bool {
  return (layer_1.ground == 1u);
}

// @block fog_sight_fogLook
fn fogLook(lit: vec3f, pixel: vec2f, s: FogLook) -> vec3f {
  let lum = dot(lit, vec3f(0.2126, 0.7152, 0.0722));
  let kept = mix(vec3f(lum), lit, s.saturation);
  let c = mix(kept, lum * s.tint, s.cool) * s.dim;
  // Distance to the nearest line, in pixels, across the lines' direction.
  let across = vec2f(sin(s.lineAngle), cos(s.lineAngle));
  let t = dot(pixel, across) / s.lineSpacingPx;
  let dist = abs(fract(t + 0.5) - 0.5) * s.lineSpacingPx;
  let cover = clamp(s.lineWidthPx * 0.5 + 0.5 - dist, 0.0, 1.0);
  return c * (1.0 + s.lineStrength * cover) + s.tint * (s.veil + s.lineFloor * cover);
}

// @block fog_sight_fogPack
fn fogPack(horizon: f32, jump: f32, foliage: f32) -> u32 {
  let h = pack2x16float(vec2f(clamp(horizon, -6e4, 6e4), 0.0)) & 0xffffu;
  return h | (u32(round(clamp(jump, 0.0, 1.0) * 255.0)) << 16u) | (min(u32(round(foliage / 0.005)), 255u) << 24u);
}

// @block fog_sight_fogProbePoint_alternate
fn fogProbePoint(world: vec3f, normal: vec3f, ground: bool) -> vec3f {
  if (ground) { return world + vec3f(0.0, 0.0, params_1.targetHeightM); }
  return world + normal * params_1.faceProbeM;
}

// @block fog_sight_fogSample
fn fogSample(i: u32, j: u32) -> f32 {
    let size = readWord(0u);
    let cols = readWord(1u);
    let count = readWord(2u);
    if (count == 0u) { return 0.0; }
    let page = readWord(3u + (j / size) * cols + i / size);
    if (page == 0u) { return 0.0; }
    let at = 3u + count + (page - 1u) * size * size + (j % size) * size + i % size;
    return bitcast<f32>(readWord(at));
  }

// @block fog_sight_fogTerm_alternate
fn fogTerm(world: vec3f, normal: vec3f, pixel: vec2f, isGround: bool) -> f32 {
  let P = params_1;
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

// @block fog_sight_fogUnpack
fn fogUnpack(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * 0.005);
}

// @block fog_sight_fogUnpackBuild
fn fogUnpackBuild(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * 0.005);
}

// @block fog_sight_fogWholeSeen_alternate
fn fogWholeSeen(p: vec3f) -> bool {
  let P = params_1;
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

// @block fog_sight_fogWholeWord
fn fogWholeWord(i: u32) -> u32 {
  return textureLoad(wholes, vec2u(i % 256u, i / 256u), 0).x;
}

// @block fog_sight_forestFragment_Output
struct forestFragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_fragment_Output
struct fragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_grassDensity
fn grassDensity(dist: f32, drop: f32) -> f32 {
  let P = &params;
  let facing = clamp(drop / dist, 0.15, 1.0);
  let footprint = (*P).eye.w * dist;
  return min((*P).density.y, facing / ((*P).density.x * footprint * footprint));
}

// @block fog_sight_groundScarsSeen
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

// @block fog_sight_item_record
struct item_1 {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_layer_uniform
@group(2) @binding(1) var<uniform> layer_1: FogLayer;

// @block fog_sight_layer_alternate
@group(3) @binding(1) var<uniform> layer: FogLayer;

// @block fog_sight_layer_primary
@group(1) @binding(1) var<uniform> layer: FogLayer;

// @block fog_sight_lit_Output
struct lit_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_look
@group(0) @binding(0) var<uniform> look: FogLook;

// @block fog_sight_modelSeen_alternate
fn modelSeen(world: vec3f, normal: vec3f, anchor: vec3f, pixel: vec2f) -> f32 {
  if (fogIsGround()) {
    return fogTerm(anchor, vec3f(), pixel, true);
  }
  return fogTerm(world, normal, pixel, false);
}

// @block fog_sight_occluders
@group(0) @binding(4) var<storage, read> occluders: array<FogBox>;

// @block fog_sight_paintedSeen
fn paintedSeen(seen: f32, paint: vec4f) -> f32 {
  return mix(seen, mix(1.0, seen, paintStyle.fogKeep), paint.w);
}

// @block fog_sight_params_uniform
@group(2) @binding(0) var<uniform> params_1: FogParams;

// @block fog_sight_params_extended
@group(3) @binding(0) var<uniform> params: FogParams;

// @block fog_sight_params_masked
@group(0) @binding(0) var<uniform> params: FogParams;

// @block fog_sight_params_primary
@group(1) @binding(0) var<uniform> params: FogParams;

// @block fog_sight_points
@group(1) @binding(0) var<storage, read> points: array<FogProbe>;

// @block fog_sight_shade_lit
fn shade_1(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec4f {
  return air(surfaceLight(base, emissive, roughness, geomRoughness, metal, ao, normal, position, shadow, eye), position, eye);
}

// @block fog_sight_smokeLight
/** The world's light on smoke at `o` on a lit sprite (-1..1, screen-
 *  aligned): a soft ball's (or, across a ribbon, a tube's) normal against the
 *  sun, wrapped (light scatters through smoke), and the sun scattered on
 *  toward the eye, strongest looking into it (Henyey-Greenstein, relative to
 *  isotropic): backlit dust and smoke glow rather than going dark. Plus the
 *  sky's light, more from above. */
fn smokeLight(f: Out, o: vec2f) -> vec3f {
  let n = normalize(vec3f(o, sqrt(max(0.0, 1.0 - dot(o, o))) + 0.3));
  let wrap = clamp((dot(n, f.sun) + 0.3) / 1.3, 0.0, 1.0);
  let g = 0.4;
  let forward = (1.0 - g * g) / pow(1.0 + g * g + 2.0 * g * f.sun.z, 1.5);
  let sky = mix(f.skySide, f.skyTop, clamp(n.y * 0.5 + 0.5, 0.0, 1.0));
  return light.sunRadiance.rgb * ((0.6 * wrap + 0.4 * forward) / PI) + sky + f.castLit;
}

// @block fog_sight_surface_alternate
fn surface(world: vec3f, normal: vec3f, color: vec4f, local: vec3f, sun_1: f32) -> vec3f {
  let eye = (&cam_1.eye);
  var n = normalize(normal);
  var shade = 1f;
  var roughness = 0.8999999761581421f;
  let fade = (1f - smoothstep(0.08f, 0.45f, length(fwidth(world))));
  if (((color.w > 0.5f) && (fade > 0f))) {
    let clumps = leafClumps(local, n, fade);
    n = clumps.xyz;
    shade = clumps.w;
    roughness = 0.85f;
  }
  let lit = shade_1((color.xyz * shade), vec3f(), roughness, 0f, 0f, 1f, n, world, sun_1, (*eye));
  return lit.xyz;
}

// @block fog_sight_terrainFragment_Output
struct terrainFragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_unlit
fn unlit(colour: vec3f, position: vec3f, eye: vec3f) -> vec4f {
  return unlitAlgorithm(colour, position, eye, data.observer.xyz, data.sunDirection.xyz, data.sunRadiance.xyz, data.fill.xyz, data.settings.y, data.settings.x, data.aerial, sky, pmrem, linear);
}

// @block fog_sight_unlitAlgorithm
fn unlitAlgorithm(colour: vec3f, worldPosition: vec3f, eye: vec3f, observer: vec3f, sunDirection: vec3f, sunRadiance: vec3f, environmentIntensity: vec3f, shadowFloor: f32, maxMip: f32, air: Aerial, sky: texture_2d<f32>, pmrem: texture_2d<f32>, linear: sampler) -> vec4f {
    // What a white matte surface facing the sky returns in sun shadow: the
    // sky's light and the shadow floor's share of the sun's, as shade and
    // standardPbr's diffuse terms have them.
    let skyLight=samplePmrem(pmrem,linear,vec3f(0.0,0.0,1.0),1.0,maxMip)*environmentIntensity;
    let white=sunRadiance*max(sunDirection.z,0.0)*shadowFloor*0.3183098861837907+skyLight;
    return applyAerial(vec4f(colour*white,1),worldPosition,eye,observer,sunDirection,air,sky,linear);
  }

// @block fog_sight_waterFragment_Output
struct waterFragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_whole
@group(0) @binding(1) var<uniform> whole: vec4f;

// @block fog_sight_wholeFound
var<workgroup> wholeFound: atomic<u32>;

// @block fog_sight_wholes_alternate
@group(2) @binding(8) var wholes: texture_2d<u32>;

// @block fog_sight_wholes_extended
@group(3) @binding(8) var wholes: texture_2d<u32>;

// @block fog_sight_wholes_primary
@group(1) @binding(8) var wholes: texture_2d<u32>;

// @block fog_sight_worldFragment_Output
struct worldFragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_sight_Camera
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

// @block fog_sight_Wind
struct Wind {
  toward: vec3f,
  driftMps: f32,
  gustMps: f32,
  gustPerM: f32,
}

// @block fog_sight_cameraWind
fn cameraWind() -> Wind { return cam.wind[0]; }
