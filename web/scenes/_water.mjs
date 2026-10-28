// The water as the scenes measure it: where the simulation says it is, and
// which pixels of a frame show it.
import { readFileSync } from "node:fs";
import { initSync, WorldView, world_layout } from "../src/wasm/game_wasm.js";
import { loadMap } from "../src/maps/node.ts";
import { nodeCatalogSet } from "../src/battle/catalog/node.ts";
import { linear, meanColour } from "./_colour.mjs";

/** How far (x, y) lies inside the water's edge (negative outside), by the
 *  simulation's exported river stretches `rivers`, `stride` floats each.
 *  Self-contained, so a page runs it too (`installInside`). */
export function waterInside(rivers, stride) {
  return (x, y) => {
    let best = -Infinity;
    for (let o = 0; o < rivers.length; o += stride) {
      const ax = rivers[o],
        ay = rivers[o + 1],
        dx = rivers[o + 2] - ax,
        dy = rivers[o + 3] - ay;
      const t = Math.min(1, Math.max(0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)));
      const half = rivers[o + 4] + (rivers[o + 5] - rivers[o + 4]) * t;
      best = Math.max(best, half - Math.hypot(x - ax - dx * t, y - ay - dy * t));
    }
    return best;
  };
}

/** In a lab page: `window.__inside(x, y)`, `waterInside` over the route's
 *  own exports. */
export const installInside = (page) =>
  page.evaluate(
    `window.__inside = (${waterInside})(window.__lab.route.exports().rivers, window.__lab.route.layout().riverStride)`,
  );

/** Saved map `id`'s water as the simulation builds it under the capture
 *  catalog: `inside(x, y)`, and the height of its surface. */
export async function mapWater(id) {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  const rules = JSON.stringify((await nodeCatalogSet("capture")).rules);
  const view = new WorldView(JSON.stringify(loadMap(id).definition), rules);
  const rivers = view.rivers();
  const { riverStride, riverFields } = JSON.parse(world_layout(rules));
  view.free();
  const surfaceZ = rivers[riverFields.indexOf("surfaceZ")];
  return { inside: waterInside(rivers, riverStride), surfaceZ };
}

/** The pixel a parity frame record (its `eye`, `target`, `up` and `canvas`)
 *  draws world point `p` at, through a lens of vertical field `fovY`; null
 *  behind the eye or off the canvas. */
function projector({ eye, target, up, canvas: [width, height] }, fovY) {
  const sub = (a, b) => a.map((v, k) => v - b[k]);
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const unit = (a) => a.map((v) => v / Math.hypot(...a));
  const cross = (a, b) => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
  const forward = unit(sub(target, eye));
  const right = unit(cross(forward, up));
  const upward = cross(right, forward);
  const focal = 1 / Math.tan(fovY / 2);
  return (p) => {
    const d = sub(p, eye);
    const depth = dot(d, forward);
    if (depth <= 0) return null;
    const x = ((focal * dot(d, right)) / depth) * (height / width);
    const y = (focal * dot(d, upward)) / depth;
    const px = [Math.floor(((x + 1) / 2) * width), Math.floor(((1 - y) / 2) * height)];
    return px[0] >= 0 && px[1] >= 0 && px[0] < width && px[1] < height ? px : null;
  };
}

/** The pixels of a frame that show `water` at least `inset` metres inside
 *  its edge, and those that show the dry ground `near` to `far` metres
 *  outside it, sampled every `step` metres over the map's `size`. Units and
 *  trees may stand over a few of them; a region's mean colour forgives that. */
export function waterAndBank(
  frame,
  fovY,
  water,
  size,
  { inset = 2, near = 8, far = 24, step = 0.5 } = {},
) {
  const project = projector(frame, fovY);
  const regions = { water: new Map(), bank: new Map() };
  for (let y = 0; y <= size[1]; y += step)
    for (let x = 0; x <= size[0]; x += step) {
      const inside = water.inside(x, y);
      const region = inside >= inset ? "water" : -inside >= near && -inside <= far ? "bank" : null;
      if (!region) continue;
      const px = project([x, y, region === "water" ? water.surfaceZ : 0]);
      if (px) regions[region].set(px[1] * frame.canvas[0] + px[0], px);
    }
  return { water: [...regions.water.values()], bank: [...regions.bank.values()] };
}

/** The mean colours of `png` over the water and the bank `regions`, linear
 *  rgb. */
export function waterAndBankColours(png, regions) {
  const mean = (pixels) => meanColour(png, pixels).rgb.map(linear);
  return { water: mean(regions.water), bank: mean(regions.bank) };
}
