// Stair-step amplitude of a fog boundary in a debug mask: per column, the
// row where white (seen, above) turns black (unseen, below) inside a band;
// fit a line; report residuals in pixels.
export function edgeMetric(raw, { x0 = 0, x1 = 1920, y0 = 0, y1 = 1080, W = 1920 } = {}) {
  const pts = [];
  for (let x = x0; x < x1; x++) {
    let prev = null;
    for (let y = y0; y < y1; y++) {
      const v = raw[(y * W + x) * 4] > 127;
      if (prev === true && v === false) { pts.push([x, y - 0.5]); break; }
      prev = v;
    }
  }
  const n = pts.length;
  if (n < 10) return { n, pts };
  let sx = 0, sy = 0, sxx = 0, sxy = 0;
  for (const [x, y] of pts) { sx += x; sy += y; sxx += x * x; sxy += x * y; }
  const a = (n * sxy - sx * sy) / (n * sxx - sx * sx), b = (sy - a * sx) / n;
  const r = pts.map(([x, y]) => y - (a * x + b));
  // Local deviation from a straight chord over a 64 px window: separates
  // stair-steps from the gentle curvature of a perspective-projected edge.
  const local = [];
  for (let i = 32; i + 32 < n; i++) {
    const [xa, ya] = pts[i - 32], [xb, yb] = pts[i + 32], [x, y] = pts[i];
    local.push(Math.abs(y - (ya + ((yb - ya) * (x - xa)) / (xb - xa))));
  }
  const max = (v) => v.reduce((m, q) => Math.max(m, Math.abs(q)), 0);
  const rms = (v) => Math.sqrt(v.reduce((m, q) => m + q * q, 0) / v.length);
  // Steps: runs of constant row longer than the line's slope implies.
  return { pts, n, slope: a, lineMaxPx: max(r), lineRmsPx: rms(r), localMaxPx: max(local), localRmsPx: rms(local) };
}
