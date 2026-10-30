// WGSL owner: sky.

// @block sky_clouds
fn clouds(dir: vec3f, sky: vec3f) -> vec3f {
  if (dir.z < 0.012) { return sky; }
  // A dome, not a plane: far clouds keep their bulk instead of thinning to
  // streaks along the horizon.
  let at = dir.xy * (light.cloudReach / (dir.z + 0.12));
  let n = fbm(at);
  let density = smoothstep(light.cloudEdge.x, light.cloudEdge.y, n);
  // Volume: read the field as each cloud's height. Its slope faces the sun
  // or turns away; the cloud between a sample and the sun shades it; thick
  // cores grey toward their bases. Tops and sunward flanks stay bright.
  let e = 0.02;
  let slope = vec2f(fbm(at + vec2f(e, 0.0)) - n, fbm(at + vec2f(0.0, e)) - n) / e;
  let normal = normalize(vec3f(-slope * 0.35, 1.0));
  let facing = clamp(dot(normal, light.sunDirection) * 0.5 + 0.5, 0.0, 1.0);
  let towardSun = fbm(at + light.cloudToSun);
  let shadow = smoothstep(light.cloudLow, light.cloudShadowTop, towardSun) * (1.0 - facing);
  let core = smoothstep(light.cloudLow, light.cloudCoreTop, n);
  let lit = light.cloudLit;
  let base = light.cloudBase + sky * 0.25;
  var cloud = mix(base, lit, clamp(facing * 1.2 - shadow * 0.5 - core * 0.3, 0.0, 1.0));
  // Thin edges toward the sun glow (forward scattering).
  let silver = pow(max(dot(dir, light.sunDirection), 0.0), 6.0) * (1.0 - core);
  cloud += light.sunTransmittance * silver * light.cloudSilver;
  // Far clouds pile toward the horizon and take its haze.
  let near = smoothstep(0.0, 0.3, dir.z);
  cloud = mix(mix(sky, cloud, 0.55), cloud, near);
  let alpha = density * light.cloudOpacity * smoothstep(0.012, 0.04, dir.z);
  return mix(sky, cloud, alpha);
}

// @block sky_bakeColor
fn bakeColor(uv: vec2f) -> vec4f {
    return max(vec4f(radiance(direction(uv)), 1.0), vec4f(0.0));
  }

// @block sky_disc
fn disc(dir: vec3f) -> vec3f {
    let disc = smoothstep(9.9955000000000005e-1f, 9.9985000000000002e-1f, dot(dir, light.sunDirection))
      * smoothstep(-0.015, 0.01, dir.z) * light.discRadiance;
    return light.sunTransmittance * disc;
  }

// @block sky_importanceGGX
fn importanceGGX(xi: vec2f, r: f32) -> vec3f {
  let alpha=r*r;
  let radius=sqrt(xi.x);
  let phi=(2.0*3.14159265359)*xi.y;
  let t1=radius*cos(phi);
  // PMREM views along the surface normal: tangent-space V=(0,0,1), s=1.
  let t2=radius*sin(phi);
  let nh=vec3f(t1,t2,sqrt(max(0.0,1.0-(t1*t1+t2*t2))));
  return normalize(vec3f(alpha*nh.x,alpha*nh.y,max(0.0,nh.z)));
}

// @block sky_bakeShade
fn bakeShade(uv: vec2f, face: f32) -> vec4f {
  return bakeAlgorithm(uv, face, source, linear);
}

// @block sky_dfg_primary
@group(GROUP) @binding(4) var dfg: texture_2d<f32>;

// @block sky_direction
fn direction(uv: vec2f) -> vec3f {
  let theta = (uv.x - 0.5) * 6.2831853071795862e+0f;
  let phi = (uv.y - 0.5) * 3.1415926535897931e+0f;
  return vec3f(cos(phi) * cos(theta), sin(phi), cos(phi) * sin(theta));
}

// @block sky_equirectUv
fn equirectUv(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * 1.5915494309189535e-1f + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * 3.1830988618379069e-1f + 0.5);
}

// @block sky_filterShade
fn filterShade(uv: vec2f, face: f32) -> vec4f {
  return filterAlgorithm(uv, face, source, linear, params);
}

// @block sky_lutView
@group(0) @binding(1) var lutView: texture_2d<f32>;

// @block sky_params
@group(0) @binding(2) var<uniform> params: vec4f;

// @block sky_roughnessMip
fn roughnessMip(r: f32) -> f32 {
  if(r>=0.8){return (1.0-r)/0.2-2.0;}
  if(r>=0.4){return (0.8-r)*3.0/0.4-1.0;}
  if(r>=0.305){return (0.4-r)/0.095+2.0;}
  if(r>=0.21){return (0.305-r)/0.095+3.0;}
  return -2.0*log2(1.16*r);
}
// @block sky_bakeAlgorithm
fn bakeAlgorithm(uv: vec2f, face: f32, source: texture_2d<f32>, linear: sampler) -> vec4f {
      return textureSampleLevel(source,linear,equirectUv(normalize(cubeDirection(uv,face))),0);
    }

// @block sky_convolve
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

// @block sky_filterAlgorithm
fn filterAlgorithm(uv: vec2f, face: f32, source: texture_2d<f32>, linear: sampler, params: vec4f) -> vec4f {
      return vec4f(convolve(source,linear,normalize(cubeDirection(uv,face)),params.x,params.y,params.z),1);
    }

// @block sky_linear_sampler
@group(2) @binding(5) var linear_2: sampler;

// @block sky_linear_alternate
@group(GROUP) @binding(5) var linear: sampler;

// @block sky_linear_sampled
@group(0) @binding(1) var linear: sampler;

// @block sky_scene_sampler
@group(0) @binding(2) var sampler_1: sampler;

// @block sky_cubeDirection
fn cubeDirection(uv:vec2f,face:f32)->vec3f {
  let p=uv*2.0-vec2f(1);
  if(face==0){return vec3f(1,p.y,p.x);}
  if(face==1){return vec3f(-p.x,1,-p.y);}
  if(face==2){return vec3f(-p.x,p.y,1);}
  if(face==3){return vec3f(-1,p.y,-p.x);}
  if(face==4){return vec3f(-p.x,-1,p.y);}
  return vec3f(p.x,p.y,-1);
}

// @block sky_cubeFace
fn cubeFace(d: vec3f) -> f32 {
  let a=abs(d);
  if(a.x>a.z) {
    if(a.x>a.y){return select(3.0,0.0,d.x>0.0);}
    return select(4.0,1.0,d.y>0.0);
  }
  if(a.z>a.y){return select(5.0,2.0,d.z>0.0);}
  return select(4.0,1.0,d.y>0.0);
}

// @block sky_cubeSample
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

// @block sky_cubeUv
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
// @block sky_light
@group(0) @binding(0) var<uniform> light: SkyLight;

// @block sky_pmrem_effects
@group(0) @binding(5) var pmrem: texture_2d<f32>;

// @block sky_pmrem_primary
@group(GROUP) @binding(3) var pmrem: texture_2d<f32>;

// @block sky_sky_primary
@group(GROUP) @binding(2) var sky: texture_2d<f32>;

// @block sky_samplePmrem
fn samplePmrem(env: texture_2d<f32>, linear: sampler, d: vec3f, r: f32, maxMip: f32) -> vec3f {
  let mip=clamp(roughnessMip(r),-2.0,maxMip);
  let f=fract(mip);
  let a=cubeSample(env,linear,d,floor(mip),maxMip);
  if(f==0){return a;}
  return mix(a,cubeSample(env,linear,d,floor(mip)+1.0,maxMip),f);
}
// @block sky_SkyLight
struct SkyLight {
  sunDirection: vec3f,
  overcast: f32,
  sunTransmittance: vec3f,
  radiance: f32,
  groundTint: vec3f,
  mieScatter: f32,
  mieExtinction: f32,
  multipleScatter: f32,
  aureole: f32,
  overcastScale: f32,
  discRadiance: f32,
  cloudReach: f32,
  cloudEdge: vec2f,
  cloudToSun: vec2f,
  cloudLow: f32,
  cloudShadowTop: f32,
  cloudCoreTop: f32,
  cloudOpacity: f32,
  cloudLit: vec3f,
  cloudSilver: f32,
  cloudBase: vec3f,
  skySaturation: f32,
  skyTint: vec3f,
}

// @block sky_backgroundColor
fn backgroundColor(uv: vec2f) -> vec4f {
    // Alpha 0 marks the sky for the post pass (its grade share).
    return max(vec4f(sample(rays.origin + rays.dx * uv.x + rays.dy * uv.y), 0.0), vec4f(0.0));
  }

// @block sky_radiance
fn radiance(dirIn: vec3f) -> vec3f {
  let betaR = vec3f(5.8019999999999999e-3f, 1.3558000000000001e-2f, 3.3099999999999997e-2f);
  let betaMScatter = light.mieScatter;
  let betaMExtinction = light.mieExtinction;
  let sunDir = light.sunDirection;
  let mu = max(dirIn.z, 0.004);
  let dir = normalize(vec3f(dirIn.xy, mu));
  let tTop = sqrt(4.0452144039999999e+7f * (mu * mu - 1.0) + 4.1731600000000000e+7f) - mu * 6.3601999999999998e+3f;
  let dt = tTop / 32.0;
  let cosTheta = dot(dir, sunDir);
  let phaseR = (cosTheta * cosTheta + 1.0) * 5.9683103659460751e-2f;
  let phaseM = 2.8647889756541152e-2f / pow(1.6400000000000001e+0f - cosTheta * 1.6000000000000001e+0f, 1.5);
  var radiance = vec3f(0.0);
  var odView = vec3f(0.0);
  for (var i = 0; i < 32; i++) {
    let t = (f32(i) + 0.5) * dt;
    let px = dir.x * t;
    let py = dir.y * t;
    let pz = dir.z * t + 6.3601999999999998e+3f;
    let rp = sqrt(px * px + py * py + pz * pz);
    let h = rp - 6.3600000000000000e+3f;
    let rhoR = exp(h / -8.0000000000000000e+0f);
    let rhoM = exp(h / -1.2000000000000000e+0f);
    odView += (betaR * rhoR + rhoM * betaMExtinction) * dt;
    let tView = exp(-odView);
    let muS = (px * sunDir.x + py * sunDir.y + pz * sunDir.z) / rp;
    let tSunTop = sqrt(rp * rp * (muS * muS - 1.0) + 4.1731600000000000e+7f) - muS * rp;
    let dts = tSunTop / 6.0;
    var odSun = vec3f(0.0);
    for (var j = 0; j < 6; j++) {
      let ts = (f32(j) + 0.5) * dts;
      let sx = px + sunDir.x * ts;
      let sy = py + sunDir.y * ts;
      let sz = pz + sunDir.z * ts;
      let hs = sqrt(sx * sx + sy * sy + sz * sz) - 6.3600000000000000e+3f;
      odSun += (betaR * exp(hs / -8.0000000000000000e+0f) + exp(hs / -1.2000000000000000e+0f) * betaMExtinction) * dts;
    }
    let horizonMu = -sqrt(max(1.0 - 4.0449600000000000e+7f / (rp * rp), 0.0));
    let shadow = smoothstep(horizonMu - 0.008, horizonMu + 0.004, muS);
    let tSun = exp(-odSun) * shadow;
    let scatterR = betaR * rhoR;
    let scatterM = rhoM * betaMScatter;
    let single = tSun * (scatterR * phaseR + scatterM * phaseM);
    let multiple = (scatterR + scatterM) * vec3f(5.5000000000000004e-1f, 7.8000000000000003e-1f, 1.1499999999999999e+0f) * light.multipleScatter;
    radiance += tView * (single + multiple) * dt;
  }
  let aureole = smoothstep(-1.2000000000000000e-1f, 7.6000000000000001e-1f, cosTheta) * light.aureole;
  let clearSkyRaw = radiance * light.radiance + light.sunTransmittance * aureole;
  let clearSky = mix(vec3f(dot(clearSkyRaw, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B))), clearSkyRaw, light.skySaturation) * light.skyTint;
  let overcastGradient = 1.05 - max(dirIn.z, 0.0) * 0.22;
  let overcastSky = vec3f(1.0400000000000000e+0f, 1.0800000000000001e+0f, 1.1399999999999999e+0f) * overcastGradient * light.overcastScale;
  let result = mix(clearSky, overcastSky, light.overcast);
  let ground = smoothstep(0.0, 0.35, -dirIn.z);
  return mix(result, result * light.groundTint, ground);
}

// @block sky_sample
fn sample(ray: vec3f) -> vec3f {
    let dir = normalize(ray);
    // Below the horizon the view shows the horizon's haze, not the LUT's
    // ground-bounce hemisphere (that stays for the environment light only):
    // the backdrop land dissolves into it rather than into a khaki band.
    let shown = normalize(vec3f(dir.xy, max(dir.z, 0.004)));
    // Clouds pass in front of the sun's disc; the LUT stays clear-sky.
    return clouds(dir, textureSampleLevel(lutView, sampler_1, equirectUv(shown), 0.0).rgb + disc(dir));
  }
