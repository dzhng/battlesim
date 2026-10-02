// A building's drawn art held to its physical parts, in a picture: the parts
// as the camera sees them, and the ground-classes view (black where anything
// stands over the ground) judged against them. The line-up holds every
// template to its parts with this; the ruins scene holds what a side draws to
// the boxes it knows.
import { readFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";
import { pixel } from "./_png.mjs";

const REPO = new URL("../../", import.meta.url);
/** The ground-classes view is black where a pixel is not wholly bare ground. */
export const isBody = (rgb) => rgb.every((v) => v === 0);
/** How far past a projected outline a covered pixel may lie: the outline is
 *  projected in floats and the frame is rasterised. */
const OUTLINE_PX = 1.5;
/** The ground searched for stray art round a building's projected parts. */
const SEARCH_PX = 16;
/** How far in from a face's edges its samples lie, as a share of its half
 *  size: eaves and corner posts are not the wall. */
const INSET = 0.6;
/** How far above its base a part's wall is sampled, metres: low enough that
 *  a building off the ground shows ground there, high enough that the ray
 *  through the sample meets a wall set a little inside the face. */
const FOOT_M = 1;
/** The share of a part's inside samples that must be covered: a part may be
 *  a roof on posts. Its foot must show somewhere along the wall. */
const COVERED = 2 / 3;
/** Samples along the foot of a wall, from end to end. */
const FOOT_SAMPLES = 41;
/** Clear picture round a building in a crop, metres. */
const CROP_M = 3;

/** Each source set's fit (how far its art may reach past a part's faces),
 *  and the set of each template, from the sets' own templates files. */
export async function setFits() {
  const catalog = JSON.parse(await readFile(new URL("assets/catalog.json", REPO), "utf8"));
  const fit = {};
  const setOf = {};
  for (const [set, { templates }] of Object.entries(catalog.city_sets)) {
    const text = await readFile(new URL(templates, REPO), "utf8");
    if (text.startsWith("version https://git-lfs"))
      throw new Error(
        `${templates} is a Git LFS pointer; run: git lfs pull --include="assets/source/city/*/templates.json"`,
      );
    const source = JSON.parse(text);
    fit[set] = source.fit;
    for (const template of source.templates) setOf[template.descriptor.id] = set;
  }
  return { fit, setOf };
}

/** The convex outline of `points`, counter-clockwise (monotone chain). */
function outline(points) {
  const sorted = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list) => {
    const hull = [];
    for (const p of list) {
      while (hull.length >= 2 && cross(hull.at(-2), hull.at(-1), p) <= 0) hull.pop();
      hull.push(p);
    }
    hull.pop();
    return hull;
  };
  return [...half(sorted), ...half(sorted.reverse())];
}

/** Whether (x, y) is inside the convex `hull`, or within `slack` of it. */
function within(hull, x, y, slack) {
  for (let i = 0; i < hull.length; i++) {
    const [a, b] = [hull[i], hull[(i + 1) % hull.length]];
    const [ex, ey] = [b[0] - a[0], b[1] - a[1]];
    // Signed distance to the edge's line, positive inside.
    if (((x - a[0]) * ey - (y - a[1]) * ex) / Math.hypot(ex, ey) > slack) return false;
  }
  return true;
}

/** The box round `points`, grown by `pad`. */
export const bounds = (points, pad) => {
  const [xs, ys] = [points.map((p) => p[0]), points.map((p) => p[1])];
  return {
    x: Math.min(...xs) - pad,
    y: Math.min(...ys) - pad,
    w: Math.max(...xs) - Math.min(...xs) + 2 * pad,
    h: Math.max(...ys) - Math.min(...ys) + 2 * pad,
  };
};

/** The box round `parts` in the world: `min` and `max`. */
export function boxRound(parts) {
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (const part of parts) {
    const [cos, sin] = [Math.abs(Math.cos(part.yaw)), Math.abs(Math.sin(part.yaw))];
    const reach = [
      cos * part.half[0] + sin * part.half[1],
      sin * part.half[0] + cos * part.half[1],
    ];
    for (const axis of [0, 1]) {
      min[axis] = Math.min(min[axis], part.center[axis] - reach[axis]);
      max[axis] = Math.max(max[axis], part.center[axis] + reach[axis]);
    }
    min[2] = Math.min(min[2], part.baseZ);
    max[2] = Math.max(max[2], part.baseZ + 2 * part.half[2]);
  }
  return { min, max };
}

/** A building's parts as the camera sees them: per part, the corners of the
 *  part grown by `fit` (`side_m` on its sides, `top_m` above it), sample
 *  points inside it at half its height (a ray to one has come through its
 *  roof or a wall) and at the foot of the wall nearest the camera; then the
 *  corners of the box `min`..`max` grown by `CROP_M`. All in page pixels.
 *  `entry` is `{ parts, min, max }`, each part a box (`center`, `yaw`,
 *  `half`, `baseZ`). */
export function projected(page, entry, fit) {
  return lab(
    page,
    ({ entry, fit, INSET, FOOT_M, FOOT_SAMPLES, CROP_M }) => {
      const px = (p) => window.__lab.projectToCss(p[0], p[1], p[2]);
      const eye = window.__lab.camera();
      const toEye = [Math.cos(eye.yaw), Math.sin(eye.yaw)];
      const corners = (min, max) =>
        [0, 1, 2, 3, 4, 5, 6, 7].map((k) =>
          px([k & 1 ? max[0] : min[0], k & 2 ? max[1] : min[1], k & 4 ? max[2] : min[2]]),
        );
      return {
        parts: entry.parts.map((part) => {
          const [cos, sin] = [Math.cos(part.yaw), Math.sin(part.yaw)];
          // A point of the part's own frame, in the world.
          const at = (u, v, z) => [
            part.center[0] + cos * u - sin * v,
            part.center[1] + sin * u + cos * v,
            z,
          ];
          const [hx, hy, hz] = part.half;
          const [gx, gy] = [hx + fit.side_m, hy + fit.side_m];
          const top = part.baseZ + 2 * hz;
          const grown = [];
          for (const u of [-gx, gx])
            for (const v of [-gy, gy])
              for (const z of [part.baseZ, top + fit.top_m]) grown.push(px(at(u, v, z)));
          const core = [];
          for (const u of [-INSET, 0, INSET])
            for (const v of [-INSET, 0, INSET]) core.push(px(at(u * hx, v * hy, part.baseZ + hz)));
          // The wall that faces the camera most squarely.
          const faces = [
            { normal: [cos, sin], out: [hx, 0], along: [0, hy] },
            { normal: [-cos, -sin], out: [-hx, 0], along: [0, hy] },
            { normal: [-sin, cos], out: [0, hy], along: [hx, 0] },
            { normal: [sin, -cos], out: [0, -hy], along: [hx, 0] },
          ];
          const facing = faces.reduce((a, b) =>
            a.normal[0] * toEye[0] + a.normal[1] * toEye[1] >=
            b.normal[0] * toEye[0] + b.normal[1] * toEye[1]
              ? a
              : b,
          );
          const foot = Array.from({ length: FOOT_SAMPLES }, (_, i) => {
            const t = (2 * i) / (FOOT_SAMPLES - 1) - 1;
            return px(
              at(
                facing.out[0] + t * facing.along[0],
                facing.out[1] + t * facing.along[1],
                part.baseZ + FOOT_M,
              ),
            );
          });
          return { grown, core, foot };
        }),
        box: corners(
          [entry.min[0] - CROP_M, entry.min[1] - CROP_M, entry.min[2]],
          [entry.max[0] + CROP_M, entry.max[1] + CROP_M, entry.max[2] + CROP_M],
        ),
      };
    },
    { entry, fit, INSET, FOOT_M, FOOT_SAMPLES, CROP_M },
  );
}

/** Hold one building, alone in its part of the picture, to its parts as
 *  `seen` (`projected`) in the ground-classes view `mask`: `inside`, it is
 *  drawn and nothing of it lies outside its parts grown by the fit; `stands`,
 *  it fills each part and meets the ground along the foot of its wall. */
export function judge(mask, seen) {
  const hulls = seen.parts.map((part) => outline(part.grown));
  const search = bounds(
    seen.parts.flatMap((part) => part.grown),
    SEARCH_PX,
  );
  let covered = 0;
  let stray = 0;
  let worst = null;
  for (
    let y = Math.max(0, Math.floor(search.y));
    y < Math.min(mask.height, search.y + search.h);
    y++
  )
    for (
      let x = Math.max(0, Math.floor(search.x));
      x < Math.min(mask.width, search.x + search.w);
      x++
    ) {
      if (!isBody(pixel(mask, x, y))) continue;
      covered++;
      if (hulls.some((hull) => within(hull, x + 0.5, y + 0.5, OUTLINE_PX))) continue;
      stray++;
      worst ??= [x, y];
    }
  const share = (points) => points.filter((p) => isBody(pixel(mask, ...p))).length / points.length;
  const cores = seen.parts.map((part) => share(part.core));
  const feet = seen.parts.map((part) => share(part.foot));
  return {
    covered,
    stray,
    worst,
    cores,
    feet,
    inside: covered > 0 && stray === 0,
    stands: cores.every((c) => c >= COVERED) && feet.every((f) => f > 0),
  };
}
