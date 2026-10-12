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

fn disc(dir: vec3f) -> vec3f {
    let disc = smoothstep(9.9955000000000005e-1f, 9.9985000000000002e-1f, dot(dir, light.sunDirection))
      * smoothstep(-0.015, 0.01, dir.z) * light.discRadiance;
    return light.sunTransmittance * disc;
  }

fn hash(i: vec2i) -> f32 {
  var h = (bitcast<u32>(i.x) * 0x8da6b343u) ^ (bitcast<u32>(i.y) * 0xd8163841u);
  h = h ^ (h >> 13u);
  h = h * 0x5bd1e995u;
  h = h ^ (h >> 15u);
  return f32(h & 0xffffffu) / 16777216.0;
}

fn noise(p: vec2f) -> f32 {
  let i = vec2i(floor(p));
  let f = fract(p);
  let u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2i(1, 0)), u.x), mix(hash(i + vec2i(0, 1)), hash(i + vec2i(1, 1)), u.x), u.y);
}

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

@group(0) @binding(1) var lutView: texture_2d<f32>;

@group(0) @binding(2) var sampler_1: sampler;

fn equirectUv(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * 1.5915494309189535e-1f + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * 3.1830988618379069e-1f + 0.5);
}

fn sample(ray: vec3f) -> vec3f {
    let dir = normalize(ray);
    // Below the horizon the view shows the horizon's haze, not the LUT's
    // ground-bounce hemisphere (that stays for the environment light only):
    // the backdrop land dissolves into it rather than into a khaki band.
    let shown = normalize(vec3f(dir.xy, max(dir.z, 0.004)));
    // Clouds pass in front of the sun's disc; the LUT stays clear-sky.
    return clouds(dir, textureSampleLevel(lutView, sampler_1, equirectUv(shown), 0.0).rgb + disc(dir));
  }

struct Rays {
  origin: vec3f,
  dx: vec3f,
  dy: vec3f,
}

@group(0) @binding(3) var<uniform> rays: Rays;

fn backgroundColor(uv: vec2f) -> vec4f {
    return max(vec4f(sample(rays.origin + rays.dx * uv.x + rays.dy * uv.y), 1.0), vec4f(0.0));
  }

struct fragment_Input {
  @location(0) uv: vec2f,
}

@fragment fn fragment(_arg_0: fragment_Input) -> @location(0) vec4f {
  return backgroundColor(_arg_0.uv);
}