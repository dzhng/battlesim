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
    let weights = array<f32,6>(0.19947,0.17603165715854927,0.12098467069287899,0.06475842766397003,0.026995328947207138,0.008764100149861079);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 6; i++) {
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