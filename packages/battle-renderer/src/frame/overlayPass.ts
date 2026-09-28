// Display-space overlays: orders, contacts, tracers, rings and marks. They draw
// after post, depth-tested against the world without its grass (so a route
// lies over the blades, never speckled by them), into their own target (the
// units' x-ray already in it, over a transparent clear), and composite
// premultiplied over post's output. So they are
// never fogged, graded or tone mapped: their colours are the values their
// builders chose, shaded exactly as the old one-shader frame shaded them
// (landmine 13).
//
// Their glow is their own (slice 27e): never the world's bloom, which runs
// before them. The resolved overlay is blurred at half resolution (a
// separable Gaussian, rows then columns) and laid under the overlay as a
// premultiplied halo (the lab gives it `presentation.overlay.glow.ground`:
// the painted ground marks glow faintly, the DOM callouts by their own CSS):
//
//   overlay → glow rows (½ res) → glow columns (½ res) → composite: O + (1 − O.a)·halo
//
// The halo is a premultiplied layer with alpha ≤ 1, so the composite is still
// one premultiplied "over": the isolation check's algebra (final = B + (W −
// B)·world) holds with the halo inside B and W.
import { tgpu, d, std, common, type TgpuBuffer, type UniformFlag } from "typegpu";
import type { WorldMeshes } from "../scene";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import {
  identityInstance,
  meshAttribs,
  MeshSlot,
  WORLD_VARYING,
  type CameraGroup,
} from "./geometry";
import { FRAME_MSAA, OVERLAY_FORMAT, type FrameTargets } from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The flat light overlays have always had: one fixed key and an ambient. */
const OVERLAY_SUN = [0.5, -0.55, 0.67] as const;

const overlayFragment = tgpu.fragmentFn({
  in: { world: WORLD_VARYING, normal: d.vec3f, color: d.vec4f, highlight: d.f32 },
  out: d.vec4f,
})((v) => {
  "use gpu";
  const toEye = std.sub(typegpuCameraLayout.$.cam.eye, v.world);
  let n = std.normalize(v.normal);
  if (std.dot(n, toEye) < 0) {
    n = std.neg(n);
  }
  const key = std.normalize(d.vec3f(OVERLAY_SUN[0], OVERLAY_SUN[1], OVERLAY_SUN[2]));
  const light = std.max(std.dot(n, key), 0);
  const shaded = std.mul(v.color.xyz, 0.3 + 0.75 * light);
  const lit = std.add(shaded, std.mul(d.vec3f(0.95, 0.8, 0.2), v.highlight * 0.7));
  return d.vec4f(lit, v.color.w);
});

/** How far toward the eye, along its own view ray, an overlay mark is drawn:
 *  its pixel is unchanged and its depth clears the ground it lies on, so a
 *  mark at ground height (the orders, at the paint's height) is never cut
 *  by the ground it is draped over, though a hull, a wall or a ridge in
 *  front still hides it. */
const OVERLAY_PULL_M = 0.5;

/** The one mesh vertex stage, its clip position pulled `OVERLAY_PULL_M`
 *  toward the eye along the view ray (the world position, which lights the
 *  mark, is where it lies). */
const overlayVertex = tgpu.vertexFn({
  in: {
    position: d.vec3f,
    normal: d.vec3f,
    color: d.vec4f,
    placement: d.vec4f,
    tint: d.vec4f,
  },
  out: {
    clip: d.builtin.position,
    world: WORLD_VARYING,
    normal: d.vec3f,
    color: d.vec4f,
    highlight: d.f32,
  },
})((v) => {
  "use gpu";
  const c = std.cos(v.placement.w);
  const s = std.sin(v.placement.w);
  const world = d.vec3f(
    v.position.x * c - v.position.y * s + v.placement.x,
    v.position.x * s + v.position.y * c + v.placement.y,
    v.position.z + v.placement.z,
  );
  const toEye = std.normalize(std.sub(typegpuCameraLayout.$.cam.eye, world));
  const drawn = std.add(world, std.mul(toEye, OVERLAY_PULL_M));
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(drawn, 1)),
    world,
    normal: d.vec3f(v.normal.x * c - v.normal.y * s, v.normal.x * s + v.normal.y * c, v.normal.z),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    highlight: v.tint.w,
  };
});

/** The halo every overlay mark carries (the lab's `presentation.overlay.glow`:
 *  its `radius_px`, and `ground` as the strength). */
export interface OverlayGlowStyle {
  /** The halo's reach on screen (about 2.5 Gaussian sigmas), in device pixels. */
  radius_px: number;
  /** The halo's gain over the blurred overlay; 0 draws none. */
  strength: number;
}

/** The longest halo the blur loops reach, in device pixels. */
export const GLOW_MAX_RADIUS_PX = 24;

export function validateOverlayGlow(glow: OverlayGlowStyle): OverlayGlowStyle {
  if (
    !(glow.radius_px > 0 && glow.radius_px <= GLOW_MAX_RADIUS_PX) ||
    !(glow.strength >= 0 && glow.strength <= 8)
  )
    throw new Error(
      `overlay glow: radius_px in (0, ${GLOW_MAX_RADIUS_PX}], strength in [0, 8], got ${JSON.stringify(glow)}`,
    );
  return glow;
}

const GlowUniform = d
  .struct({ radiusPx: d.f32, sigmaPx: d.f32, strength: d.f32, pad: d.f32 })
  .$name("OverlayGlow");

function glowUniform(glow: OverlayGlowStyle) {
  return {
    radiusPx: glow.radius_px,
    sigmaPx: glow.radius_px / 2.5,
    strength: glow.strength,
    pad: 0,
  };
}

const glowRowsLayout = tgpu.bindGroupLayout({
  glow: { uniform: GlowUniform, visibility: ["fragment"] },
  overlay: { texture: d.texture2d(), visibility: ["fragment"] },
});
const glowColumnsLayout = tgpu.bindGroupLayout({
  glow: { uniform: GlowUniform, visibility: ["fragment"] },
  rows: { texture: d.texture2d(), visibility: ["fragment"] },
});
const overlaySource = tgpu.bindGroupLayout({
  glow: { uniform: GlowUniform, visibility: ["fragment"] },
  overlay: { texture: d.texture2d(), visibility: ["fragment"] },
  halo: { texture: d.texture2d(), visibility: ["fragment"] },
});

/** A half-resolution texel of the overlay, blurred along its row: each tap
 *  averages the 2×2 full-resolution block under it. */
const glowRows = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let g = glowRowsLayout.$.glow;
  let size = vec2i(textureDimensions(glowRowsLayout.$.overlay));
  let base = vec2i(pixel) * 2;
  let reach = i32(ceil(g.radiusPx * 0.5));
  var sum = vec4f(0.0);
  var total = 0.0;
  for (var k = -reach; k <= reach; k++) {
    let x = f32(k) * 2.0;
    let w = exp(-(x * x) / (2.0 * g.sigmaPx * g.sigmaPx));
    for (var dy = 0; dy < 2; dy++) {
      for (var dx = 0; dx < 2; dx++) {
        let p = clamp(base + vec2i(k * 2 + dx, dy), vec2i(0), size - 1);
        sum += w * textureLoad(glowRowsLayout.$.overlay, p, 0);
      }
    }
    total += 4.0 * w;
  }
  return sum / total;
}`)
  .$uses({ glowRowsLayout });

/** The rows' blur down its column: the halo at half resolution. */
const glowColumns = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let g = glowColumnsLayout.$.glow;
  let size = vec2i(textureDimensions(glowColumnsLayout.$.rows));
  let p = vec2i(pixel);
  let reach = i32(ceil(g.radiusPx * 0.5));
  var sum = vec4f(0.0);
  var total = 0.0;
  for (var k = -reach; k <= reach; k++) {
    let y = f32(k) * 2.0;
    let w = exp(-(y * y) / (2.0 * g.sigmaPx * g.sigmaPx));
    sum += w * textureLoad(glowColumnsLayout.$.rows, clamp(p + vec2i(0, k), vec2i(0), size - 1), 0);
    total += w;
  }
  return sum / total;
}`)
  .$uses({ glowColumnsLayout });

/** One overlay texel, already premultiplied by its resolve, over its halo:
 *  the half-resolution blur read bilinearly and scaled by the strength, its
 *  alpha held at or under 1 so it stays a premultiplied colour. */
const compositeOverlay = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let o = textureLoad(overlaySource.$.overlay, vec2i(pixel), 0);
  let g = overlaySource.$.glow;
  if (g.strength <= 0.0) { return o; }
  let top = vec2i(textureDimensions(overlaySource.$.halo)) - 1;
  let at = pixel * 0.5 - 0.5;
  let p0 = vec2i(floor(at));
  let f = at - floor(at);
  let a = textureLoad(overlaySource.$.halo, clamp(p0, vec2i(0), top), 0);
  let b = textureLoad(overlaySource.$.halo, clamp(p0 + vec2i(1, 0), vec2i(0), top), 0);
  let c = textureLoad(overlaySource.$.halo, clamp(p0 + vec2i(0, 1), vec2i(0), top), 0);
  let e = textureLoad(overlaySource.$.halo, clamp(p0 + vec2i(1, 1), vec2i(0), top), 0);
  let blur = mix(mix(a, b, f.x), mix(c, e, f.x), f.y);
  let gain = min(g.strength, 1.0 / max(blur.w, 1e-4));
  return o + (1.0 - o.w) * blur * gain;
}`)
  .$uses({ overlaySource });

// Drawn over a transparent clear, blending leaves colour premultiplied by
// alpha, which is what the resolve averages and the composite expects.
const premultiplying = {
  color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
  alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
} as const;
const over = {
  color: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
  alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
} as const;

const screen = (shade: typeof glowRows) =>
  tgpu.fragmentFn({ in: { pos: d.builtin.position }, out: d.vec4f })((v) => {
    "use gpu";
    return shade(v.pos.xy);
  });

export async function createOverlayPass(
  root: Root,
  registry: GpuRegistry,
  displayFormat: GPUTextureFormat,
  initialGlow: OverlayGlowStyle,
) {
  const base = {
    attribs: meshAttribs,
    vertex: overlayVertex,
    fragment: overlayFragment,
    primitive: { topology: "triangle-list", cullMode: "none" },
    targets: { format: OVERLAY_FORMAT, blend: premultiplying },
    multisample: { count: FRAME_MSAA },
  } as const;
  const opaque = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read-write"),
  });
  const translucent = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read"),
  });
  const composite = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(compositeOverlay),
    targets: { format: displayFormat, blend: over },
  });
  const rows = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(glowRows),
    targets: { format: OVERLAY_FORMAT },
  });
  const columns = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(glowColumns),
    targets: { format: OVERLAY_FORMAT },
  });
  await Promise.all([opaque, translucent, composite, rows, columns].map((p) => p.initAsync()));

  const identity = identityInstance(root, registry);
  const meshes = {
    opaque: new MeshSlot(root, registry, identity),
    translucent: new MeshSlot(root, registry, identity),
  };
  const glow = registry.own(root.createBuffer(GlowUniform).$usage("uniform"));
  let glowStyle = validateOverlayGlow(initialGlow);
  glow.write(glowUniform(glowStyle));

  return {
    set(next: WorldMeshes) {
      meshes.opaque.set(next.opaque);
      meshes.translucent.set(next.translucent);
    },
    /** The overlays' halo from the next frame on. */
    setGlow(next: OverlayGlowStyle) {
      glowStyle = validateOverlayGlow(next);
      glow.write(glowUniform(glowStyle));
    },
    /** The bind groups that read these targets' overlay and its halo. */
    sourceFor: (targets: FrameTargets) => bindOverlaySource(root, glow, targets),
    /** Draw the overlays against the world's depth, over the x-ray the
     *  prepass left in the target, blur their halo, then lay both over
     *  `output`. */
    encode(
      encoder: GPUCommandEncoder,
      targets: FrameTargets,
      source: OverlaySource,
      cameraGroup: CameraGroup,
      output: GPUTextureView,
    ) {
      const pass = encoder.beginRenderPass({
        label: "overlay",
        colorAttachments: [
          {
            view: targets.overlayMsaa.createView(),
            resolveTarget: targets.overlay.createView(),
            loadOp: "load",
            storeOp: "discard",
          },
        ],
        depthStencilAttachment: {
          view: targets.overlayDepth.createView(),
          depthLoadOp: "load",
          depthStoreOp: "discard",
        },
      });
      meshes.opaque.draw(opaque.with(pass).with(cameraGroup));
      meshes.translucent.draw(translucent.with(pass).with(cameraGroup));
      pass.end();
      if (glowStyle.strength > 0) {
        const clear = (target: GPUTexture) => ({
          view: target.createView(),
          loadOp: "clear" as const,
          storeOp: "store" as const,
          clearValue: [0, 0, 0, 0],
        });
        rows
          .with(encoder)
          .with(source.rows)
          .withColorAttachment(clear(targets.overlayGlowRows))
          .draw(3);
        columns
          .with(encoder)
          .with(source.columns)
          .withColorAttachment(clear(targets.overlayGlow))
          .draw(3);
      }
      composite
        .with(encoder)
        .with(source.composite)
        .withColorAttachment({ view: output, loadOp: "load", storeOp: "store" })
        .draw(3);
    },
    stats() {
      return { glowRadiusPx: glowStyle.radius_px, glowStrength: glowStyle.strength };
    },
  };
}
export type OverlayPass = Awaited<ReturnType<typeof createOverlayPass>>;

/** The bind groups that read one frame size's overlay and its halo. */
function bindOverlaySource(
  root: Root,
  glow: TgpuBuffer<typeof GlowUniform> & UniformFlag,
  targets: FrameTargets,
) {
  return {
    rows: root.createBindGroup(glowRowsLayout, { glow, overlay: targets.overlay.createView() }),
    columns: root.createBindGroup(glowColumnsLayout, {
      glow,
      rows: targets.overlayGlowRows.createView(),
    }),
    composite: root.createBindGroup(overlaySource, {
      glow,
      overlay: targets.overlay.createView(),
      halo: targets.overlayGlow.createView(),
    }),
  };
}
type OverlaySource = ReturnType<typeof bindOverlaySource>;
