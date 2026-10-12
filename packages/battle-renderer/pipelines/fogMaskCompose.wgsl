struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

@group(0) @binding(2) var lit: texture_2d<f32>;

@group(0) @binding(3) var mask: texture_2d<f32>;

struct FogMaskView {
  mask: u32,
}

@group(0) @binding(1) var<uniform> view: FogMaskView;

struct FogStyleUniform {
  tint: vec3f,
  dim: f32,
  cool: f32,
  saturation: f32,
  veil: f32,
  lineStrength: f32,
  lineSpacingPx: f32,
  lineWidthPx: f32,
  lineAngle: f32,
  lineFloor: f32,
  edgeSoftnessPx: f32,
  rimColor: vec3f,
  rimAlpha: f32,
  rimWidthPx: f32,
  reachPx: f32,
}

@group(0) @binding(0) var<uniform> look: FogStyleUniform;

@group(0) @binding(4) var distance_1: texture_2d<f32>;

fn fogLook(lit: vec3f, pixel: vec2f, s: FogStyleUniform) -> vec3f {
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

fn composeFog(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let lit = textureLoad(lit, p, 0);
  let m = textureLoad(mask, p, 0).xy;
  if (view.mask == 1u) {
    // Unseen (more than half the pixel's samples) black; untouched (none)
    // white; partly unseen grey.
    let v = select(select(0.35, 16.0, m.x <= 0.0), 0.0, m.x > 0.5);
    return vec4f(vec3f(v), 1.0);
  }
  if (view.mask == 2u) {
    // Mostly ground (more than half the pixel's samples) white, else black.
    let ground = textureLoad(mask, p, 0).z > 0.5;
    return vec4f(vec3f(select(0.0, 16.0, ground)), 1.0);
  }
  if (view.mask == 3u) {
    // The terrain's bytes where every sample of the pixel is ground; black
    // where anything else covers any of it.
    let ground = textureLoad(mask, p, 0).z >= 0.999;
    return vec4f(select(vec3f(0.0), lit.xyz, ground), 1.0);
  }
  if (m.x <= 0.0) { return lit; }
  let s = look;
  var unseen = m.x;
  if (s.edgeSoftnessPx > 0.0) {
    let toSeen = textureLoad(distance_1, p, 0).y * 16.0;
    unseen *= smoothstep(0.0, 1.0, (toSeen - 0.5) / s.edgeSoftnessPx);
  }
  return vec4f(mix(lit.xyz, fogLook(lit.xyz, pixel, s), unseen), lit.w);
}

@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return composeFog(pos.xy);
}