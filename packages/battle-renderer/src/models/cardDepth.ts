/** Downsample depth over covered samples, then premultiply by the same rounded
 * coverage as albedo. This prevents empty neighbour texels pulling surfaces
 * toward the back of the bake frame during filtering. */
export function filterCardDepth(
  source: Float32Array,
  albedo: Uint8Array,
  width: number,
  height: number,
  factor: number,
): Uint8Array {
  const w = width / factor,
    h = height / factor;
  const out = new Uint8Array(w * h * 2);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      let sum = 0,
        coverage = 0;
      for (let dy = 0; dy < factor; dy++)
        for (let dx = 0; dx < factor; dx++) {
          const i = (y * factor + dy) * width + x * factor + dx;
          const a = albedo[4 * i + 3];
          coverage += a;
          sum += source[i] * a;
        }
      const alpha = Math.round(coverage / (factor * factor)) / 255;
      const bits = coverage ? Math.round((sum / coverage) * alpha * 65535) : 0;
      const i = 2 * (y * w + x);
      out[i] = bits & 255;
      out[i + 1] = bits >>> 8;
    }
  return out;
}
