struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

struct Grade {
  saturation: f32,
  contrast: f32,
  splitTone: f32,
  shadowLift: f32,
  exposure: f32,
  shadowTint: vec4f,
  highlightTint: vec4f,
  lift: vec4f,
}

fn outputSrgb(linear: vec3f) -> vec3f {
  return select(pow(linear,vec3f(0.41666))*1.055-0.055,linear*12.92,linear<=vec3f(0.0031308));
}

fn agx(color: vec3f, exposure: f32) -> vec3f {
  let to2020 = mat3x3f(vec3f(0.6274,0.0691,0.0164),vec3f(0.3293,0.9195,0.0880),vec3f(0.0433,0.0113,0.8956));
  let inset = mat3x3f(vec3f(0.856627153315983,0.137318972929847,0.11189821299995),vec3f(0.0951212405381588,0.761241990602591,0.0767994186031903),vec3f(0.0482516061458583,0.101439036467562,0.811302368396859));
  let outset = mat3x3f(vec3f(1.1271005818144368,-0.1413297634984383,-0.14132976349843826),vec3f(-0.11060664309660323,1.157823702216272,-0.11060664309660294),vec3f(-0.016493938717834573,-0.016493938717834257,1.2519364065950405));
  let toSrgb = mat3x3f(vec3f(1.6605,-0.1246,-0.0182),vec3f(-0.5876,1.1329,-0.1006),vec3f(-0.0728,-0.0083,1.1187));
  let x = clamp((log2(max(inset*(to2020*(color*exposure)),vec3f(1e-10)))+12.47393)/(4.026069+12.47393),vec3f(0),vec3f(1));
  let x2 = x*x;
  let x4 = x2*x2;
  let contrast = 15.5*(x4*x2)-40.14*(x4*x)+(31.96*x4-6.868*(x2*x)+(0.4298*x2+(0.1191*x-0.00232)));
  return clamp(toSrgb*pow(max(vec3f(0),outset*contrast),vec3f(2.2)),vec3f(0),vec3f(1));
}

const LUMA: vec3f = vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023);

fn gradeColor(input: vec3f, params: Grade) -> vec3f {
  let base = max(input, vec3f(0));
  let baseLuma = max(dot(base,LUMA),0.0001);
  let mapped = baseLuma/(baseLuma+1.0);
  let curve = mapped*mapped*(3.0-mapped*2.0);
  let l = mix(mapped,curve,params.contrast);
  let contrastLuma = l/max(0.0001,1.0-l);
  var graded = base*(contrastLuma/baseLuma);
  let shadowTint = mix(vec3f(1),mix(params.shadowTint.xyz,vec3f(1),smoothstep(0.0,0.34,l)),params.splitTone);
  let highlightTint = mix(vec3f(1),mix(vec3f(1),params.highlightTint.xyz,smoothstep(0.44,0.98,l)),params.splitTone);
  let tinted = graded*shadowTint*highlightTint;
  graded = tinted*(dot(graded,LUMA)/max(dot(tinted,LUMA),0.0001));
  let lift = params.lift.xyz*params.shadowLift*(1.0-smoothstep(0.08,0.38,l))*0.45;
  graded = graded*(vec3f(1)-lift)+lift;
  let midtone = smoothstep(0.1,0.42,l)*(1.0-smoothstep(0.62,0.96,l));
  let saturation = 1.0+params.saturation*(0.55+midtone*0.45);
  return max(mix(vec3f(dot(graded,LUMA)),graded,saturation),vec3f(0));
}

fn highPlaneAlgorithm(uv: vec2f, scene: texture_2d<f32>, bloom: texture_2d<f32>, linearSampler: sampler, grade: Grade) -> vec4f {
  let hdr = textureSample(scene,linearSampler,uv);
  let glow = textureSample(bloom,linearSampler,uv).rgb;
  let alpha = clamp(hdr.a, 0.0, 1.0);
  var straight = vec3f(0);
  if (alpha > 0.0) { straight = hdr.rgb / alpha; }
  return vec4f(outputSrgb(agx(gradeColor(straight+glow,grade),grade.exposure))*alpha,alpha);
}

@group(0) @binding(1) var scene: texture_2d<f32>;

@group(0) @binding(2) var bloom: texture_2d<f32>;

@group(0) @binding(0) var linearSampler: sampler;

@group(0) @binding(3) var<uniform> grade: Grade;

fn highPlaneShader(uv: vec2f) -> vec4f {
  return highPlaneAlgorithm(uv, scene, bloom, linearSampler, grade);
}

struct fragment_Input {
  @location(0) uv: vec2f,
}

@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return highPlaneShader(_arg_0.uv);
}