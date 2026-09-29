// The bounds of an icon drawing's geometry (its strokes' centre lines), so
// the generator can centre every icon on its ink rather than on the grid it
// was drawn on. A stroke adds the same width on every side, so the centre of
// these bounds is the centre of the ink.
//
// Reads the subset of SVG the generated icons use: `<path d>` (M L H V C S Q
// T A Z, absolute and relative), `<rect>` and `<circle>`. Curves and arcs are
// sampled finely enough for a 24-unit grid.

export type Bounds = [x0: number, y0: number, x1: number, y1: number];

/** Points sampled along each curve or arc. */
const SAMPLES = 64;

/** Every point on the drawing's centre lines worth bounding. */
function* points(body: string): Generator<[number, number]> {
  const num = (s: string) => Number(s);
  const attrs = (tag: string) =>
    Object.fromEntries([...tag.matchAll(/([\w-]+)="([^"]*)"/g)].map((m) => [m[1], m[2]]));
  for (const [tag, name] of body.matchAll(/<(path|rect|circle)\b[^>]*>/g)) {
    const a = attrs(tag);
    if (name === "rect") {
      const [x, y, w, h] = [a.x, a.y, a.width, a.height].map((v) => num(v ?? "0"));
      yield [x, y];
      yield [x + w, y + h];
    } else if (name === "circle") {
      const [cx, cy, r] = [a.cx, a.cy, a.r].map((v) => num(v ?? "0"));
      yield [cx - r, cy - r];
      yield [cx + r, cy + r];
    } else yield* pathPoints(a.d ?? "");
  }
}

function* pathPoints(d: string): Generator<[number, number]> {
  const tokens = [...d.matchAll(/[a-zA-Z]|-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?/g)].map((m) => m[0]);
  let i = 0;
  let cmd = "";
  let [x, y] = [0, 0];
  let [sx, sy] = [0, 0];
  // The last curve's second control point, for S and T.
  let [qx, qy] = [0, 0];
  const next = () => Number(tokens[i++]);
  while (i < tokens.length) {
    if (/[a-zA-Z]/.test(tokens[i])) cmd = tokens[i++];
    const rel = cmd === cmd.toLowerCase();
    const [ox, oy] = rel ? [x, y] : [0, 0];
    const C = cmd.toUpperCase();
    if (C === "Z") {
      [x, y] = [sx, sy];
      yield [x, y];
      continue;
    }
    if (C === "M" || C === "L" || C === "T") {
      const [px, py] = [ox + next(), oy + next()];
      if (C === "T") {
        const [cx, cy] = [2 * x - qx, 2 * y - qy];
        yield* quad(x, y, cx, cy, px, py);
        [qx, qy] = [cx, cy];
      } else [qx, qy] = [px, py];
      [x, y] = [px, py];
      if (C === "M") {
        [sx, sy] = [x, y];
        // Further pairs after a moveto are linetos.
        cmd = rel ? "l" : "L";
      }
      yield [x, y];
    } else if (C === "H") {
      x = (rel ? x : 0) + next();
      [qx, qy] = [x, y];
      yield [x, y];
    } else if (C === "V") {
      y = (rel ? y : 0) + next();
      [qx, qy] = [x, y];
      yield [x, y];
    } else if (C === "C" || C === "S") {
      const [c1x, c1y] = C === "C" ? [ox + next(), oy + next()] : [2 * x - qx, 2 * y - qy];
      const [c2x, c2y, px, py] = [ox + next(), oy + next(), ox + next(), oy + next()];
      for (let k = 0; k <= SAMPLES; k++) {
        const t = k / SAMPLES;
        const u = 1 - t;
        yield [
          u * u * u * x + 3 * u * u * t * c1x + 3 * u * t * t * c2x + t * t * t * px,
          u * u * u * y + 3 * u * u * t * c1y + 3 * u * t * t * c2y + t * t * t * py,
        ];
      }
      [qx, qy, x, y] = [c2x, c2y, px, py];
    } else if (C === "Q") {
      const [cx, cy, px, py] = [ox + next(), oy + next(), ox + next(), oy + next()];
      yield* quad(x, y, cx, cy, px, py);
      [qx, qy, x, y] = [cx, cy, px, py];
    } else if (C === "A") {
      const [rx, ry, rot, large, sweep] = [next(), next(), next(), next(), next()];
      const [px, py] = [ox + next(), oy + next()];
      yield* arc(x, y, rx, ry, rot, large !== 0, sweep !== 0, px, py);
      [x, y, qx, qy] = [px, py, px, py];
    } else throw new Error(`inkBounds: path command "${cmd}" is not read`);
  }
}

function* quad(x0: number, y0: number, cx: number, cy: number, x1: number, y1: number) {
  for (let k = 0; k <= SAMPLES; k++) {
    const t = k / SAMPLES;
    const u = 1 - t;
    yield [u * u * x0 + 2 * u * t * cx + t * t * x1, u * u * y0 + 2 * u * t * cy + t * t * y1] as [
      number,
      number,
    ];
  }
}

/** An SVG elliptical arc's points, by its centre parameterisation (SVG 1.1
 *  appendix F.6.5). */
function* arc(
  x1: number,
  y1: number,
  rx: number,
  ry: number,
  rotDeg: number,
  large: boolean,
  sweep: boolean,
  x2: number,
  y2: number,
): Generator<[number, number]> {
  if (rx === 0 || ry === 0) {
    yield [x2, y2];
    return;
  }
  const phi = (rotDeg * Math.PI) / 180;
  const [cos, sin] = [Math.cos(phi), Math.sin(phi)];
  const [dx, dy] = [(x1 - x2) / 2, (y1 - y2) / 2];
  const x1p = cos * dx + sin * dy;
  const y1p = -sin * dx + cos * dy;
  rx = Math.abs(rx);
  ry = Math.abs(ry);
  // Radii too small for the chord grow until they fit.
  const lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
  if (lambda > 1) [rx, ry] = [rx * Math.sqrt(lambda), ry * Math.sqrt(lambda)];
  const num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
  const den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
  const co = (large === sweep ? -1 : 1) * Math.sqrt(Math.max(0, num / den));
  const cxp = (co * rx * y1p) / ry;
  const cyp = (-co * ry * x1p) / rx;
  const cx = cos * cxp - sin * cyp + (x1 + x2) / 2;
  const cy = sin * cxp + cos * cyp + (y1 + y2) / 2;
  const angle = (ux: number, uy: number, vx: number, vy: number) =>
    Math.atan2(ux * vy - uy * vx, ux * vx + uy * vy);
  const t1 = angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry);
  let dt = angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
  if (!sweep && dt > 0) dt -= 2 * Math.PI;
  if (sweep && dt < 0) dt += 2 * Math.PI;
  for (let k = 0; k <= SAMPLES; k++) {
    const t = t1 + (dt * k) / SAMPLES;
    const [ex, ey] = [rx * Math.cos(t), ry * Math.sin(t)];
    yield [cos * ex - sin * ey + cx, sin * ex + cos * ey + cy];
  }
}

/** The drawing's bounds. */
export function inkBounds(body: string): Bounds {
  const b: Bounds = [Infinity, Infinity, -Infinity, -Infinity];
  for (const [x, y] of points(body)) {
    b[0] = Math.min(b[0], x);
    b[1] = Math.min(b[1], y);
    b[2] = Math.max(b[2], x);
    b[3] = Math.max(b[3], y);
  }
  if (!Number.isFinite(b[0])) throw new Error("inkBounds: the drawing has no geometry");
  return b;
}
