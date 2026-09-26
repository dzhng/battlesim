// Position error of a rendered fog edge against the true sight-shadow line
// (the vertical plane through the eye and a building corner), in pixels.
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const norm = (a) => { const l = Math.hypot(...a); return [a[0] / l, a[1] / l, a[2] / l]; };
export function project(cam, p, W = 1920, H = 1080) {
  const z = norm(sub(cam.eye, cam.target)), x = norm(cross([0, 0, 1], z)), y = cross(z, x);
  const d = sub(p, cam.eye);
  const vx = dot(x, d), vy = dot(y, d), vz = dot(z, d);
  const f = 1 / Math.tan((cam.fov * Math.PI) / 360);
  const nx = ((f / (W / H)) * vx) / -vz, ny = (f * vy) / -vz;
  return [(nx * 0.5 + 0.5) * W, (0.5 - ny * 0.5) * H];
}
/** `edge`: [[x, y]] rendered transition points; eye/corner xy; ground z (flat). */
export function positionError(edge, cam, eye, corner, groundZ) {
  const pts = [];
  for (let t = 1; t < 40; t += 0.002) {
    const p = [eye[0] + (corner[0] - eye[0]) * t, eye[1] + (corner[1] - eye[1]) * t, groundZ];
    const s = project(cam, p);
    if (s[0] >= 0 && s[0] < 1920 && s[1] >= 0 && s[1] < 1080) pts.push(s);
  }
  pts.sort((a, b) => a[0] - b[0]);
  const yAt = (x) => {
    let lo = 0, hi = pts.length - 1;
    while (hi - lo > 1) { const m = (lo + hi) >> 1; if (pts[m][0] < x) lo = m; else hi = m; }
    const [x0, y0] = pts[lo], [x1, y1] = pts[hi];
    return y0 + ((y1 - y0) * (x - x0)) / (x1 - x0 || 1);
  };
  const err = edge.filter(([x]) => x >= pts[0][0] && x <= pts[pts.length - 1][0]).map(([x, y]) => y - yAt(x));
  const max = err.reduce((m, e) => Math.max(m, Math.abs(e)), 0);
  const mean = err.reduce((m, e) => m + Math.abs(e), 0) / err.length;
  return { n: err.length, maxPx: max, meanPx: mean };
}
