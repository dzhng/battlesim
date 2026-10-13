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

@group(0) @binding(2) var rows: texture_2d<f32>;

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

fn distanceColumns(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  if (textureLoad(mask, p, 0).z <= 0.5) { return vec4f(1.0, 1.0, 0.0, 1.0); }
  let height = i32(textureDimensions(rows).y);
  let reach = i32(look.reachPx);
  var best = vec2f(16.0 * 16.0);
  for (var k = -reach; k <= reach; k++) {
    let y = p.y + k;
    if (y < 0 || y >= height) { continue; }
    let row = textureLoad(rows, vec2i(p.x, y), 0).xy * 16.0;
    best = min(best, row * row + vec2f(f32(k * k)));
  }
  return vec4f(min(sqrt(best), vec2f(16.0)) / 16.0, 0.0, 1.0);
}

@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceColumns(pos.xy);
}