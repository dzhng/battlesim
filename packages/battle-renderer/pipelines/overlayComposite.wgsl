struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

@group(0) @binding(1) var overlay: texture_2d<f32>;

struct OverlayGlow {
  radiusPx: f32,
  sigmaPx: f32,
  strength: f32,
  pad: f32,
}

@group(0) @binding(0) var<uniform> glow: OverlayGlow;

@group(0) @binding(2) var halo: texture_2d<f32>;

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

@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return compositeOverlay(pos.xy);
}