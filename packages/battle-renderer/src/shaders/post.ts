// Adapted from ~/dev/game battle-renderer/src/shaders/post.ts (reuse
// manifest). Local changes: the grade's tints and the bloom's numbers come from
// `presentation.light` (the grade uniform and the bloom WGSL builders) instead
// of module constants, and the per-preset grade strength is gone.
import type { BloomSettings } from "../light/sceneLight";

/** Rec. 709 luma, for the grade and the bloom threshold. */
export const GRADE_LUMA = [0.2126, 0.7152, 0.0722] as const;

/** AgX/OETF constants follow pinned Three 0.185.1 (MIT, see LICENSE.three). */
export const gradeColorWgsl = `(input: vec3f, params: Grade) -> vec3f {
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
}`;
export const agxWgsl = `(color: vec3f, exposure: f32) -> vec3f {
  let to2020 = mat3x3f(vec3f(0.6274,0.0691,0.0164),vec3f(0.3293,0.9195,0.0880),vec3f(0.0433,0.0113,0.8956));
  let inset = mat3x3f(vec3f(0.856627153315983,0.137318972929847,0.11189821299995),vec3f(0.0951212405381588,0.761241990602591,0.0767994186031903),vec3f(0.0482516061458583,0.101439036467562,0.811302368396859));
  let outset = mat3x3f(vec3f(1.1271005818144368,-0.1413297634984383,-0.14132976349843826),vec3f(-0.11060664309660323,1.157823702216272,-0.11060664309660294),vec3f(-0.016493938717834573,-0.016493938717834257,1.2519364065950405));
  let toSrgb = mat3x3f(vec3f(1.6605,-0.1246,-0.0182),vec3f(-0.5876,1.1329,-0.1006),vec3f(-0.0728,-0.0083,1.1187));
  let x = clamp((log2(max(inset*(to2020*(color*exposure)),vec3f(1e-10)))+12.47393)/(4.026069+12.47393),vec3f(0),vec3f(1));
  let x2 = x*x;
  let x4 = x2*x2;
  let contrast = 15.5*(x4*x2)-40.14*(x4*x)+(31.96*x4-6.868*(x2*x)+(0.4298*x2+(0.1191*x-0.00232)));
  return clamp(toSrgb*pow(max(vec3f(0),outset*contrast),vec3f(2.2)),vec3f(0),vec3f(1));
}`;
export const outputSrgbWgsl = `(linear: vec3f) -> vec3f {
  return select(pow(linear,vec3f(0.41666))*1.055-0.055,linear*12.92,linear<=vec3f(0.0031308));
}`;

export const BLOOM_KERNEL_RADII = [6, 10, 14, 18, 22] as const;

/** Pure shader functions; each runtime supplies the sampled resources and uniforms. */
export function bloomHighpassWgsl(bloom: BloomSettings): string {
  return `(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler) -> vec4f {
  let c = textureSample(source,linearSampler,uv);
  return mix(vec4f(0),c,smoothstep(${bloom.threshold},${bloom.threshold + bloom.smooth_width},dot(c.rgb,vec3f(${GRADE_LUMA.join(",")}))));
}`;
}

export function bloomBlurWgsl(radius: number, width: number, height: number, axis: 0 | 1) {
  const sigma = radius / 3;
  const weights = Array.from(
    { length: radius },
    (_, i) => (0.39894 * Math.exp((-0.5 * i * i) / (sigma * sigma))) / sigma,
  );
  const delta = axis === 0 ? `vec2f(${1 / width},0)` : `vec2f(0,${1 / height})`;
  return `(uv: vec2f, source: texture_2d<f32>, linearSampler: sampler) -> vec4f {
    let weights = array<f32,${radius}>(${weights.join(",")});
    var color = textureSample(source,linearSampler,uv).rgb*weights[0];
    for(var i = 1; i < ${radius}; i++) {
      let offset = ${delta}*f32(i);
      color += (textureSample(source,linearSampler,uv+offset).rgb+textureSample(source,linearSampler,uv-offset).rgb)*weights[i];
    }
    return vec4f(color,1);
  }`;
}

export function bloomCompositeWgsl(bloom: BloomSettings): string {
  return `(uv: vec2f, level0: texture_2d<f32>, level1: texture_2d<f32>, level2: texture_2d<f32>, level3: texture_2d<f32>, level4: texture_2d<f32>, linearSampler: sampler) -> vec4f {
  return (${[1, 0.8, 0.6, 0.4, 0.2].map((f, i) => `textureSample(level${i},linearSampler,uv)*${f * (1 - bloom.radius) + (1.2 - f) * bloom.radius}`).join("+")})*${bloom.strength};
}`;
}

export const postFinalWgsl = `(uv: vec2f, scene: texture_2d<f32>, bloom: texture_2d<f32>, linearSampler: sampler, grade: Grade) -> vec4f {
  let hdr = textureSample(scene,linearSampler,uv)+textureSample(bloom,linearSampler,uv);
  return vec4f(outputSrgb(agx(gradeColor(hdr.rgb,grade),grade.exposure)),clamp(hdr.a,0.0,1.0));
}`;

/** Pinned Three RenderOutputNode alpha ordering, without authored grade or bloom.
 * MIT attribution: LICENSE.three in this directory. */
export const postDirectWgsl = `(hdr:vec4f, exposure:f32)->vec4f {
  let alpha = clamp(hdr.a, 0.0, 1.0);
  var straight = vec3f(0);
  if (alpha > 0.0) { straight = hdr.rgb / alpha; }
  return vec4f(outputSrgb(agx(straight, exposure)) * alpha, alpha);
}`;
