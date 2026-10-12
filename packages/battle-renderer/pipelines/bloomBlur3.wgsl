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
    let weights = array<f32,18>(0.06649000000000001,0.06557291119230299,0.06289684508761083,0.058677219052849755,0.05324102991993857,0.04698504399475956,0.040328223564293,0.03366625515093221,0.027334856195822894,0.021586142554656674,0.016579428361602425,0.012385123131260133,0.008998442982402379,0.006358734236915492,0.004370289867713112,0.0029213667166203597,0.0018993201471647547,0.001201009762281257);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 18; i++) {
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