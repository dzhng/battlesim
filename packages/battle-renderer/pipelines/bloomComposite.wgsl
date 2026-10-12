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

fn compositeAlgorithm(uv: vec2f, level0: texture_2d<f32>, level1: texture_2d<f32>, level2: texture_2d<f32>, level3: texture_2d<f32>, level4: texture_2d<f32>, linearSampler: sampler, bloom: Bloom) -> vec4f {
  return (textureSample(level0,linearSampler,uv)*bloom.weights.x+textureSample(level1,linearSampler,uv)*bloom.weights.y+textureSample(level2,linearSampler,uv)*bloom.weights.z+textureSample(level3,linearSampler,uv)*bloom.weights.w+textureSample(level4,linearSampler,uv)*bloom.weight4)*bloom.strength;
}

@group(0) @binding(1) var level0: texture_2d<f32>;

@group(0) @binding(2) var level1: texture_2d<f32>;

@group(0) @binding(3) var level2: texture_2d<f32>;

@group(0) @binding(4) var level3: texture_2d<f32>;

@group(0) @binding(5) var level4: texture_2d<f32>;

@group(0) @binding(0) var linearSampler: sampler;

@group(0) @binding(6) var<uniform> bloom: Bloom;

fn compositeShader(uv: vec2f) -> vec4f {
  return compositeAlgorithm(uv, level0, level1, level2, level3, level4, linearSampler, bloom);
}

struct fragment_Input {
  @location(0) uv: vec2f,
}

@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return compositeShader(_arg_0.uv);
}