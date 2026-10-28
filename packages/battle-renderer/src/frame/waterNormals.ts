// The water's normal map, made once at load by an exported bake
// (`waterNormalsBake`, decision 16): no image ships. It tiles: every wave
// it sums turns a whole number of times across the tile, so the water
// samples it at any scale without a seam. Each texel holds the surface's
// slope along the tile's two axes, at most about one; the water turns that
// into a normal at its own strength (`waterNormal`).
import { tgpu, d, common, std } from "typegpu";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** Texels a side. */
export const WATER_NORMALS_SIZE = 256;
/** Waves summed. */
const WAVES = 24;
/** The format the slopes are kept in (the frame's HDR format). */
const FORMAT = "rgba16float" as const;

/** The waves: whole cycles across the tile along each axis, spread round
 *  the compass by the golden angle and out from two cycles to about twelve,
 *  each its height (shorter waves lower, so no band of them owns the slope)
 *  and its phase. A formula, not tuning: the look's strength and scales are
 *  the water's own. */
const waves = Array.from({ length: WAVES }, (_, i) => {
  const angle = i * 2.399963;
  const cycles = 2 + i * 0.45;
  const m = Math.round(cycles * Math.cos(angle));
  const n = Math.round(cycles * Math.sin(angle));
  const k = Math.hypot(m, n) || 1;
  return { m, n, height: 1 / k ** 1.5, phase: ((i * 0.618034) % 1) * 2 * Math.PI };
});
/** The finest wave's cycles across the tile: what a pixel must resolve for
 *  the map's detail to show. */
export const WATER_NORMALS_FINEST = Math.max(...waves.map((w) => Math.hypot(w.m, w.n)));
/** The steepest a texel can lean, so the slopes stay about one at most. */
const STEEPEST = waves.reduce((sum, w) => sum + w.height * 2 * Math.PI * Math.hypot(w.m, w.n), 0);
const terms = waves
  .map(
    ({ m, n, height, phase }) =>
      `  { let c = cos(${2 * Math.PI}*(${m}.0*uv.x + ${n}.0*uv.y) + ${phase});\n` +
      `    slope += vec2f(${m}.0, ${n}.0) * (${(height * 2 * Math.PI) / STEEPEST} * c); }`,
  )
  .join("\n");

/** The slope of the summed waves at tile position `uv`. */
const waterNormalsSlope = tgpu.fn(
  [d.vec2f],
  d.vec4f,
)(/* wgsl */ `(uv: vec2f) -> vec4f {
  var slope = vec2f(0.0);
${terms}
  return vec4f(slope, 0.0, 1.0);
}`);

/** The baked map and the sampler that wraps it; baked when made. */
export function createWaterNormals(root: Root, registry: GpuRegistry) {
  const texture = registry.texture({
    label: "water-normals",
    size: [WATER_NORMALS_SIZE, WATER_NORMALS_SIZE],
    format: FORMAT,
    usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
  });
  const sampler = registry.device.createSampler({
    label: "water-normals",
    addressModeU: "repeat",
    addressModeV: "repeat",
    magFilter: "linear",
    minFilter: "linear",
  });
  const bake = root
    .createRenderPipeline({
      vertex: common.fullScreenTriangle,
      fragment: tgpu.fragmentFn({ in: { uv: d.vec2f }, out: d.vec4f })(({ uv }) => {
        "use gpu";
        return waterNormalsSlope(std.fract(uv));
      }),
      targets: { format: FORMAT },
    })
    .$name("waterNormalsBake");
  const encoder = registry.device.createCommandEncoder({ label: "water-normals-bake" });
  bake
    .with(encoder)
    .withColorAttachment({ view: texture.createView(), clearValue: [0, 0, 0, 1] })
    .draw(3);
  registry.device.queue.submit([encoder.finish()]);
  return { view: texture.createView(), sampler };
}
