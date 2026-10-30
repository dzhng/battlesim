// WGSL owner: water.

// @block water_WaterLook
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

// @block water_waterNormal
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
// @block water_sceneColourMip_taps
@group(0) @binding(1) var taps: sampler;

// @block water_sceneMip
fn sceneMip(pos: vec2f) -> vec4f {
    let size = max(vec2u(1u), textureDimensions(source) / 2u);
    return textureSampleLevel(source, taps, pos / vec2f(size), 0.0);
  }

// @block water_tapped
fn tapped(u: vec2f, size: vec2f) -> vec4i {
    let last = size - 1.0;
    let first = clamp(floor(u - 1.0 / 256.0), vec2f(0.0), last);
    let end = clamp(floor(u + 1.0 / 256.0) + 1.0, vec2f(0.0), last);
    return vec4i(vec2i(first), vec2i(end));
  }

// @block water_guardMip
fn guardMip(pos: vec2f) -> vec4f {
    let before = vec2f(textureDimensions(source));
    let size = max(vec2f(1.0), floor(before / 2.0));
    let taps = tapped(pos / size * before - 0.5, before);
    var worst = vec2f(0.0);
    for (var y = taps.y; y <= taps.w; y++) {
      for (var x = taps.x; x <= taps.z; x++) {
        worst = max(worst, textureLoad(source, vec2i(x, y), 0).xy);
      }
    }
    return vec4f(worst, 0.0, 0.0);
  }

// @block water_waterGuardSeed_mask
@group(0) @binding(0) var mask: texture_2d<f32>;

// @block water_waterGuardSeed_paint
@group(0) @binding(1) var paint: texture_2d<f32>;

// @block water_guardSeed
fn guardSeed(pos: vec2f) -> vec4f {
    let p = vec2i(pos);
    let unseen = textureLoad(mask, p, 0).x;
    return vec4f(unseen, textureLoad(paint, p, 0).a, 0.0, 0.0);
  }

// @block water_waterNormalsSlope
fn waterNormalsSlope(uv: vec2f) -> vec4f {
  var slope = vec2f(0.0);
  { let c = cos(6.283185307179586*(2.0*uv.x + 0.0*uv.y) + 0);
    slope += vec2f(2.0, 0.0) * (0.03657989402359994 * c); }
  { let c = cos(6.283185307179586*(-2.0*uv.x + 2.0*uv.y) + 3.883222148137428);
    slope += vec2f(-2.0, 2.0) * (0.02175053511945528 * c); }
  { let c = cos(6.283185307179586*(0.0*uv.x + -3.0*uv.y) + 1.4832589890952703);
    slope += vec2f(0.0, -3.0) * (0.019911572267311942 * c); }
  { let c = cos(6.283185307179586*(2.0*uv.x + 3.0*uv.y) + 5.366481137232698);
    slope += vec2f(2.0, 3.0) * (0.015112277585657587 * c); }
  { let c = cos(6.283185307179586*(-4.0*uv.x + -1.0*uv.y) + 2.9665179781905406);
    slope += vec2f(-4.0, -1.0) * (0.012358072690023082 * c); }
  { let c = cos(6.283185307179586*(4.0*uv.x + -2.0*uv.y) + 0.566554819148381);
    slope += vec2f(4.0, -2.0) * (0.010939939989078317 * c); }
  { let c = cos(6.283185307179586*(-1.0*uv.x + 5.0*uv.y) + 4.4497769672858105);
    slope += vec2f(-1.0, 5.0) * (0.008985814014380072 * c); }
  { let c = cos(6.283185307179586*(-2.0*uv.x + -5.0*uv.y) + 2.049813808243654);
    slope += vec2f(-2.0, -5.0) * (0.008279210753227175 * c); }
  { let c = cos(6.283185307179586*(5.0*uv.x + 2.0*uv.y) + 5.933035956381081);
    slope += vec2f(5.0, 2.0) * (0.008279210753227175 * c); }
  { let c = cos(6.283185307179586*(-6.0*uv.x + 2.0*uv.y) + 3.5330727973389213);
    slope += vec2f(-6.0, 2.0) * (0.006504927236357362 * c); }
  { let c = cos(6.283185307179586*(3.0*uv.x + -6.0*uv.y) + 1.133109638296762);
    slope += vec2f(3.0, -6.0) * (0.005954949064424633 * c); }
  { let c = cos(6.283185307179586*(2.0*uv.x + 7.0*uv.y) + 5.016331786434194);
    slope += vec2f(2.0, 7.0) * (0.005267208793390278 * c); }
  { let c = cos(6.283185307179586*(-6.0*uv.x + -4.0*uv.y) + 2.6163686273920352);
    slope += vec2f(-6.0, -4.0) * (0.005342996979995973 * c); }
  { let c = cos(6.283185307179586*(8.0*uv.x + -2.0*uv.y) + 0.21640546834988122);
    slope += vec2f(8.0, -2.0) * (0.004369238500755799 * c); }
  { let c = cos(6.283185307179586*(-5.0*uv.x + 7.0*uv.y) + 4.099627616487308);
    slope += vec2f(-5.0, 7.0) * (0.004100753333794961 * c); }
  { let c = cos(6.283185307179586*(-1.0*uv.x + -9.0*uv.y) + 1.6996644574451487);
    slope += vec2f(-1.0, -9.0) * (0.0037968815354002448 * c); }
  { let c = cos(6.283185307179586*(7.0*uv.x + 6.0*uv.y) + 5.582886605582575);
    slope += vec2f(7.0, 6.0) * (0.003695925787053853 * c); }
  { let c = cos(6.283185307179586*(-10.0*uv.x + 0.0*uv.y) + 3.182923446540416);
    slope += vec2f(-10.0, 0.0) * (0.0032718051858603107 * c); }
  { let c = cos(6.283185307179586*(7.0*uv.x + -7.0*uv.y) + 0.7829602874982566);
    slope += vec2f(7.0, -7.0) * (0.0033217571590202404 * c); }
  { let c = cos(6.283185307179586*(0.0*uv.x + 11.0*uv.y) + 4.666182435635683);
    slope += vec2f(0.0, 11.0) * (0.002835948949106844 * c); }
  { let c = cos(6.283185307179586*(-7.0*uv.x + -8.0*uv.y) + 2.266219276593524);
    slope += vec2f(-7.0, -8.0) * (0.0029852355431249666 * c); }
  { let c = cos(6.283185307179586*(11.0*uv.x + 2.0*uv.y) + 6.149441424730962);
    slope += vec2f(11.0, 2.0) * (0.002767610226483606 * c); }
  { let c = cos(6.283185307179586*(-10.0*uv.x + 7.0*uv.y) + 3.749478265688803);
    slope += vec2f(-10.0, 7.0) * (0.0024260387597942828 * c); }
  { let c = cos(6.283185307179586*(3.0*uv.x + -12.0*uv.y) + 1.3495151066466433);
    slope += vec2f(3.0, -12.0) * (0.002378312198083263 * c); }
  return vec4f(slope, 0.0, 1.0);
}

// @block water_waveStep_was
@group(0) @binding(0) var was: texture_2d<f32>;

// @block water_WaveRun
struct WaveRun {
  shift: vec2i,
  step: u32,
  splats: u32,
  still: u32,
  pad0: u32,
  pad1: u32,
  pad2: u32,
}

// @block water_waveStep_run
@group(0) @binding(1) var<uniform> run: WaveRun;

// @block water_WaveSplat
struct WaveSplat {
  splat: vec4f,
  step: f32,
  kind: f32,
}

// @block water_waveStep_splats
@group(0) @binding(2) var<storage, read> splats: array<WaveSplat, WAVE_SPLATS_MAX>;

// @block water_waveStep
fn waveStep(pos: vec2f) -> vec4f {
    let size = vec2i(textureDimensions(was));
    let c = vec2i(pos);
    let shift = select(vec2i(0), run.shift, run.step == 0u);
    var h: array<f32, 9>;
    var i = 0;
    for (var y = -1; y <= 1; y++) {
      for (var x = -1; x <= 1; x++) {
        let at = clamp(c + vec2i(x, y), vec2i(0), size - 1) + shift;
        let inside = all(at >= vec2i(0)) && all(at < size);
        h[i] = select(0.0, textureLoad(was, at, 0).r, inside);
        i++;
      }
    }
    let here = c + shift;
    let was = select(vec4f(0.0), textureLoad(was, here, 0),
      all(here >= vec2i(0)) && all(here < size));
    if (run.still == 1u) { return was; }
    let laplacian = ((h[1] + h[3] + h[5] + h[7]) * 0.5 + (h[0] + h[2] + h[6] + h[8]) * 0.25
      - 3.0 * h[4]) * 0.333333;
    var next = (2.0 - WAVE_DAMPING) * h[4] - (1.0 - WAVE_DAMPING) * was.g
      + 4.0 * WAVE_SPEED * WAVE_SPEED * laplacian;
    var push = 0.0;
    for (var k = 0u; k < run.splats; k++) {
      let s = splats[k];
      if (u32(s.step) != run.step) { continue; }
      // A ring, not a mound: a dip with a rim, which sums to nothing, so
      // pushes made again and again never pile the water up.
      let d2 = dot(pos - s.splat.xy, pos - s.splat.xy) / max(s.splat.z * s.splat.z, 0.25);
      push += s.splat.w * (1.0 - d2) * exp(-d2);
    }
    next = clamp(next + clamp(push, -4.0, 4.0) * 0.25, -1.0, 1.0);
    let edge = min(min(c.x, size.x - 1 - c.x), min(c.y, size.y - 1 - c.y));
    next *= smoothstep(0.0, 24.0, f32(edge));
    return vec4f(next, h[4], 0.0, 0.0);
  }

// @block water_waterCarried
fn waterCarried() -> vec2f {
  let wind = cameraWind();
  return (wind.toward.xy * (cam.time * wind.driftMps));
}

// @block water_waterNormals
@group(1) @binding(8) var waterNormals: texture_2d<f32>;

// @block water_waterTiles
@group(1) @binding(9) var waterTiles: sampler;

// @block water_waterLayer
fn waterLayer(xy: vec2f, scale: f32, turn: vec2f, wind: vec2f, strength: f32, footprint: f32) -> vec3f {
  let held = (1f - smoothstep(1.5f, 6f, ((footprint * scale) * 256f)));
  let at = (xy - wind);
  let uv = (vec2f(((at.x * turn.x) + (at.y * turn.y)), ((at.y * turn.x) - (at.x * turn.y))) * scale);
  let slope = textureSampleLevel(waterNormals, waterTiles, uv, 0).xy;
  let along = vec2f(((slope.x * turn.x) - (slope.y * turn.y)), ((slope.x * turn.y) + (slope.y * turn.x)));
  return normalize(vec3f((along * (-(strength) * held)), 1f));
}

// @block water_blendRnm
fn blendRnm(base: vec3f, detail: vec3f) -> vec3f {
  let t = (base + vec3f(0, 0, 1));
  let u = (detail * vec3f(-1, -1, 1));
  return normalize(((t * (dot(t, u) / t.z)) - u));
}

// @block water_waves
@group(2) @binding(13) var waves: texture_2d<f32>;

// @block water_sceneTaps
@group(2) @binding(11) var sceneTaps: sampler;

// @block water_waveHeight
fn waveHeight(uv: vec2f) -> f32 {
  return textureSampleLevel(waves, sceneTaps, uv, 0).x;
}

// @block water_waveNormal
fn waveNormal(xy: vec2f, n: vec3f) -> vec3f {
  let area = (&cam.waves);
  let low = f32((params.view.z == 1u));
  let shown = ((*area).w * (1f - low));
  let uv = ((xy - (*area).xy) / (*area).z);
  let inside = min(uv, (vec2f(1) - uv));
  if (((shown <= 0f) || (min(inside.x, inside.y) <= 0f))) {
    return n;
  }
  const texel = 1.0 / WAVE_TEXELS;
  let dx = (waveHeight((uv + vec2f(texel, 0f))) - waveHeight((uv - vec2f(texel, 0f))));
  let dy = (waveHeight((uv + vec2f(0f, texel))) - waveHeight((uv - vec2f(0f, texel))));
  let edge = smoothstep(0f, 0.08f, min(inside.x, inside.y));
  let ripple = (vec2f(dx, dy) * (((16.0 * WAVE_NORMAL) * shown) * edge));
  let flatten = (saturate((length(ripple) * 1.8f)) * 0.72f);
  let calmed = normalize(mix(n, vec3f(0, 0, 1), flatten));
  return blendRnm(calmed, normalize(vec3f((ripple * -1f), 1f)));
}

// @block water_worldWater_depth
@group(2) @binding(9) var depth: texture_depth_multisampled_2d;

// @block water_bedDistance
fn bedDistance(at: vec2i, world: vec3f) -> f32 {
    let cam = cam;
    let ndc = ndcAt(at);
    var far = 0.0;
    for (var s = 0; s < 4; s++) {
      let p = cam.invViewProj * vec4f(ndc, textureLoad(depth, at, s), 1.0);
      far += min(length(p.xyz / p.w - world), 50.0);
    }
    return far / 4.0;
  }

// @block water_waterDepth
fn waterDepth(clip: vec4f, world: vec3f) -> f32 {
  return bedDistance(vec2i(clip.xy), world);
}

// @block water_sceneColour
@group(2) @binding(10) var sceneColour: texture_2d<f32>;

// @block water_seenUnder
@group(2) @binding(12) var seenUnder: texture_2d<f32>;

// @block water_waterUnder
fn waterUnder(clip: vec4f, world: vec3f, n: vec3f, straight: f32, look: u32) -> vec4f {
    let here = vec2i(clip.xy);
    if (params.view.z == 1u) {
      return vec4f(textureLoad(sceneColour, here, 0).rgb, straight);
    }
    let r = params.waterLooks[look].refraction;
    let cam = cam;
    let screen = vec2f(cam.width, cam.height);
    let shift = vec3f(n.xy * (min(straight, r.y) * r.x), 0.0);
    let a = cam.viewProj * vec4f(world, 1.0);
    let b = cam.viewProj * vec4f(world + shift, 1.0);
    var px = (b.xy / b.w - a.xy / a.w) * vec2f(0.5, -0.5) * screen;
    px *= min(1.0, r.z * cam.height / max(length(px), 1e-6));
    let to = vec2i(clamp(clip.xy + px, vec2f(0.0), screen - 1.0));
    let ahead = cam.invViewProj * vec4f(ndcAt(to), textureLoad(depth, to, 0), 1.0);
    let behind = length(ahead.xyz / ahead.w - cam.eye) - length(world - cam.eye);
    let unseen = textureLoad(seenUnder, to, 0).x
      - textureLoad(seenUnder, here, 0).x;
    let keep = smoothstep(0.0, 0.15, behind) * (1.0 - smoothstep(0.0, 0.5, unseen));
    let at = clip.xy + px * keep;
    let far = bedDistance(vec2i(at), world);
    let level = min(far / r.w, f32(textureNumLevels(sceneColour) - 1));
    // The blurred tap averages its footprint's unfogged colours, and a
    // swamp's bed keeps its paint. Where any of that footprint is seen less
    // than the water's own pixel, or is marked, the bed is read at the
    // pixel, unshifted and unblurred: nothing unseen shows through, and the
    // bed's mark lies under the surface's, so it shows once. The guard's
    // level holds the worst of each texel's footprint (`guardMip`), so its
    // texels under the tap, at either level the sampler may pick, say it
    // exactly.
    let mine = textureLoad(seenUnder, here, 0).x;
    var worst = vec2f(0.0, textureLoad(paint, here, 0).a);
    for (var m = u32(floor(level)); m <= u32(ceil(level)); m++) {
      let size = vec2f(textureDimensions(seenUnder, m));
      let taps = tapped(at / screen * size - 0.5, size);
      for (var y = taps.y; y <= taps.w; y++) {
        for (var x = taps.x; x <= taps.z; x++) {
          worst = max(worst, textureLoad(seenUnder, vec2i(x, y), m).xy);
        }
      }
    }
    if (worst.x > mine || worst.y > 0.0) {
      return vec4f(textureLoad(sceneColour, here, 0).rgb, straight);
    }
    let colour = textureSampleLevel(sceneColour, sceneTaps, at / screen, level);
    return vec4f(colour.rgb, far);
  }

// @block water_slopeAt
fn slopeAt(uv: vec2f) -> vec2f {
  return textureSampleLevel(waterNormals, waterTiles, uv, 0).xy;
}

// @block water_crest
fn crest(apart: f32, width: f32) -> f32 {
  let blur = (0.5f * fwidth(apart));
  let c = ((1f - smoothstep(0f, (width + blur), abs(apart))) * (width / (width + blur)));
  return (c * c);
}

// @block water_crests
fn crests(a: vec2f, b: vec2f, sharpness: f32) -> f32 {
  let one = crest(((((a.x + a.y) - b.x) + b.y) * sharpness), 0.85f);
  let two = crest(((((a.x - a.y) - b.x) - b.y) * sharpness), 0.85f);
  return ((one + two) * 0.9f);
}

// @block water_waterCaustics
fn waterCaustics(world: vec3f, far: f32, footprint: f32, look: u32) -> f32 {
  if ((params.view.z == 1u)) {
    return 0f;
  }
  let w = (&params.waterLooks[look]);
  let c = (&(*w).caustics);
  let view = normalize((cam.eye - world));
  let bed = (world - (view * far));
  let depth_1 = max((world.z - bed.z), 0f);
  let up = abs(normalize(cross(dpdx(bed), dpdy(bed))).z);
  let level = smoothstep(0.3f, 0.7f, up);
  let sun_1 = environmentSun();
  let through = (bed.xy + (sun_1.xy * ((0.75f * depth_1) / max(sun_1.z, 0.3f))));
  let carried = waterCarried();
  let uv1 = ((through - (carried * (*w).drifts.x)) * (*c).x);
  let uv2 = ((through - (carried * (*w).drifts.z)) * ((*c).x * 1.8f));
  let s1 = slopeAt((uv1 + (slopeAt(uv2) * 0.18f)));
  let s2 = slopeAt((uv2 + (slopeAt(uv1) * 0.18f)));
  let fade = (exp((-(depth_1) * (*c).z)) * smoothstep(0.01f, 0.08f, depth_1));
  let held = (1f - smoothstep(0.25f, 0.5f, (((footprint * (*c).x) * 1.8f) * 12.36931687685298f)));
  let high = clamp((sun_1.z * 1.5f), 0.35f, 1f);
  return (((((crests(s1, s2, (*c).w) * (*c).y) * fade) * level) * held) * high);
}

// @block water_lobe
fn lobe(roughness: f32, n: vec3f, world: vec3f, sun_1: f32, eye: vec3f) -> vec3f {
  return surfaceLight(vec3f(), vec3f(), roughness, 0f, 0.5f, 1f, n, world, sun_1, eye);
}

// @block water_waterReflection
fn waterReflection(n: vec3f, world: vec3f, sun_1: f32, eye: vec3f, look: u32) -> vec3f {
  let w = (&params.waterLooks[look]);
  let sharp = (lobe((*w).roughness.x, n, world, sun_1, eye) * (*w).lobes.x);
  let sheen = (lobe((*w).roughness.y, n, world, sun_1, eye) * (*w).lobes.y);
  let broad = (lobe((*w).roughness.z, n, world, sun_1, eye) * (*w).lobes.z);
  return ((sharp + sheen) + broad);
}

// @block water_waterTransmission
fn waterTransmission(n: vec3f, view: vec3f) -> f32 {
  let edgeOn = (1f - clamp(dot(n, view), 0f, 1f));
  return (1f - (0.02f + (0.98f * pow(edgeOn, 5f))));
}

// @block water_atPixel
fn atPixel(clip: vec4f) -> vec3f {
    let p = cam.invViewProj * vec4f(ndcAt(vec2i(clip.xy)), clip.z, 1.0);
    return p.xyz / p.w;
  }
