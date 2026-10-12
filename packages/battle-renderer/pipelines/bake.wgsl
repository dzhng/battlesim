struct fullScreenTriangle_Output {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
}

@vertex fn fullScreenTriangle(@builtin(vertex_index) vertexIndex: u32) -> fullScreenTriangle_Output {
  const pos = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  const uv = array<vec2f, 3>(vec2f(0, 1), vec2f(2, 1), vec2f(0, -1));

  return fullScreenTriangle_Output(vec4f(pos[vertexIndex], 0, 1), uv[vertexIndex]);
}

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
}

@group(0) @binding(0) var<uniform> light: SkyLight;

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
    let multiple = (scatterR + scatterM) * vec3f(5.0000000000000000e-1f, 6.9999999999999996e-1f, 1.0000000000000000e+0f) * light.multipleScatter;
    radiance += tView * (single + multiple) * dt;
  }
  let aureole = smoothstep(-1.2000000000000000e-1f, 7.6000000000000001e-1f, cosTheta) * light.aureole;
  let clearSky = radiance * light.radiance + light.sunTransmittance * aureole;
  let overcastGradient = 1.05 - max(dirIn.z, 0.0) * 0.22;
  let overcastSky = vec3f(1.0200000000000000e+0f, 1.0500000000000000e+0f, 1.1000000000000001e+0f) * overcastGradient * light.overcastScale;
  let result = mix(clearSky, overcastSky, light.overcast);
  let ground = smoothstep(0.0, 0.35, -dirIn.z);
  return mix(result, result * light.groundTint, ground);
}

fn direction(uv: vec2f) -> vec3f {
  let theta = (uv.x - 0.5) * 6.2831853071795862e+0f;
  let phi = (uv.y - 0.5) * 3.1415926535897931e+0f;
  return vec3f(cos(phi) * cos(theta), sin(phi), cos(phi) * sin(theta));
}

fn bakeColor(uv: vec2f) -> vec4f {
    return max(vec4f(radiance(direction(uv)), 1.0), vec4f(0.0));
  }

struct fragment_Input {
  @location(0) uv: vec2f,
}

@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return bakeColor(_arg_0.uv);
}