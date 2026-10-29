struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

@group(0) @binding(0) var source: texture_2d<f32>;

fn tapped(u: vec2f, size: vec2f) -> vec4i {
    let last = size - 1.0;
    let first = clamp(floor(u - 1.0 / 256.0), vec2f(0.0), last);
    let end = clamp(floor(u + 1.0 / 256.0) + 1.0, vec2f(0.0), last);
    return vec4i(vec2i(first), vec2i(end));
  }

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

@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return guardMip(pos.xy);
}