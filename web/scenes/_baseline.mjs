// Visual regression: a scene's capture against its approved picture.
//
// An approved picture (a baseline) is a PNG at
// `web/scenes/baselines/<fixture>/<name>.png`, stored in Git LFS. A scene
// captures through `ctx.matchBaseline(target, name)`; the capture must match
// its baseline to within pixelmatch's per-pixel colour threshold, anti-aliased
// edges ignored, with content allowed a pixel or two off its approved place
// and the capture a pixel or two larger or smaller. A mismatch fails the check
// and leaves `<name>.actual.png` and `<name>.diff.png` (differences in red,
// forgiven shifts in yellow, over the faded baseline) in the scene's evidence.
//
// A baseline is a human's approval, never the code's: with
// `UPDATE_BASELINES=1` (or a comma-separated list of fixture ids) the runner
// writes the captures as the new baselines instead of comparing, and they
// are reviewed in the diff like any source change. A missing baseline fails
// until it is blessed this way; a baseline a completed scene no longer
// captures fails as stale, so a removed shot is removed on purpose.
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

export const BASELINES = new URL("./baselines/", import.meta.url);

/** pixelmatch's per-pixel threshold (0..1 in YIQ colour distance). */
const PIXEL_THRESHOLD = 0.1;

/** How far content may be drawn off its approved place, in pixels, and how
 *  much a capture's size may differ. Rasterising moves an edge a pixel or two
 *  for no visible reason; a new, missing or recoloured thing is not a shift.
 *  Every other differing pixel fails: tolerance is by where a pixel is, never
 *  how many, since a share of the frame hides a real change that size (a
 *  17 px cursor in a 1280 px frame). */
const SHIFT_PX = 2;

/** How far, per channel, a forgiven pixel's colour may lie outside the colours
 *  within `SHIFT_PX` of it in the other picture: about pixelmatch's threshold. */
const SHIFT_COLOUR = 24;

const LFS_POINTER = "version https://git-lfs";

/** Whether this run blesses `fixtureId`'s captures instead of comparing. */
function blessing(fixtureId, env = process.env.UPDATE_BASELINES) {
  if (!env) return false;
  if (env === "1" || env === "true") return true;
  return env.split(",").includes(fixtureId);
}

/** The colour pixelmatch paints a differing pixel in the diff picture. */
const DIFFERS = [255, 0, 0];

/** `png` cut to its top-left `width` × `height`. */
function cropped(png, width, height) {
  if (png.width === width && png.height === height) return png;
  const out = new PNG({ width, height });
  for (let y = 0; y < height; y++)
    png.data.copy(out.data, y * width * 4, y * png.width * 4, (y * png.width + width) * 4);
  return out;
}

/** Whether `a`'s pixel at (x, y) is drawn from the colours within `SHIFT_PX`
 *  of it in `b`: one of them moved there, or a blend of them, as an edge
 *  rasterised a fraction of a pixel away is. A colour not among them is new. */
function nearby(a, b, x, y) {
  const lo = [255, 255, 255];
  const hi = [0, 0, 0];
  for (let by = Math.max(0, y - SHIFT_PX); by <= Math.min(b.height - 1, y + SHIFT_PX); by++)
    for (let bx = Math.max(0, x - SHIFT_PX); bx <= Math.min(b.width - 1, x + SHIFT_PX); bx++)
      for (let c = 0; c < 3; c++) {
        const v = b.data[(by * b.width + bx) * 4 + c];
        if (v < lo[c]) lo[c] = v;
        if (v > hi[c]) hi[c] = v;
      }
  const i = (y * a.width + x) * 4;
  for (let c = 0; c < 3; c++)
    if (a.data[i + c] < lo[c] - SHIFT_COLOUR || a.data[i + c] > hi[c] + SHIFT_COLOUR) return false;
  return true;
}

/** Compare two decoded PNGs: the pixels that differ beyond a shift of up to
 *  `SHIFT_PX`, and a diff picture (red: differs; yellow: forgiven). Both ways
 *  round, so a removed thing is not forgiven for the ground it leaves. */
export function comparePngs(expected, actual) {
  if (
    Math.abs(expected.width - actual.width) > SHIFT_PX ||
    Math.abs(expected.height - actual.height) > SHIFT_PX
  )
    return { sizeMismatch: true };
  const width = Math.min(expected.width, actual.width);
  const height = Math.min(expected.height, actual.height);
  const a = cropped(expected, width, height);
  const b = cropped(actual, width, height);
  const diff = new PNG({ width, height });
  pixelmatch(a.data, b.data, diff.data, width, height, {
    threshold: PIXEL_THRESHOLD,
    includeAA: false,
    alpha: 0.3,
    diffColor: DIFFERS,
  });
  let differing = 0;
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      // A differing pixel is pure red over the baseline's grey, never a grey.
      if (
        diff.data[i] !== DIFFERS[0] ||
        diff.data[i + 1] !== DIFFERS[1] ||
        diff.data[i + 2] !== DIFFERS[2]
      )
        continue;
      if (nearby(a, b, x, y) && nearby(b, a, x, y)) diff.data[i + 1] = 255;
      else differing++;
    }
  return { sizeMismatch: false, differing, diff };
}

/**
 * The baseline API for one fixture's run. `check` reports a result the way
 * the scene's own checks do; `evidencePath` names a file in its evidence.
 * `root` and `env` are the runner's (the baselines folder, UPDATE_BASELINES).
 */
export function baselines(
  fixtureId,
  { check, evidencePath },
  { root = BASELINES, env = process.env.UPDATE_BASELINES } = {},
) {
  const dir = new URL(`${fixtureId}/`, root);
  const bless = blessing(fixtureId, env);
  const captured = new Set();
  return {
    /** Capture `target` (a page or a locator) and match it against `name`.
     *  `style` is CSS applied for the capture only: to hide what can never
     *  be pinned (a live 3D backdrop, a build's identity), never a defect.
     *  Returns the captured PNG, for a scene that also measures it. */
    async match(target, name, { style } = {}) {
      if (!/^[a-z0-9][a-z0-9-]*$/.test(name)) throw new Error(`bad baseline name "${name}"`);
      if (captured.has(name)) throw new Error(`baseline "${name}" captured twice`);
      captured.add(name);
      // Settled: every font loaded and two frames laid out, however late
      // the last render came (a label drawn in a layout effect, a font it
      // first asks for). Frozen: no running animation, transition or caret.
      const page = typeof target.page === "function" ? target.page() : target;
      await page.evaluate(async () => {
        await document.fonts.ready;
        await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      });
      const png = await target.screenshot({
        animations: "disabled",
        caret: "hide",
        style,
      });
      const file = new URL(`${name}.png`, dir);
      if (bless) {
        await mkdir(dir, { recursive: true });
        await writeFile(file, png);
        check(`looks as approved: ${name} (blessed as the new baseline)`, true);
        return png;
      }
      const stored = await readFile(file).catch(() => null);
      await writeFile(evidencePath(`${name}.actual.png`), png);
      if (!stored) {
        check(
          `looks as approved: ${name}`,
          false,
          `no baseline at ${file.pathname}; review ${name}.actual.png, then bless with UPDATE_BASELINES=${fixtureId}`,
        );
        return png;
      }
      if (stored.subarray(0, LFS_POINTER.length).toString() === LFS_POINTER) {
        check(
          `looks as approved: ${name}`,
          false,
          `the baseline is an unfetched LFS pointer: git lfs pull --include="web/scenes/baselines/**"`,
        );
        return png;
      }
      const expected = PNG.sync.read(stored);
      const actual = PNG.sync.read(png);
      const { sizeMismatch, differing, diff } = comparePngs(expected, actual);
      if (diff) await writeFile(evidencePath(`${name}.diff.png`), PNG.sync.write(diff));
      check(
        `looks as approved: ${name}`,
        !sizeMismatch && differing === 0,
        sizeMismatch
          ? `size ${actual.width}×${actual.height}, baseline ${expected.width}×${expected.height}`
          : `${differing} px differ; see ${name}.diff.png`,
      );
      return png;
    },
    /** After a completed scene: every baseline was captured this run. */
    async finish() {
      if (bless) return;
      const stored = (await readdir(dir).catch(() => []))
        .filter((f) => f.endsWith(".png"))
        .map((f) => f.slice(0, -4));
      const stale = stored.filter((name) => !captured.has(name));
      if (stored.length)
        check(
          "every approved baseline is still captured",
          stale.length === 0,
          stale.length ? `stale: ${stale.join(", ")} (delete them if the shot was retired)` : "",
        );
    },
  };
}

/** Take the pointer out of the page, as a player not pointing at anything:
 *  hover states clear and the game cursor hides, so a capture that is not
 *  of the pointer does not draw it. It touches the corner first: just after a
 *  click replaces the element under the pointer, Chromium has no element to
 *  leave until it hit-tests again, and leaving at once reports nothing. */
export async function pointerAway(page) {
  await page.mouse.move(0, 0);
  await page.mouse.move(-5, -5);
}
