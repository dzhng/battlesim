// WGSL owner: shared_math.

// @block shared_math_algorithm_alternate
fn algorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, step: vec2f) -> vec4f {
    let weights = array<f32,22>(0.0544009090909091,0.05389745955754261,0.05241489372361353,0.050034020247207635,0.0468813777814855,0.043118099390876144,0.03892630025024841,0.0344945849088966,0.030004265132440738,0.025617653185595862,0.021469402862030917,0.01766138936290091,0.014261133355105735,0.011303358265657105,0.008793974889708197,0.006715637191174865,0.005034003418001028,0.0037039410220253534,0.0026750930659104446,0.0018964351865657352,0.001319657984652459,0.0009013823526604976);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 22; i++) {
      let offset = step*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block shared_math_algorithm_extended
fn algorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, step: vec2f) -> vec4f {
    let weights = array<f32,14>(0.08548714285714286,0.08354678756262175,0.07798599458785281,0.06952826326819299,0.05920577275680463,0.048153142304813504,0.03740615924103117,0.027753611855987157,0.019667710568697906,0.013312082894952657,0.008605896616157115,0.0053137870025046,0.0031337912693315652,0.001765199910796836);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 14; i++) {
      let offset = step*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block shared_math_algorithm_layered
fn algorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, step: vec2f) -> vec4f {
    let weights = array<f32,18>(0.06649000000000001,0.06557291119230299,0.06289684508761083,0.058677219052849755,0.05324102991993857,0.04698504399475956,0.040328223564293,0.03366625515093221,0.027334856195822894,0.021586142554656674,0.016579428361602425,0.012385123131260133,0.008998442982402379,0.006358734236915492,0.004370289867713112,0.0029213667166203597,0.0018993201471647547,0.001201009762281257);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 18; i++) {
      let offset = step*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block shared_math_algorithm_masked
fn algorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, step: vec2f) -> vec4f {
    let weights = array<f32,10>(0.119682,0.11441569062074908,0.09996680944212386,0.07982511867716392,0.05825548349780134,0.03885505659838202,0.023684912103725177,0.013195003369491407,0.006718320685514796,0.0031262625741366435);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 10; i++) {
      let offset = step*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block shared_math_algorithm_primary
fn algorithm(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler, step: vec2f) -> vec4f {
    let weights = array<f32,6>(0.19947,0.17603165715854927,0.12098467069287899,0.06475842766397003,0.026995328947207138,0.008764100149861079);
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < 6; i++) {
      let offset = step*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }

// @block shared_math_bakeAlgorithm
fn bakeAlgorithm(uv: vec2f, face: f32, source: texture_2d<f32>, linear: sampler) -> vec4f {
      return textureSampleLevel(source,linear,equirectUv(normalize(cubeDirection(uv,face))),0);
    }

// @block shared_math_convolve
fn convolve(env: texture_2d<f32>, linear: sampler, n: vec3f, r: f32, mip: f32, maxMip: f32) -> vec3f {
  if(r<0.001){return cubeSample(env,linear,n,mip,maxMip);}
  let up=select(vec3f(1,0,0),vec3f(0,0,1),abs(n.z)<0.999);
  let tangent=normalize(cross(up,n));
  let bitangent=cross(n,tangent);
  var color=vec3f(0);var weight=0.0;
  for(var i=0u;i<512u;i++) {
    let xi=vec2f(f32(i)/512.0,radicalInverse(i));
    let ht=importanceGGX(xi,r);
    let h=normalize(tangent*ht.x+bitangent*ht.y+n*ht.z);
    let l=normalize(h*(dot(n,h)*2.0)-n);
    let nl=max(dot(n,l),0.0);
    if(nl>0.0){color+=cubeSample(env,linear,l,mip,maxMip)*nl;weight+=nl;}
  }
  if(weight>0.0){color/=weight;}
  return color;
}

// @block shared_math_coveredSamples
fn coveredSamples(coverage: f32, edge: f32, noise: f32) -> u32 {
    let threshold = mix(noise, 0.5, clamp(edge * 2.0, 0.0, 1.0));
    let samples = u32(clamp(floor(coverage * 4.0 + threshold), 0.0, 4.0));
    return (1u << samples) - 1u;
  }

// @block shared_math_crossing
fn crossing(x: f32, edge: f32, width: f32) -> f32 {
  let w = max(width, 1e-3);
  return smoothstep(edge - w, edge + w, x);
}

// @block shared_math_depth_xrayCountLayout
@group(2) @binding(1) var depth: texture_depth_multisampled_2d;

// @block shared_math_depth_cullLayout
@group(0) @binding(2) var depth: texture_depth_multisampled_2d;

// @block shared_math_fbm
fn fbm(p0: vec2f) -> f32 {
  // Warped, so masses round into heaps rather than streaks.
  var p = p0 + vec2f(noise(p0 * 0.5), noise(p0 * 0.5 + vec2f(5.2, 1.3))) * 0.9;
  var sum = 0.0;
  var amp = 0.55;
  var norm = 0.0;
  for (var k = 0; k < 5; k++) {
    sum += amp * noise(p);
    norm += amp;
    p = vec2f(p.x * 1.7 - p.y * 1.1, p.x * 1.1 + p.y * 1.7) + vec2f(17.3, 9.1);
    amp *= 0.45;
  }
  return sum / norm;
}

// @block shared_math_filterAlgorithm
fn filterAlgorithm(uv: vec2f, face: f32, source: texture_2d<f32>, linear: sampler, params: vec4f) -> vec4f {
      return vec4f(convolve(source,linear,normalize(cubeDirection(uv,face)),params.x,params.y,params.z),1);
    }

// @block shared_math_hash
fn hash(i: vec2i) -> f32 {
  var h = (bitcast<u32>(i.x) * 0x8da6b343u) ^ (bitcast<u32>(i.y) * 0xd8163841u);
  h = h ^ (h >> 13u);
  h = h * 0x5bd1e995u;
  h = h ^ (h >> 15u);
  return f32(h & 0xffffffu) / 16777216.0;
}

// @block shared_math_linearSampler_effects
@group(0) @binding(3) var linearSampler: sampler;

// @block shared_math_linearSampler_blurLayout
@group(0) @binding(0) var linearSampler: sampler;

// @block shared_math_linear_sampler
@group(2) @binding(5) var linear_2: sampler;

// @block shared_math_linear_alternate
@group(GROUP) @binding(5) var linear: sampler;

// @block shared_math_linear_sampled
@group(0) @binding(1) var linear: sampler;

// @block shared_math_linear_cardLayout
@group(1) @binding(2) var linear: sampler;

// @block shared_math_noise
fn noise(p: vec2f) -> f32 {
  let i = vec2i(floor(p));
  let f = fract(p);
  let u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2i(1, 0)), u.x), mix(hash(i + vec2i(0, 1)), hash(i + vec2i(1, 1)), u.x), u.y);
}

// @block shared_math_pcgHash
fn pcgHash(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}

// @block shared_math_pixelNoise
fn pixelNoise(pixel: vec2f) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2f(0.06711056, 0.00583715))));
  }

// @block shared_math_rawShader
fn rawShader(uv: vec2f) -> vec4f {
  let texel = textureSample(scene, linearSampler, uv);
  return vec4f(texel.xyz, 1f);
}

// @block shared_math_roadGrain
const GROUND_ROAD_GRAIN_FADE_FROM = 0.6;
const GROUND_ROAD_GRAIN_FADE_TO = 1.2;
const GROUND_ROAD_GRAIN_OCTAVE_1 = 2.03;
const GROUND_ROAD_GRAIN_OCTAVE_2 = 4.01;
const GROUND_ROAD_GRAIN_OCTAVE_3 = 8.3;
const GROUND_ROAD_GRAIN_WEIGHT_0 = 0.25;
const GROUND_ROAD_GRAIN_WEIGHT_1 = 0.35;
const GROUND_ROAD_GRAIN_WEIGHT_2 = 0.4;
const GROUND_ROAD_GRAIN_STONE_FROM = 0.7;
const GROUND_ROAD_GRAIN_STONE_TO = 0.76;
const GROUND_ROAD_GRAIN_WEIGHT_3 = 0.5;
fn roadGrain(p: vec2f, pixel: f32) -> f32 {
 let shown=vec4f(1.0)-smoothstep(vec4f(GROUND_ROAD_GRAIN_FADE_FROM),vec4f(GROUND_ROAD_GRAIN_FADE_TO),pixel*vec4f(1.0,GROUND_ROAD_GRAIN_OCTAVE_1,GROUND_ROAD_GRAIN_OCTAVE_2,GROUND_ROAD_GRAIN_OCTAVE_3));
 var grain=0.0;
 if(shown.x>0.0){grain+=(valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y))-0.5)*GROUND_ROAD_GRAIN_WEIGHT_0*shown.x;}
 if(shown.y>0.0){grain+=(valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*GROUND_ROAD_GRAIN_OCTAVE_1+vec2f(19.1,7.7))-0.5)*GROUND_ROAD_GRAIN_WEIGHT_1*shown.y;}
 if(shown.z>0.0){grain+=(valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*GROUND_ROAD_GRAIN_OCTAVE_2+vec2f(3.3,41.9))-0.5)*GROUND_ROAD_GRAIN_WEIGHT_2*shown.z;}
 if(shown.w>0.0){
  let stone=valueNoise(p*GROUND_ROAD_GRAIN_OCTAVE_3+vec2f(71.9,13.7));
  grain+=(smoothstep(GROUND_ROAD_GRAIN_STONE_FROM,GROUND_ROAD_GRAIN_STONE_TO,stone)-smoothstep(GROUND_ROAD_GRAIN_STONE_FROM,GROUND_ROAD_GRAIN_STONE_TO,1.0-stone))*GROUND_ROAD_GRAIN_WEIGHT_3*shown.w;
 }
 return grain;
}

// @block shared_math_scene_sampler
@group(0) @binding(2) var sampler_1: sampler;

// @block shared_math_sceneDepth
@group(0) @binding(1) var sceneDepth: texture_depth_multisampled_2d;

// @block shared_math_shader
fn shader(uv: vec2f) -> vec4f {
  return algorithm(uv, source, linearSampler, step_1);
}

// @block shared_math_tiled
@group(1) @binding(4) var tiled: sampler;

// @block shared_math_valueNoise
fn valueNoise(p: vec2f) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let u = f * f * (3.0 - 2.0 * f);
  let ix = bitcast<u32>(i32(cell.x));
  let iy = bitcast<u32>(i32(cell.y));
  var h = array<f32, 4>();
  for (var k = 0u; k < 4u; k++) {
    let s = pcgHash(((ix + (k & 1u)) * 1597334677u) ^ ((iy + (k >> 1u)) * 3812015801u));
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  return mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
}

// @block shared_math_valueNoise3
fn valueNoise3(p: vec3f) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let u = f * f * (3.0 - 2.0 * f);
  let i = vec3u(bitcast<vec3u>(vec3i(cell)));
  var h = array<f32, 8>();
  for (var k = 0u; k < 8u; k++) {
    let q = i + vec3u(k & 1u, (k >> 1u) & 1u, k >> 2u);
    let s = pcgHash((q.x * 1597334677u) ^ (q.y * 3812015801u) ^ (q.z * 2798796415u));
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  let x0 = mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
  let x1 = mix(mix(h[4], h[5], u.x), mix(h[6], h[7], u.x), u.y);
  return mix(x0, x1, u.z);
}

// @block shared_math_wanderNoise
const GROUND_WANDER_PUSH_SCALE = 0.61;
const GROUND_WANDER_PUSH = 1.4;
const GROUND_WANDER_OCTAVE_1 = 2.03;
const GROUND_WANDER_OCTAVE_2 = 4.01;
const GROUND_WANDER_WEIGHT_0 = 0.5;
const GROUND_WANDER_WEIGHT_1 = 0.3;
const GROUND_WANDER_WEIGHT_2 = 0.2;
fn wanderNoise(at: vec2f) -> f32 {
 let push=vec2f(valueNoise(at*GROUND_WANDER_PUSH_SCALE+vec2f(11.3,47.1)),valueNoise(at*GROUND_WANDER_PUSH_SCALE+vec2f(83.9,5.7)))-0.5;
 let p=at+push*GROUND_WANDER_PUSH;
 let a=valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y));
 let b=valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*GROUND_WANDER_OCTAVE_1+vec2f(19.1,7.7));
 let c=valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*GROUND_WANDER_OCTAVE_2+vec2f(3.3,41.9));
 return a*GROUND_WANDER_WEIGHT_0+b*GROUND_WANDER_WEIGHT_1+c*GROUND_WANDER_WEIGHT_2;
}

// @block shared_math_cubeDirection
fn cubeDirection(uv:vec2f,face:f32)->vec3f {
  let p=uv*2.0-vec2f(1);
  if(face==0){return vec3f(1,p.y,p.x);}
  if(face==1){return vec3f(-p.x,1,-p.y);}
  if(face==2){return vec3f(-p.x,p.y,1);}
  if(face==3){return vec3f(-1,p.y,-p.x);}
  if(face==4){return vec3f(-p.x,-1,p.y);}
  return vec3f(p.x,p.y,-1);
}

// @block shared_math_cubeFace
fn cubeFace(d: vec3f) -> f32 {
  let a=abs(d);
  if(a.x>a.z) {
    if(a.x>a.y){return select(3.0,0.0,d.x>0.0);}
    return select(4.0,1.0,d.y>0.0);
  }
  if(a.z>a.y){return select(5.0,2.0,d.z>0.0);}
  return select(4.0,1.0,d.y>0.0);
}

// @block shared_math_cubeSample
fn cubeSample(env: texture_2d<f32>, linear: sampler, d: vec3f, mip: f32, maxMip: f32) -> vec3f {
  var face=cubeFace(d);
  let filterIndex=max(4.0-mip,0.0);
  let size=exp2(max(mip,4.0));
  var uv=cubeUv(d,face)*(size-2.0)+vec2f(1);
  if(face>2.0){uv.y+=size;face-=3.0;}
  uv.x+=face*size;
  uv.x+=filterIndex*48.0;
  uv.y+=4.0*(exp2(maxMip)-size);
  uv/=vec2f(textureDimensions(env));
  return textureSampleGrad(env,linear,uv,vec2f(0),vec2f(0)).rgb;
}

// @block shared_math_cubeUv
fn cubeUv(d: vec3f, face: f32) -> vec2f {
  var p=vec2f(0);
  if(face==0){p=vec2f(d.z,d.y)/abs(d.x);}
  else if(face==1){p=vec2f(-d.x,-d.z)/abs(d.y);}
  else if(face==2){p=vec2f(-d.x,d.y)/abs(d.z);}
  else if(face==3){p=vec2f(-d.z,d.y)/abs(d.x);}
  else if(face==4){p=vec2f(-d.x,d.z)/abs(d.y);}
  else{p=vec2f(d.x,d.y)/abs(d.z);}
  return 0.5*(p+vec2f(1));
}
