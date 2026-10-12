struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

fn algorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, step: vec2f) -> vec4f {
    let weights = array<f32,22>(0.0544009090909091,0.05389745955754261,0.05241489372361353,0.050034020247207635,0.0468813777814855,0.043118099390876144,0.03892630025024841,0.0344945849088966,0.030004265132440738,0.025617653185595862,0.021469402862030917,0.01766138936290091,0.014261133355105735,0.011303358265657105,0.008793974889708197,0.006715637191174865,0.005034003418001028,0.0037039410220253534,0.0026750930659104446,0.0018964351865657352,0.001319657984652459,0.0009013823526604976);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 22; i++) {
      let offset = step*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

@group(0) @binding(1) var source: texture_2d<f32>;

@group(0) @binding(0) var linearSampler: sampler;

@group(0) @binding(2) var<uniform> step_1: vec2f;

fn shader(uv: vec2f) -> vec4f {
  return algorithm(uv, source, linearSampler, step_1);
}

struct fragment_Input {
  @location(0) uv: vec2f,
}

@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return shader(_arg_0.uv);
}