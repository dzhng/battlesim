// WGSL owner: effects.

// @block effects_OverlayGlow
struct OverlayGlow {
  radiusPx: f32,
  sigmaPx: f32,
  strength: f32,
  pad: f32,
}

// @block effects_RoadLook
struct RoadLook {
  core: vec4f,
  worn: vec4f,
  shape: vec4f,
  join: vec4f,
  shoulder: vec4f,
  edge: vec4f,
  ruts: vec4f,
  track: vec4f,
  slabs: vec4f,
  curb: vec4f,
  paint: vec4f,
  marks: vec4f,
  crossing: vec4f,
  stones: vec4f,
}

// @block effects_clouds
fn clouds(dir: vec3f, sky: vec3f) -> vec3f {
  if (dir.z < 0.012) { return sky; }
  // A dome, not a plane: far clouds keep their bulk instead of thinning to
  // streaks along the horizon.
  let at = dir.xy * (light.cloudReach / (dir.z + 0.12));
  let n = fbm(at);
  let density = smoothstep(light.cloudEdge.x, light.cloudEdge.y, n);
  // Volume: read the field as each cloud's height. Its slope faces the sun
  // or turns away; the cloud between a sample and the sun shades it; thick
  // cores grey toward their bases. Tops and sunward flanks stay bright.
  let e = 0.02;
  let slope = vec2f(fbm(at + vec2f(e, 0.0)) - n, fbm(at + vec2f(0.0, e)) - n) / e;
  let normal = normalize(vec3f(-slope * 0.35, 1.0));
  let facing = clamp(dot(normal, light.sunDirection) * 0.5 + 0.5, 0.0, 1.0);
  let towardSun = fbm(at + light.cloudToSun);
  let shadow = smoothstep(light.cloudLow, light.cloudShadowTop, towardSun) * (1.0 - facing);
  let core = smoothstep(light.cloudLow, light.cloudCoreTop, n);
  let lit = light.cloudLit;
  let base = light.cloudBase + sky * 0.25;
  var cloud = mix(base, lit, clamp(facing * 1.2 - shadow * 0.5 - core * 0.3, 0.0, 1.0));
  // Thin edges toward the sun glow (forward scattering).
  let silver = pow(max(dot(dir, light.sunDirection), 0.0), 6.0) * (1.0 - core);
  cloud += light.sunTransmittance * silver * light.cloudSilver;
  // Far clouds pile toward the horizon and take its haze.
  let near = smoothstep(0.0, 0.3, dir.z);
  cloud = mix(mix(sky, cloud, 0.55), cloud, near);
  let alpha = density * light.cloudOpacity * smoothstep(0.012, 0.04, dir.z);
  return mix(sky, cloud, alpha);
}

// @block effects_compositeOverlay
fn compositeOverlay(pixel: vec2f) -> vec4f {
  let o = textureLoad(overlay, vec2i(pixel), 0);
  let g = glow;
  if (g.strength <= 0.0) { return o; }
  let top = vec2i(textureDimensions(halo)) - 1;
  let at = pixel * 0.5 - 0.5;
  let p0 = vec2i(floor(at));
  let f = at - floor(at);
  let a = textureLoad(halo, clamp(p0, vec2i(0), top), 0);
  let b = textureLoad(halo, clamp(p0 + vec2i(1, 0), vec2i(0), top), 0);
  let c = textureLoad(halo, clamp(p0 + vec2i(0, 1), vec2i(0), top), 0);
  let e = textureLoad(halo, clamp(p0 + vec2i(1, 1), vec2i(0), top), 0);
  let blur = mix(mix(a, b, f.x), mix(c, e, f.x), f.y);
  let gain = min(g.strength, 1.0 / max(blur.w, 1e-4));
  return o + (1.0 - o.w) * blur * gain;
}

// @block effects_glow
@group(0) @binding(0) var<uniform> glow: OverlayGlow;

// @block effects_glowColumns
fn glowColumns(pixel: vec2f) -> vec4f {
  let g = glow;
  let size = vec2i(textureDimensions(rows));
  let p = vec2i(pixel);
  let reach = i32(ceil(g.radiusPx * 0.5));
  var sum = vec4f(0.0);
  var total = 0.0;
  for (var k = -reach; k <= reach; k++) {
    let y = f32(k) * 2.0;
    let w = exp(-(y * y) / (2.0 * g.sigmaPx * g.sigmaPx));
    sum += w * textureLoad(rows, clamp(p + vec2i(0, k), vec2i(0), size - 1), 0);
    total += w;
  }
  return sum / total;
}

// @block effects_glowRows
fn glowRows(pixel: vec2f) -> vec4f {
  let g = glow;
  let size = vec2i(textureDimensions(overlay));
  let base = vec2i(pixel) * 2;
  let reach = i32(ceil(g.radiusPx * 0.5));
  var sum = vec4f(0.0);
  var total = 0.0;
  for (var k = -reach; k <= reach; k++) {
    let x = f32(k) * 2.0;
    let w = exp(-(x * x) / (2.0 * g.sigmaPx * g.sigmaPx));
    for (var dy = 0; dy < 2; dy++) {
      for (var dx = 0; dx < 2; dx++) {
        let p = clamp(base + vec2i(k * 2 + dx, dy), vec2i(0), size - 1);
        sum += w * textureLoad(overlay, p, 0);
      }
    }
    total += 4.0 * w;
  }
  return sum / total;
}

// @block effects_highlightGlow
fn highlightGlow(highlight: f32) -> vec3f {
  return (vec3f(0.949999988079071, 0.800000011920929, 0.20000000298023224) * (highlight * 0.7f));
}

// @block effects_overlay
@group(0) @binding(1) var overlay: texture_2d<f32>;

// @block effects_overlayFragment_Input
struct overlayFragment_Input {
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block effects_overlayVertex_Output
struct overlayVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block effects_paintColour
fn paintColour(paint: vec4f) -> vec3f {
  let c = pow(max(paint.xyz / max(paint.w, 1e-4), vec3f(0.0)), vec3f(2.2));
  let grey = dot(c, vec3f(0.2126, 0.7152, 0.0722));
  return max(mix(vec3f(grey), c, paintStyle.saturation), vec3f(0.0));
}

// @block effects_paintFragment_Input
struct paintFragment_Input {
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block effects_paintGlow
fn paintGlow(paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  return paintColour(paint) * paintStyle.emissive * paint.w;
}

// @block effects_paintMarchVertex_Output
struct paintMarchVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block effects_paintVertex_Output
struct paintVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block effects_paint_primary
@group(2) @binding(7) var paint: texture_2d<f32>;

// @block effects_paintedAlbedo
fn paintedAlbedo(albedo: vec3f, paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return albedo; }
  let colour = min(paintColour(paint), vec3f(1.0));
  return mix(albedo, colour * paintStyle.albedo, paint.w);
}
