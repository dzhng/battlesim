// The fog mask pass: the one owner of how unseen looks on screen (slice 15b).
// Every world material writes, beside its lit colour, a fog mask sample:
// (unseen, seen, ground) coverage from FogTerm, or nothing where fog never
// applies (units, the sky, the backdrop). The 4× MSAA resolve turns the samples into
// per-pixel fractions. Then, in screen space:
//
//   distance rows → distance columns → composite (HDR, before post)
//                                   ↘ rim (display space, after post)
//
// - The distance passes find, within `reachPx`, each ground pixel's
//   Euclidean distance to the nearest unseen and the nearest seen ground
//   pixel (a separable, bounded distance transform: the nearest along each
//   row, then the least `row² + dy²` down each column). The edge is where
//   sight ends across the ground, which is what a sight shadow is and what
//   was read as a cast shadow. A face (a wall, a roof, a tree) is seen or
//   unseen whole and keeps a hard edge; one surface standing in front of
//   another (a seen roof against unseen ground behind it, unseen trunks
//   against a seen field) makes no edge; nor do pixels fog never covers (a
//   unit, the sky, the map's edge).
// - The composite gives unseen pixels the frame's FogStyle through `fogLook`,
//   weighted by their unseen coverage and, within `edge_softness` pixels of
//   the seen side, faded in: the unseen side of the edge softens and seen
//   pixels are returned exactly as lit.
// - The rim draws a line `rim.width_px` wide along the edge on the seen side,
//   in display colour after post (as overlays are), so it is exactly the
//   style's colour, never graded or bloomed. A sun shadow's edge has no line.
//
// The style (`presentation.fog.styles.<name>`) is live via `setStyle`; the
// lab's mask view shows the resolved mask itself, white where seen.
import { tgpu, d, common, type TgpuBuffer, type UniformFlag } from "typegpu";
import {
  FOG_DISTANCE_CAP_PX,
  FogStyleUniform,
  fogLook,
  fogStyleUniform,
  type FogStyle,
} from "./fogStyle";
import { FOG_DISTANCE_FORMAT, HDR_FORMAT, type FrameTargets } from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The mask view's values in HDR: white after post for pixels with no unseen
 *  coverage, mid-grey for partly unseen ones. */
const MASK_SEEN = 16;
const MASK_PART = "0.35";

const FogMaskView = d.struct({ mask: d.u32 }).$name("FogMaskView");

const rowsLayout = tgpu.bindGroupLayout({
  look: { uniform: FogStyleUniform, visibility: ["fragment"] },
  mask: { texture: d.texture2d(), visibility: ["fragment"] },
});

const columnsLayout = tgpu.bindGroupLayout({
  look: { uniform: FogStyleUniform, visibility: ["fragment"] },
  mask: { texture: d.texture2d(), visibility: ["fragment"] },
  rows: { texture: d.texture2d(), visibility: ["fragment"] },
});

const composeLayout = tgpu.bindGroupLayout({
  look: { uniform: FogStyleUniform, visibility: ["fragment"] },
  view: { uniform: FogMaskView, visibility: ["fragment"] },
  lit: { texture: d.texture2d(), visibility: ["fragment"] },
  mask: { texture: d.texture2d(), visibility: ["fragment"] },
  distance: { texture: d.texture2d(), visibility: ["fragment"] },
});

const CAP = `${FOG_DISTANCE_CAP_PX.toFixed(1)}`;

/**
 * A resolved mask texel's side of the ground's edge: `(unseen, seen)`, each
 * 1 when more than half the pixel's samples are ground fog covers and none of
 * its samples is on the other side. A pixel that mixes sides (the edge's own
 * anti-aliased pixel, or a silhouette pixel where an unseen wall stands over
 * seen ground) is neither, so only pixels wholly on one side make an edge.
 */
const groundSide = tgpu.fn(
  [d.vec4f],
  d.vec2f,
)(/* wgsl */ `(m: vec4f) -> vec2f {
  if (m.z <= 0.5) { return vec2f(0.0); }
  return vec2f(f32(m.x > 0.5 && m.y <= 0.0), f32(m.y > 0.5 && m.x <= 0.0));
}`);

/** Per pixel, the distance along its row to the nearest wholly unseen and
 *  wholly seen ground pixel within reach (`CAP` for none), over `CAP`. */
const distanceRows = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let width = i32(textureDimensions(rowsLayout.$.mask).x);
  let reach = i32(rowsLayout.$.look.reachPx);
  var best = vec2f(${CAP});
  for (var k = -reach; k <= reach; k++) {
    let x = p.x + k;
    if (x < 0 || x >= width) { continue; }
    let side = groundSide(textureLoad(rowsLayout.$.mask, vec2i(x, p.y), 0));
    best = min(best, select(vec2f(${CAP}), vec2f(f32(abs(k))), side > vec2f(0.5)));
  }
  return vec4f(best / ${CAP}, 0.0, 1.0);
}`)
  .$uses({ rowsLayout, groundSide });

/** Per pixel that is mostly ground, the Euclidean distance to the nearest
 *  wholly unseen and wholly seen ground pixel within reach, from the rows'
 *  distances down its column, over `CAP`; every other pixel is `CAP` from
 *  both. */
const distanceColumns = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  if (textureLoad(columnsLayout.$.mask, p, 0).z <= 0.5) { return vec4f(1.0, 1.0, 0.0, 1.0); }
  let height = i32(textureDimensions(columnsLayout.$.rows).y);
  let reach = i32(columnsLayout.$.look.reachPx);
  var best = vec2f(${CAP} * ${CAP});
  for (var k = -reach; k <= reach; k++) {
    let y = p.y + k;
    if (y < 0 || y >= height) { continue; }
    let row = textureLoad(columnsLayout.$.rows, vec2i(p.x, y), 0).xy * ${CAP};
    best = min(best, row * row + vec2f(f32(k * k)));
  }
  return vec4f(min(sqrt(best), vec2f(${CAP})) / ${CAP}, 0.0, 1.0);
}`)
  .$uses({ columnsLayout });

/** The world with fog: seen pixels exactly as lit; unseen ones take the
 *  style by their coverage, faded in over `edge_softness` from the seen side. */
const composeFog = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let lit = textureLoad(composeLayout.$.lit, p, 0);
  let m = textureLoad(composeLayout.$.mask, p, 0).xy;
  if (composeLayout.$.view.mask == 1u) {
    // Unseen (more than half the pixel's samples) black; untouched (none)
    // white; partly unseen grey.
    let v = select(select(${MASK_PART}, ${MASK_SEEN}.0, m.x <= 0.0), 0.0, m.x > 0.5);
    return vec4f(vec3f(v), 1.0);
  }
  if (m.x <= 0.0) { return lit; }
  let s = composeLayout.$.look;
  var unseen = m.x;
  if (s.edgeSoftnessPx > 0.0) {
    let toSeen = textureLoad(composeLayout.$.distance, p, 0).y * ${CAP};
    unseen *= smoothstep(0.0, 1.0, (toSeen - 0.5) / s.edgeSoftnessPx);
  }
  return vec4f(mix(lit.xyz, fogLook(lit.xyz, pixel, s), unseen), lit.w);
}`)
  .$uses({ composeLayout, fogLook });

/** The rim over post's output: the style's colour on seen pixels within
 *  `rim.width_px` of an unseen one (plus the edge pixel's own half), weighted
 *  by the pixel's seen coverage; nothing anywhere else. */
const rimAt = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let s = composeLayout.$.look;
  let m = textureLoad(composeLayout.$.mask, p, 0).xy;
  let seen = m.y;
  if (seen <= 0.0 || m.x > 0.5 || s.rimAlpha <= 0.0 || s.rimWidthPx <= 0.0) {
    return vec4f(0.0);
  }
  let toUnseen = textureLoad(composeLayout.$.distance, p, 0).x * ${CAP};
  let cover = clamp(s.rimWidthPx + 1.0 - toUnseen, 0.0, 1.0);
  return vec4f(s.rimColor, s.rimAlpha * cover * seen);
}`)
  .$uses({ composeLayout });

const screen = (shade: typeof distanceRows) =>
  tgpu.fragmentFn({ in: { pos: d.builtin.position }, out: d.vec4f })((v) => {
    "use gpu";
    return shade(v.pos.xy);
  });

/** Straight alpha over post's output, its alpha kept. */
const over = {
  color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
  alpha: { srcFactor: "zero", dstFactor: "one" },
} as const;

interface FogMaskBuffers {
  look: TgpuBuffer<typeof FogStyleUniform> & UniformFlag;
  view: TgpuBuffer<typeof FogMaskView> & UniformFlag;
}

/** The bind groups that read one frame size's targets. */
function bindFogMask(root: Root, { look, view }: FogMaskBuffers, t: FrameTargets) {
  return {
    rows: root.createBindGroup(rowsLayout, { look, mask: t.fogMask.createView() }),
    columns: root.createBindGroup(columnsLayout, {
      look,
      mask: t.fogMask.createView(),
      rows: t.fogDistanceRows.createView(),
    }),
    compose: root.createBindGroup(composeLayout, {
      look,
      view,
      lit: t.lit.createView(),
      mask: t.fogMask.createView(),
      distance: t.fogDistance.createView(),
    }),
  };
}
export type FogMaskGroups = ReturnType<typeof bindFogMask>;

export async function createFogMaskPass(
  root: Root,
  registry: GpuRegistry,
  displayFormat: GPUTextureFormat,
  initialStyle: FogStyle,
) {
  const rows = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(distanceRows),
    targets: { format: FOG_DISTANCE_FORMAT },
  });
  const columns = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(distanceColumns),
    targets: { format: FOG_DISTANCE_FORMAT },
  });
  const compose = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(composeFog),
    targets: { format: HDR_FORMAT },
  });
  const rim = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: screen(rimAt),
    targets: { format: displayFormat, blend: over },
  });
  await Promise.all([rows, columns, compose, rim].map((p) => p.initAsync()));

  const look = registry.own(root.createBuffer(FogStyleUniform).$usage("uniform"));
  const view = registry.own(root.createBuffer(FogMaskView).$usage("uniform"));
  let style = initialStyle;
  let maskView = false;
  look.write(fogStyleUniform(style));
  view.write({ mask: 0 });
  /** Whether this style draws anything the distances are needed for. */
  const edges = () => style.edge_softness > 0 || (style.rim.width_px > 0 && style.rim.alpha > 0);

  return {
    /** How unseen looks, and its edge, from the next frame on. */
    setStyle(next: FogStyle) {
      style = next;
      look.write(fogStyleUniform(next));
    },
    /** Show the resolved mask (white seen, black unseen) instead of the look. */
    setMaskView(on: boolean) {
      maskView = on;
      view.write({ mask: on ? 1 : 0 });
    },
    /** The bind groups that read a frame size's targets. */
    groupsFor: (t: FrameTargets): FogMaskGroups => bindFogMask(root, { look, view }, t),
    /** After the world: the distances (when the style has an edge to draw),
     *  then the fogged world into `hdr`, post's input. */
    encode(encoder: GPUCommandEncoder, t: FrameTargets, groups: FogMaskGroups) {
      if (edges() && !maskView) {
        const pass = (target: GPUTexture) => ({
          view: target.createView(),
          loadOp: "clear" as const,
          storeOp: "store" as const,
          clearValue: [1, 1, 1, 1],
        });
        rows.with(encoder).with(groups.rows).withColorAttachment(pass(t.fogDistanceRows)).draw(3);
        columns.with(encoder).with(groups.columns).withColorAttachment(pass(t.fogDistance)).draw(3);
      }
      compose
        .with(encoder)
        .with(groups.compose)
        .withColorAttachment({ view: t.hdr.createView(), loadOp: "clear", storeOp: "store" })
        .draw(3);
    },
    /** After post: the rim over `output`. */
    encodeRim(encoder: GPUCommandEncoder, groups: FogMaskGroups, output: GPUTextureView) {
      if (maskView || style.rim.width_px <= 0 || style.rim.alpha <= 0) return;
      rim
        .with(encoder)
        .with(groups.compose)
        .withColorAttachment({ view: output, loadOp: "load", storeOp: "store" })
        .draw(3);
    },
    stats() {
      return {
        edgeSoftnessPx: style.edge_softness,
        rimWidthPx: style.rim.width_px,
        reachPx: edges() ? fogStyleUniform(style).reachPx : 0,
        maskView,
      };
    },
  };
}
export type FogMaskPass = Awaited<ReturnType<typeof createFogMaskPass>>;
