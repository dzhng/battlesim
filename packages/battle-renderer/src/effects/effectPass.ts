// The effect pass: draws `EffectFrame`'s instances into the lit world, after
// the world pass has resolved it and before the fog mask pass gives unseen
// pixels their look, so effects take bloom, grade and tone map like any
// light. Three shapes (`SHAPE`), each a camera-facing quad:
//
// - a streak between two points (tracers, flash tongues, sparks), at least
//   `min_px` wide, dimmed rather than drawn thinner; or a smoke ribbon (a
//   round's trail) when its colour carries an opacity, lit as a flipbook's
//   smoke is, across its width as a tube;
// - a glow sprite (flashes, a round's head), or a solid disc (a round seen
//   as an object) when its colour carries an opacity;
// - a flipbook sprite from the effect atlas (fireballs, flames, smoke,
//   dust), premultiplied. Fire carries its own colour; smoke and dust carry
//   an albedo lit by the world's own light (the environment's uniform and
//   PMREM, `EffectLight`): each sprite shades as a soft ball, its sunward
//   side taking the sun (wrapped: light scatters through smoke), the sun
//   scattered forward when seen against it, and all of it the sky's light,
//   the sheet's own relief on top, so a column reads as volume, turns with
//   the sun, glows backlit and takes the sky's colour in its shade. The
//   world's cast lights (`light/castLights.ts`) light it too, as they light
//   the ground: a missile's motor its trail, a flash its smoke.
//
// Glows and light streaks add light; ribbons, discs and flipbooks blend
// over. Single-sampled into the resolved `lit` target, they test against the
// world's depth by reading it (sample 0), and fade into what they meet (soft particles). Beside the
// colour they lower the fog mask's unseen and seen coverage by their own
// strength, so a burst is shown as bright over unseen ground as over seen
// (the feed decides what is published there, not the fog).
//
// Raw WebGPU; the camera is the frame's one uniform (`world/camera.ts`).
import { EFFECT_FLOATS, type EffectBatch } from "./effectFrame";
import { CAST_FALLOFF, CastLights } from "../light/castLights";
import { tgpu } from "typegpu";
import { Camera } from "../world/camera";
import { Environment } from "../world/environment";
import { cubeUvWGSL } from "../shaders/pmrem";
import { FLIPBOOK_SIZE, FLIPBOOKS } from "./flipbooks";
import { FOG_MASK_FORMAT, HDR_FORMAT, type FrameTargets } from "../frame/targets";
import type { GpuRegistry, GpuSlot } from "../frame/registry";

const SHADER = /* wgsl */ `
@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var sceneDepth: texture_depth_multisampled_2d;
@group(0) @binding(2) var atlas: texture_2d_array<f32>;
@group(0) @binding(3) var linearSampler: sampler;
@group(0) @binding(4) var<uniform> light: Environment;
@group(0) @binding(5) var pmrem: texture_2d<f32>;
@group(0) @binding(6) var<uniform> casts: CastLights;
${cubeUvWGSL}

/** The cast lights' irradiance at \`p\` in smoke, over π: each one's falloff,
 *  with no facing (light scatters through smoke). */
fn castAt(p: vec3f) -> vec3f {
  var sum = vec3f(0.0);
  let count = u32(casts.header.x);
  for (var i = 0u; i < count; i++) {
    let toward = casts.lights[i].position.xyz - p;
    let x2 = dot(toward, toward) * casts.lights[i].position.w;
    if (x2 < 1.0) {
      sum += casts.lights[i].color.rgb * (1.0 - x2) * (1.0 - x2) / (1.0 + ${CAST_FALLOFF.toFixed(1)} * x2);
    }
  }
  return sum * 0.31830988;
}

struct In {
  @builtin(vertex_index) vi: u32,
  @location(0) a: vec4f,
  @location(1) b: vec4f,
  @location(2) color: vec4f,
  @location(3) misc: vec4f,
};
struct Out {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) color: vec4f,
  @location(2) @interpolate(flat) misc: vec4f,
  @location(3) @interpolate(flat) extra: vec4f,
  @location(4) along: f32,
  @location(5) viewDepth: f32,
  /** A lit sprite's sun in its own frame (right, up, toward the eye), and
   *  the sky's light on its top and on its side. */
  @location(6) @interpolate(flat) sun: vec3f,
  @location(8) @interpolate(flat) skyTop: vec3f,
  @location(9) @interpolate(flat) skySide: vec3f,
  /** Where on the sprite, screen-aligned, -1..1. */
  @location(7) offset: vec2f,
  /** A lit sprite's cast light (castAt), along a ribbon from end to end. */
  @location(10) castLit: vec3f,
};

const NEAR_W = 0.05;

/** The world's light on a lit sprite: the sun in the sprite's own frame
 *  (the camera's right, up, and toward the eye), and the sky's diffuse
 *  light (its PMREM at full roughness, times the fill, as world materials
 *  take it) on its top and on its side. */
struct SpriteLight {
  sun: vec3f,
  skyTop: vec3f,
  skySide: vec3f,
};
fn spriteLight() -> SpriteLight {
  // The camera's right and up are the view-projection's first two rows.
  let m = cam.viewProj;
  let right = normalize(vec3f(m[0].x, m[1].x, m[2].x));
  let up = normalize(vec3f(m[0].y, m[1].y, m[2].y));
  let toward = cross(right, up);
  let sun = light.sunDirection.xyz;
  var l: SpriteLight;
  l.sun = vec3f(dot(sun, right), dot(sun, up), dot(sun, toward));
  // The PMREM wants y flipped.
  let maxMip = light.settings.x;
  let side = normalize(vec3f(toward.x, toward.y, 0.0) + vec3f(0.0, 0.0, 1e-3));
  l.skyTop = samplePmrem(pmrem, linearSampler, vec3f(0.0, 0.0, 1.0), 1.0, maxMip) * light.fill.xyz;
  l.skySide = samplePmrem(pmrem, linearSampler, vec3f(side.x, -side.y, side.z), 1.0, maxMip)
    * light.fill.xyz;
  return l;
}

/** Clip-space units per metre at unit depth, across and up. */
fn projScale() -> vec2f {
  let m = cam.viewProj;
  return vec2f(length(vec3f(m[0].x, m[1].x, m[2].x)), length(vec3f(m[0].y, m[1].y, m[2].y)));
}

@vertex fn vs(v: In) -> Out {
  var corners = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
    vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0));
  let c = corners[v.vi];
  let half = vec2f(cam.width, cam.height) * 0.5;
  let s = projScale();
  var out: Out;
  out.uv = c;
  out.color = v.color;
  out.misc = v.misc;
  out.extra = vec4f(0.0);
  out.along = 0.0;
  out.viewDepth = 1.0;
  out.sun = vec3f(0.0, 0.0, 1.0);
  out.skyTop = vec3f(0.0);
  out.skySide = vec3f(0.0);
  out.castLit = vec3f(0.0);
  out.offset = c;
  let shape = u32(v.misc.x);
  if (shape == 0u) {
    var ca = cam.viewProj * vec4f(v.a.xyz, 1.0);
    var cb = cam.viewProj * vec4f(v.b.xyz, 1.0);
    if (ca.w < NEAR_W && cb.w < NEAR_W) {
      out.pos = vec4f(0.0, 0.0, -2.0, 1.0);
      return out;
    }
    var alongA = v.misc.y;
    var alongB = v.misc.z;
    if (ca.w < NEAR_W) {
      let t = (NEAR_W - ca.w) / (cb.w - ca.w);
      ca = mix(ca, cb, t);
      alongA = mix(alongA, alongB, t);
    }
    if (cb.w < NEAR_W) {
      let t = (NEAR_W - cb.w) / (ca.w - cb.w);
      cb = mix(cb, ca, t);
      alongB = mix(alongB, alongA, t);
    }
    let sa = ca.xy / ca.w * half;
    let sb = cb.xy / cb.w * half;
    let d = sb - sa;
    let len = length(d);
    let dir = select(vec2f(1.0, 0.0), d / max(len, 1e-6), len > 1e-4);
    let perp = vec2f(-dir.y, dir.x);
    let end = select(ca, cb, c.x > 0.0);
    let truePx = v.a.w * s.y * half.y / end.w;
    let px = max(truePx, v.b.w);
    let k = min(1.0, truePx / px);
    let ribbon = v.color.a > 0.0;
    // A light streak overhangs its ends by a quarter of its width and fades
    // over the overhang, so stretches laid end to end add up to one line; a
    // ribbon, blended over rather than added, is cut square instead.
    let overhang = select(0.25, 0.0, ribbon);
    let off = (perp * c.y + dir * c.x * overhang * 2.0) * px * 0.5;
    out.pos = vec4f(end.xy + off / half * end.w, end.zw);
    out.color = select(vec4f(v.color.rgb * k, 0.0), v.color * k, ribbon);
    out.along = select(alongA, alongB, c.x > 0.0);
    out.viewDepth = end.w;
    // Pixels from the quad's middle to each end, and the overhang; a
    // ribbon's soft-particle depth, half its width.
    out.extra = vec4f(len * 0.5 + px * overhang, px * overhang, v.a.w * 0.5, 0.0);
    if (ribbon) {
      out.offset = perp * c.y;
      let l = spriteLight();
      out.sun = l.sun;
      out.skyTop = l.skyTop;
      out.skySide = l.skySide;
      out.castLit = castAt(select(v.a.xyz, v.b.xyz, c.x > 0.0));
    }
    return out;
  }
  let clip = cam.viewProj * vec4f(v.a.xyz, 1.0);
  if (clip.w < NEAR_W) {
    out.pos = vec4f(0.0, 0.0, -2.0, 1.0);
    return out;
  }
  let cr = cos(v.b.x);
  let sr = sin(v.b.x);
  let turned = vec2f(c.x * cr - c.y * sr, c.x * sr + c.y * cr);
  let truePx = v.a.w * s.y * half.y / clip.w;
  let minPx = select(0.0, v.b.y, shape == 1u);
  let px = max(truePx, minPx);
  let radius = v.a.w * px / max(truePx, 1e-6);
  out.pos = vec4f(clip.xy + turned * radius * s, clip.zw);
  if (shape == 1u) {
    // A glow dims as the square of its shrink below the least size; a
    // solid disc (opacity in alpha) thins its coverage with it.
    let k = truePx / px;
    out.color = select(vec4f(v.color.rgb * k * k, 0.0), v.color * k, v.color.a > 0.0);
  }
  out.extra = vec4f(v.b.y, v.b.z, v.b.w, 0.0);
  out.viewDepth = clip.w;
  out.offset = turned;
  if (shape == 2u && v.misc.z > 0.5) {
    let l = spriteLight();
    out.sun = l.sun;
    out.skyTop = l.skyTop;
    out.skySide = l.skySide;
    out.castLit = castAt(v.a.xyz);
  }
  return out;
}

struct Frag {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
};

const LUMA = vec3f(0.2126, 0.7152, 0.0722);
const PI = 3.14159265;

/** The world's light on smoke at \`o\` on a lit sprite (-1..1, screen-
 *  aligned): a soft ball's (or, across a ribbon, a tube's) normal against the
 *  sun, wrapped (light scatters through smoke), and the sun scattered on
 *  toward the eye, strongest looking into it (Henyey-Greenstein, relative to
 *  isotropic): backlit dust and smoke glow rather than going dark. Plus the
 *  sky's light, more from above. */
fn smokeLight(f: Out, o: vec2f) -> vec3f {
  let n = normalize(vec3f(o, sqrt(max(0.0, 1.0 - dot(o, o))) + 0.3));
  let wrap = clamp((dot(n, f.sun) + 0.3) / 1.3, 0.0, 1.0);
  let g = 0.4;
  let forward = (1.0 - g * g) / pow(1.0 + g * g + 2.0 * g * f.sun.z, 1.5);
  let sky = mix(f.skySide, f.skyTop, clamp(n.y * 0.5 + 0.5, 0.0, 1.0));
  return light.sunRadiance.rgb * ((0.6 * wrap + 0.4 * forward) / PI) + sky + f.castLit;
}

fn cell(tuv: vec2f, frame: f32, cols: f32) -> vec2f {
  let at = vec2f(frame % cols, floor(frame / cols));
  return (at + tuv) / cols;
}

@fragment fn fs(f: Out) -> Frag {
  // Derivatives first, in uniform control flow.
  let tuv = clamp(vec2f(f.uv.x, -f.uv.y) * 0.5 + 0.5, vec2f(0.004), vec2f(0.996));
  let gx = dpdx(tuv);
  let gy = dpdy(tuv);
  let shape = u32(f.misc.x);
  let raw = textureLoad(sceneDepth, vec2i(f.pos.xy), 0);
  let scene = select(1e9, cam.znear / raw, raw > 0.0);
  let gap = scene - f.viewDepth;
  if (gap < 0.0) {
    discard;
  }
  var rgb = vec3f(0.0);
  var alpha = 0.0;
  if (shape == 0u && f.color.a > 0.0) {
    // A smoke ribbon: thick through its middle, thinning to its edges.
    let across = f.uv.y * f.uv.y;
    let cover = exp(-across * 2.0) * (1.0 - across)
      * clamp(gap / max(f.extra.z, 1e-3), 0.0, 1.0);
    alpha = f.color.a * cover;
    rgb = f.color.rgb * smokeLight(f, f.offset) * alpha;
  } else if (shape == 0u) {
    let across = f.uv.y * f.uv.y;
    let a = clamp(f.along, 0.0, 1.0);
    let fromEnd = (1.0 - abs(f.uv.x)) * f.extra.x;
    let cap = smoothstep(0.0, max(f.extra.y, 1e-3) * 2.0, fromEnd);
    // A sharp line with a little bloom of its own, or (misc.w) a soft glow
    // falling off across its whole width.
    let sharp = exp(-across * 9.0) + 0.3 * exp(-across * 2.5);
    let soft = exp(-across * 3.0) * (1.0 - across);
    let profile = select(sharp, soft, f.misc.w > 0.5);
    rgb = f.color.rgb * profile * a * a * cap * clamp(gap / 0.1, 0.0, 1.0);
  } else if (shape == 1u && f.color.a > 0.0) {
    // A solid disc: the round seen as an object.
    let cover = (1.0 - smoothstep(0.6, 1.0, length(f.uv))) * clamp(gap / 0.1, 0.0, 1.0);
    rgb = f.color.rgb * cover;
    alpha = f.color.a * cover;
  } else if (shape == 1u) {
    let r = length(f.uv);
    let ang = atan2(f.uv.y, f.uv.x);
    let rays = pow(abs(cos(ang * 2.0)), 16.0) * exp(-r * 2.5) * 0.6 * f.extra.y;
    let edge = 1.0 - smoothstep(0.75, 1.0, r);
    rgb = f.color.rgb * (exp(-r * r * 8.0) + rays) * edge * clamp(gap / 0.5, 0.0, 1.0);
  } else {
    let layer = i32(f.extra.y + 0.5);
    let cols = select(5.0, 8.0, layer == 1);
    let frames = cols * cols;
    let frame = clamp(f.extra.x, 0.0, frames - 1.0);
    let f0 = floor(frame);
    let f1 = min(f0 + 1.0, frames - 1.0);
    let g0 = gx / cols;
    let g1 = gy / cols;
    let s0 = textureSampleGrad(atlas, linearSampler, cell(tuv, f0, cols), layer, g0, g1);
    let s1 = textureSampleGrad(atlas, linearSampler, cell(tuv, f1, cols), layer, g0, g1);
    let tex = mix(s0, s1, frame - f0);
    let straight = tex.rgb / max(tex.a, 1e-3);
    let lum = dot(straight, LUMA);
    let opacity = f.color.a * clamp(gap / max(f.extra.z, 1e-3), 0.0, 1.0);
    if (f.misc.z > 0.5) {
      // Lit, the sheet's own relief on top.
      let relief = mix(0.65, 1.25, smoothstep(0.08, 0.45, lum));
      rgb = f.color.rgb * smokeLight(f, f.offset) * relief * tex.a * opacity;
    } else {
      let boost = 1.0 + f.misc.y * lum * lum;
      rgb = tex.rgb * f.color.rgb * boost * opacity;
    }
    alpha = tex.a * opacity;
  }
  let strength = clamp(alpha + max(rgb.r, max(rgb.g, rgb.b)) * 0.25, 0.0, 1.0);
  var out: Frag;
  out.color = vec4f(rgb, alpha);
  out.fog = vec4f(0.0, 0.0, 0.0, strength);
  return out;
}
`;

/** The atlas's mip chain: the full sheet down to one texel. */
const MIP_LEVELS = Math.log2(FLIPBOOK_SIZE) + 1;

/** Every flipbook into one layer of a mipmapped sRGB 2D array, premultiplied. */
async function loadAtlas(device: GPUDevice, registry: GpuRegistry): Promise<GPUTexture> {
  const texture = registry.texture({
    label: "effect-atlas",
    size: [FLIPBOOK_SIZE, FLIPBOOK_SIZE, FLIPBOOKS.length],
    format: "rgba8unorm-srgb",
    mipLevelCount: MIP_LEVELS,
    // RENDER_ATTACHMENT: copyExternalImageToTexture writes through it.
    usage:
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.COPY_DST |
      GPUTextureUsage.RENDER_ATTACHMENT,
  });
  await Promise.all(
    FLIPBOOKS.map(async (book, layer) => {
      const response = await fetch(book.url);
      if (!response.ok) throw new Error(`effect flipbook ${book.url}: HTTP ${response.status}`);
      const blob = await response.blob();
      const sheet = await createImageBitmap(blob, {
        premultiplyAlpha: "premultiply",
        colorSpaceConversion: "none",
      }).catch((error: unknown) => {
        // A worktree without `git lfs pull --include="assets/**"` serves pointers.
        throw new Error(`effect flipbook ${book.url} is not an image (an LFS pointer?): ${error}`);
      });
      if (sheet.width !== FLIPBOOK_SIZE || sheet.height !== FLIPBOOK_SIZE)
        throw new Error(
          `effect flipbook ${book.url} is ${sheet.width}×${sheet.height}, not ${FLIPBOOK_SIZE}²`,
        );
      for (let level = 0; level < MIP_LEVELS; level++) {
        const side = FLIPBOOK_SIZE >> level;
        const image =
          level === 0
            ? sheet
            : await createImageBitmap(sheet, {
                resizeWidth: side,
                resizeHeight: side,
                resizeQuality: "high",
                premultiplyAlpha: "premultiply",
                colorSpaceConversion: "none",
              });
        device.queue.copyExternalImageToTexture(
          { source: image },
          { texture, mipLevel: level, origin: [0, 0, layer], premultipliedAlpha: true },
          [side, side],
        );
        if (image !== sheet) image.close();
      }
      sheet.close();
    }),
  );
  return texture;
}

const INSTANCE_BYTES = EFFECT_FLOATS * 4;

/** The world's light, as raw resources: the environment uniform (sun,
 *  radiance, fill, PMREM mips), its PMREM atlas and the cast lights. */
export interface EffectLight {
  uniform: GPUBuffer;
  pmrem: GPUTexture;
  lights: GPUBuffer;
}

export async function createEffectPass(
  device: GPUDevice,
  registry: GpuRegistry,
  light: EffectLight,
) {
  const atlas = await loadAtlas(device, registry);
  const module = device.createShaderModule({
    label: "effects",
    code: tgpu.resolve({ template: SHADER, externals: { Camera, Environment, CastLights } }),
  });
  const layout = device.createBindGroupLayout({
    label: "effects",
    entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: {} },
      {
        binding: 1,
        visibility: GPUShaderStage.FRAGMENT,
        texture: { sampleType: "depth", multisampled: true },
      },
      {
        binding: 2,
        visibility: GPUShaderStage.FRAGMENT,
        texture: { sampleType: "float", viewDimension: "2d-array" },
      },
      {
        binding: 3,
        visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
        sampler: {},
      },
      { binding: 4, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: {} },
      { binding: 5, visibility: GPUShaderStage.VERTEX, texture: { sampleType: "float" } },
      { binding: 6, visibility: GPUShaderStage.VERTEX, buffer: {} },
    ],
  });
  const add = { srcFactor: "one", dstFactor: "one-minus-src-alpha" } as const;
  const keep = { srcFactor: "zero", dstFactor: "one" } as const;
  const pipeline = await device.createRenderPipelineAsync({
    label: "effects",
    layout: device.createPipelineLayout({ bindGroupLayouts: [layout] }),
    vertex: {
      module,
      entryPoint: "vs",
      buffers: [
        {
          arrayStride: INSTANCE_BYTES,
          stepMode: "instance",
          attributes: [0, 1, 2, 3].map((k) => ({
            shaderLocation: k,
            offset: k * 16,
            format: "float32x4" as const,
          })),
        },
      ],
    },
    fragment: {
      module,
      entryPoint: "fs",
      targets: [
        // Premultiplied over the lit world; its alpha kept.
        { format: HDR_FORMAT, blend: { color: add, alpha: keep } },
        // The fog mask's unseen and seen coverage, lowered by the effect's strength.
        {
          format: FOG_MASK_FORMAT,
          blend: { color: { srcFactor: "zero", dstFactor: "one-minus-src-alpha" }, alpha: keep },
          writeMask: GPUColorWrite.RED | GPUColorWrite.GREEN,
        },
      ],
    },
    primitive: { topology: "triangle-list" },
  });
  const sampler = device.createSampler({
    minFilter: "linear",
    magFilter: "linear",
    mipmapFilter: "linear",
  });
  const atlasView = atlas.createView({ dimension: "2d-array" });
  const pmremView = light.pmrem.createView();
  const instances: GpuSlot<GPUBuffer> = registry.slot();
  let capacity = 0;
  let count = 0;

  return {
    /** The bind group reading one frame size's depth. */
    groupFor(t: FrameTargets, camera: GPUBuffer): GPUBindGroup {
      return device.createBindGroup({
        label: "effects",
        layout,
        entries: [
          { binding: 0, resource: { buffer: camera } },
          { binding: 1, resource: t.depth.createView() },
          { binding: 2, resource: atlasView },
          { binding: 3, resource: sampler },
          { binding: 4, resource: { buffer: light.uniform } },
          { binding: 5, resource: pmremView },
          { binding: 6, resource: { buffer: light.lights } },
        ],
      });
    },
    /** This frame's instances, replacing the last. */
    set(batch: EffectBatch) {
      count = batch.count;
      if (count === 0) return;
      if (capacity < batch.data.length / EFFECT_FLOATS) {
        capacity = batch.data.length / EFFECT_FLOATS;
        instances.set(
          device.createBuffer({
            label: "effect-instances",
            size: capacity * INSTANCE_BYTES,
            usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
          }),
        );
      }
      device.queue.writeBuffer(instances.current!, 0, batch.data.buffer, 0, count * INSTANCE_BYTES);
    },
    /** After the world pass, before the fog mask pass. */
    encode(encoder: GPUCommandEncoder, t: FrameTargets, group: GPUBindGroup) {
      if (count === 0) return;
      const pass = encoder.beginRenderPass({
        label: "effects",
        colorAttachments: [
          { view: t.lit.createView(), loadOp: "load", storeOp: "store" },
          { view: t.fogMask.createView(), loadOp: "load", storeOp: "store" },
        ],
      });
      pass.setPipeline(pipeline);
      pass.setBindGroup(0, group);
      pass.setVertexBuffer(0, instances.current!);
      pass.draw(6, count);
      pass.end();
    },
    stats: () => ({ instances: count, capacity }),
  };
}
export type EffectPass = Awaited<ReturnType<typeof createEffectPass>>;
