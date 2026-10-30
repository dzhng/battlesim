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

// @block post_bloom_highpassLayout
@group(0) @binding(2) var<uniform> bloom: Bloom;

// @block post_bloom_finalLayout
@group(0) @binding(2) var bloom: texture_2d<f32>;

// @block post_bloom_compositeLayout
@group(0) @binding(6) var<uniform> bloom: Bloom;

// @block post_compositeShader
fn compositeShader(uv: vec2f) -> vec4f {
  return (textureSample(level0,linearSampler,uv)*bloom.weights.x+textureSample(level1,linearSampler,uv)*bloom.weights.y+textureSample(level2,linearSampler,uv)*bloom.weights.z+textureSample(level3,linearSampler,uv)*bloom.weights.w+textureSample(level4,linearSampler,uv)*bloom.weight4)*bloom.strength;
}

// @block post_directShader
fn directShader(uv: vec2f) -> vec4f {
  let hdr = textureSample(scene, linearSampler, uv);
  let exposure = grade.exposure;
  let alpha = clamp(hdr.a, 0.0, 1.0);
  var straight = vec3f(0);
  if (alpha > 0.0) { straight = hdr.rgb / alpha; }
  return vec4f(outputSrgb(agx(straight, exposure)) * alpha, alpha);
}

// @block post_finalShader
fn finalShader(uv: vec2f) -> vec4f {
  let world = textureSample(scene,linearSampler,uv);
  let hdr = world.rgb+textureSample(bloom,linearSampler,uv).rgb;
  // The sky pass leaves the scene's alpha 0 where nothing covers it (the
  // bloom's own alpha is no coverage, so it is not read); the sky takes
  // `skyGradeMix` of the grade, everything in front of it all of it.
  let gradeShare = select(1.0, grade.skyGradeMix, world.a < 0.5);
  let graded = mix(hdr, gradeColor(hdr,grade), gradeShare);
  return vec4f(outputSrgb(agx(graded,grade.exposure)),1.0);
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

// @block post_highPlaneShader
fn highPlaneShader(uv: vec2f) -> vec4f {
  let hdr = textureSample(scene,linearSampler,uv);
  let glow = textureSample(bloom,linearSampler,uv).rgb;
  let alpha = clamp(hdr.a, 0.0, 1.0);
  var straight = vec3f(0);
  if (alpha > 0.0) { straight = hdr.rgb / alpha; }
  return vec4f(outputSrgb(agx(gradeColor(straight+glow,grade),grade.exposure))*alpha,alpha);
}

// @block post_highpass
fn highpass(uv: vec2f) -> vec4f {
  let c = textureSample(source,linearSampler,uv);
  return mix(vec4f(0),c,smoothstep(bloom.threshold,bloom.knee,dot(c.rgb,vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B))));
}

// @block post_level0
@group(0) @binding(1) var level0: texture_2d<f32>;

// @block post_level1
@group(0) @binding(2) var level1: texture_2d<f32>;

// @block post_level2
@group(0) @binding(3) var level2: texture_2d<f32>;

// @block post_level3
@group(0) @binding(4) var level3: texture_2d<f32>;

// @block post_level4
@group(0) @binding(5) var level4: texture_2d<f32>;

// @block post_lit
@group(0) @binding(2) var lit: texture_2d<f32>;

// @block post_outputSrgb
fn outputSrgb(linear: vec3f) -> vec3f {
  return select(pow(linear,vec3f(0.41666))*1.055-0.055,linear*12.92,linear<=vec3f(0.0031308));
}

// @block post_scene
@group(0) @binding(1) var scene: texture_2d<f32>;

// @block post_source_sampled
@group(0) @binding(0) var source: texture_2d<f32>;

// @block post_source_blurLayout
@group(0) @binding(1) var source: texture_2d<f32>;

// @block post_blurStep
@group(0) @binding(2) var<uniform> blurStep: vec2f;

// @block post_gaussianBlur_radius21
fn blurShader(uv: vec2f) -> vec4f {
  let sampleStep = blurStep;
    let weights = array<f32,22>(0.0544009090909091,0.05389745955754261,0.05241489372361353,0.050034020247207635,0.0468813777814855,0.043118099390876144,0.03892630025024841,0.0344945849088966,0.030004265132440738,0.025617653185595862,0.021469402862030917,0.01766138936290091,0.014261133355105735,0.011303358265657105,0.008793974889708197,0.006715637191174865,0.005034003418001028,0.0037039410220253534,0.0026750930659104446,0.0018964351865657352,0.001319657984652459,0.0009013823526604976);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 22; i++) {
      let offset = sampleStep*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block post_gaussianBlur_radius13
fn blurShader(uv: vec2f) -> vec4f {
  let sampleStep = blurStep;
    let weights = array<f32,14>(0.08548714285714286,0.08354678756262175,0.07798599458785281,0.06952826326819299,0.05920577275680463,0.048153142304813504,0.03740615924103117,0.027753611855987157,0.019667710568697906,0.013312082894952657,0.008605896616157115,0.0053137870025046,0.0031337912693315652,0.001765199910796836);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 14; i++) {
      let offset = sampleStep*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block post_gaussianBlur_radius17
fn blurShader(uv: vec2f) -> vec4f {
  let sampleStep = blurStep;
    let weights = array<f32,18>(0.06649000000000001,0.06557291119230299,0.06289684508761083,0.058677219052849755,0.05324102991993857,0.04698504399475956,0.040328223564293,0.03366625515093221,0.027334856195822894,0.021586142554656674,0.016579428361602425,0.012385123131260133,0.008998442982402379,0.006358734236915492,0.004370289867713112,0.0029213667166203597,0.0018993201471647547,0.001201009762281257);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 18; i++) {
      let offset = sampleStep*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block post_gaussianBlur_radius9
fn blurShader(uv: vec2f) -> vec4f {
  let sampleStep = blurStep;
    let weights = array<f32,10>(0.119682,0.11441569062074908,0.09996680944212386,0.07982511867716392,0.05825548349780134,0.03885505659838202,0.023684912103725177,0.013195003369491407,0.006718320685514796,0.0031262625741366435);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 10; i++) {
      let offset = sampleStep*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block post_gaussianBlur_radius5
fn blurShader(uv: vec2f) -> vec4f {
  let sampleStep = blurStep;
    let weights = array<f32,6>(0.19947,0.17603165715854927,0.12098467069287899,0.06475842766397003,0.026995328947207138,0.008764100149861079);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 6; i++) {
      let offset = sampleStep*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block post_linearSampler_blurLayout
@group(0) @binding(0) var linearSampler: sampler;

// @block post_rawShader
fn rawShader(uv: vec2f) -> vec4f {
  let texel = textureSample(scene, linearSampler, uv);
  return vec4f(texel.xyz, 1f);
}
