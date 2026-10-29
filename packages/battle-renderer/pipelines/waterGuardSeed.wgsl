struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

@group(0) @binding(0) var mask: texture_2d<f32>;

@group(0) @binding(1) var paint: texture_2d<f32>;

fn guardSeed(pos: vec2f) -> vec4f {
    let p = vec2i(pos);
    let unseen = textureLoad(mask, p, 0).x;
    return vec4f(unseen, textureLoad(paint, p, 0).a, 0.0, 0.0);
  }

@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return guardSeed(pos.xy);
}