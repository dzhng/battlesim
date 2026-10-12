struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

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

@group(0) @binding(3) var mask: texture_2d<f32>;

@group(0) @binding(4) var distance_1: texture_2d<f32>;

fn rimAt(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let s = look;
  let m = textureLoad(mask, p, 0).xy;
  let seen = m.y;
  if (seen <= 0.0 || m.x > 0.5 || s.rimAlpha <= 0.0 || s.rimWidthPx <= 0.0) {
    return vec4f(0.0);
  }
  let toUnseen = textureLoad(distance_1, p, 0).x * 16.0;
  let cover = clamp(s.rimWidthPx + 1.0 - toUnseen, 0.0, 1.0);
  return vec4f(s.rimColor, s.rimAlpha * cover * seen);
}

@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return rimAt(pos.xy);
}