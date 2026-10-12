struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

struct Bloom {
  weights: vec4f,
  weight4: f32,
  strength: f32,
  threshold: f32,
  knee: f32,
}

fn highpassAlgorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, bloom: Bloom) -> vec4f {
  let c = textureSample(source,linearSampler,uv);
  return mix(vec4f(0),c,smoothstep(bloom.threshold,bloom.knee,dot(c.rgb,vec3f(0.2126,0.7152,0.0722))));
}

@group(0) @binding(1) var source: texture_2d<f32>;

@group(0) @binding(0) var linearSampler: sampler;

@group(0) @binding(2) var<uniform> bloom: Bloom;

fn highpass(uv: vec2f) -> vec4f {
  return highpassAlgorithm(uv, source, linearSampler, bloom);
}

struct fragment_Input {
  @location(0) uv: vec2f,
}

@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return highpass(_arg_0.uv);
}