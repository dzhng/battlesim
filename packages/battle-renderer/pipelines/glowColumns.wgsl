struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

struct OverlayGlow {
  radiusPx: f32,
  sigmaPx: f32,
  strength: f32,
  pad: f32,
}

@group(0) @binding(0) var<uniform> glow: OverlayGlow;

@group(0) @binding(1) var rows: texture_2d<f32>;

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

@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowColumns(pos.xy);
}