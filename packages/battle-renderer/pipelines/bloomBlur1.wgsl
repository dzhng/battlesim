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
    let weights = array<f32,10>(0.119682,0.11441569062074908,0.09996680944212386,0.07982511867716392,0.05825548349780134,0.03885505659838202,0.023684912103725177,0.013195003369491407,0.006718320685514796,0.0031262625741366435);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 10; i++) {
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