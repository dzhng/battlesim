struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

@group(0) @binding(1) var mask: texture_2d<f32>;

struct FogLook {
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

@group(0) @binding(0) var<uniform> look: FogLook;

fn groundSide(m: vec4f) -> vec2f {
  if (m.z <= 0.5) { return vec2f(0.0); }
  return vec2f(f32(m.x > 0.5 && m.y <= 0.0), f32(m.y > 0.5 && m.x <= 0.0));
}

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

@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceRows(pos.xy);
}