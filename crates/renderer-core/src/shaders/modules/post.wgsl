// WGSL owner: post.

// @block post_Bloom
struct Bloom {
  weights: vec4f,
  weight4: f32,
  strength: f32,
  threshold: f32,
  knee: f32,
}

// @block post_Grade
struct Grade {
  saturation: f32,
  contrast: f32,
  splitTone: f32,
  shadowLift: f32,
  exposure: f32,
  skyGradeMix: f32,
  shadowTint: vec4f,
  highlightTint: vec4f,
  lift: vec4f,
}

// @block post_agx
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

// @block post_bloom_alternate
@group(0) @binding(2) var<uniform> bloom: Bloom;

// @block post_bloom_extended
@group(0) @binding(2) var bloom: texture_2d<f32>;

// @block post_bloom_primary
@group(0) @binding(6) var<uniform> bloom: Bloom;

// @block post_compositeAlgorithm
fn compositeAlgorithm(uv: vec2f, level0: texture_2d<f32>, level1: texture_2d<f32>, level2: texture_2d<f32>, level3: texture_2d<f32>, level4: texture_2d<f32>, linearSampler: sampler, bloom: Bloom) -> vec4f {
  return (textureSample(level0,linearSampler,uv)*bloom.weights.x+textureSample(level1,linearSampler,uv)*bloom.weights.y+textureSample(level2,linearSampler,uv)*bloom.weights.z+textureSample(level3,linearSampler,uv)*bloom.weights.w+textureSample(level4,linearSampler,uv)*bloom.weight4)*bloom.strength;
}

// @block post_compositeShader
fn compositeShader(uv: vec2f) -> vec4f {
  return compositeAlgorithm(uv, level0, level1, level2, level3, level4, linearSampler, bloom);
}

// @block post_directAlgorithm
fn directAlgorithm(hdr: vec4f, exposure: f32) -> vec4f {
  let alpha = clamp(hdr.a, 0.0, 1.0);
  var straight = vec3f(0);
  if (alpha > 0.0) { straight = hdr.rgb / alpha; }
  return vec4f(outputSrgb(agx(straight, exposure)) * alpha, alpha);
}

// @block post_directShader
fn directShader(uv: vec2f) -> vec4f {
  return directAlgorithm(textureSample(scene, linearSampler, uv), grade.exposure);
}

// @block post_finalAlgorithm
fn finalAlgorithm(uv: vec2f, scene: texture_2d<f32>, bloom: texture_2d<f32>, linearSampler: sampler, grade: Grade) -> vec4f {
  let world = textureSample(scene,linearSampler,uv);
  let hdr = world.rgb+textureSample(bloom,linearSampler,uv).rgb;
  // The sky pass leaves the scene's alpha 0 where nothing covers it (the
  // bloom's own alpha is no coverage, so it is not read); the sky takes
  // `skyGradeMix` of the grade, everything in front of it all of it.
  let gradeShare = select(1.0, grade.skyGradeMix, world.a < 0.5);
  let graded = mix(hdr, gradeColor(hdr,grade), gradeShare);
  return vec4f(outputSrgb(agx(graded,grade.exposure)),1.0);
}

// @block post_finalShader
fn finalShader(uv: vec2f) -> vec4f {
  return finalAlgorithm(uv, scene, bloom, linearSampler, grade);
}

// @block post_grade
@group(0) @binding(3) var<uniform> grade: Grade;

// @block post_gradeColor
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

// @block post_highPlaneAlgorithm
fn highPlaneAlgorithm(uv: vec2f, scene: texture_2d<f32>, bloom: texture_2d<f32>, linearSampler: sampler, grade: Grade) -> vec4f {
  let hdr = textureSample(scene,linearSampler,uv);
  let glow = textureSample(bloom,linearSampler,uv).rgb;
  let alpha = clamp(hdr.a, 0.0, 1.0);
  var straight = vec3f(0);
  if (alpha > 0.0) { straight = hdr.rgb / alpha; }
  return vec4f(outputSrgb(agx(gradeColor(straight+glow,grade),grade.exposure))*alpha,alpha);
}

// @block post_highPlaneShader
fn highPlaneShader(uv: vec2f) -> vec4f {
  return highPlaneAlgorithm(uv, scene, bloom, linearSampler, grade);
}

// @block post_highpass
fn highpass(uv: vec2f) -> vec4f {
  return highpassAlgorithm(uv, source, linearSampler, bloom);
}

// @block post_highpassAlgorithm
fn highpassAlgorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, bloom: Bloom) -> vec4f {
  let c = textureSample(source,linearSampler,uv);
  return mix(vec4f(0),c,smoothstep(bloom.threshold,bloom.knee,dot(c.rgb,vec3f(0.2126,0.7152,0.0722))));
}
