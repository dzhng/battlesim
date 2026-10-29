struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

@group(0) @binding(0) var was: texture_2d<f32>;

struct WaveRun {
  shift: vec2i,
  step: u32,
  splats: u32,
  still: u32,
}

@group(0) @binding(1) var<uniform> run: WaveRun;

struct WaveSplat {
  splat: vec4f,
  step: f32,
  kind: f32,
}

@group(0) @binding(2) var<storage, read> splats: array<WaveSplat, 1536>;

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
    var next = (2.0 - 0.08) * h[4] - (1.0 - 0.08) * was.g
      + 4.0 * 0.2 * 0.2 * laplacian;
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

@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return waveStep(pos.xy);
}