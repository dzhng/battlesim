// Visual regression: a scene's capture against its approved picture.
//
// An approved picture (a baseline) is a PNG at
// `web/scenes/baselines/<fixture>/<name>.png`, stored in Git LFS. A scene
// captures through `ctx.matchBaseline(target, name)`; the capture must match
// its baseline to within pixelmatch's per-pixel colour threshold, anti-aliased
// edges ignored, and a tripwire share of differing pixels. A mismatch fails
// the check and leaves `<name>.actual.png` and `<name>.diff.png` (differences
// in red over the faded baseline) in the scene's evidence.
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

/** The share of pixels that may differ before a capture fails: a tripwire for
 *  sub-pixel noise, far below any real change (one wrapped word is hundreds
 *  of pixels; a collapsed card thousands). */
const MAX_DIFF_SHARE = 0.0005;

const LFS_POINTER = "version https://git-lfs";

/** Whether this run blesses `fixtureId`'s captures instead of comparing. */
export function blessing(fixtureId, env = process.env.UPDATE_BASELINES) {
  if (!env) return false;
  if (env === "1" || env === "true") return true;
  return env.split(",").includes(fixtureId);
}

/** Compare two decoded PNGs: the differing pixels, and a diff picture. */
export function comparePngs(expected, actual) {
  if (expected.width !== actual.width || expected.height !== actual.height)
    return {
      sizeMismatch: true,
      differing: expected.width * expected.height,
      diff: null,
    };
  const diff = new PNG({ width: expected.width, height: expected.height });
  const differing = pixelmatch(
    expected.data,
    actual.data,
    diff.data,
    expected.width,
    expected.height,
    { threshold: PIXEL_THRESHOLD, includeAA: false, alpha: 0.3 },
  );
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
    async match(target, name, { maxDiffShare = MAX_DIFF_SHARE, style } = {}) {
      if (!/^[a-z0-9][a-z0-9-]*$/.test(name)) throw new Error(`bad baseline name "${name}"`);
      if (captured.has(name)) throw new Error(`baseline "${name}" captured twice`);
      captured.add(name);
      // Frozen: no running animation, transition or blinking caret.
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
      const share = differing / (expected.width * expected.height);
      if (diff) await writeFile(evidencePath(`${name}.diff.png`), PNG.sync.write(diff));
      check(
        `looks as approved: ${name}`,
        !sizeMismatch && share <= maxDiffShare,
        sizeMismatch
          ? `size ${actual.width}×${actual.height}, baseline ${expected.width}×${expected.height}`
          : `${differing} px differ (${(share * 100).toFixed(3)}%); see ${name}.diff.png`,
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
      if (stored.length || stale.length)
        check(
          "every approved baseline is still captured",
          stale.length === 0,
          stale.length ? `stale: ${stale.join(", ")} (delete them if the shot was retired)` : "",
        );
    },
  };
}
