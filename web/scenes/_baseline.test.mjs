// @vitest-environment node
import { mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { PNG } from "pngjs";
import { afterEach, beforeEach, expect, test } from "vitest";
import { baselines, comparePngs } from "./_baseline.mjs";

const DARK = [16, 24, 32];
const LIT = [120, 220, 255];

/** A dark picture with lit boxes: `[x, y, w, h, colour?]`. */
function picture(width, height, boxes) {
  const png = new PNG({ width, height });
  for (let i = 0; i < width * height; i++) png.data.set([...DARK, 255], i * 4);
  for (const [x0, y0, w, h, colour = LIT] of boxes)
    for (let y = y0; y < y0 + h; y++)
      for (let x = x0; x < x0 + w; x++) png.data.set([...colour, 255], (y * width + x) * 4);
  return png;
}

/** A 200×100 dark card, with a lit `w`×`h` block at its top left: a stand-in
 *  for a HUD element, the block its content. */
const card = (w, h) => PNG.sync.write(picture(200, 100, [[0, 0, w, h]]));

/** What a page offers the helper: a screenshot, and a page to settle. */
const target = (png) => ({ screenshot: async () => png, evaluate: async () => {} });

let dir;
let checks;
let run;
beforeEach(async () => {
  dir = await mkdtemp(join(tmpdir(), "baseline-"));
  await mkdir(join(dir, "evidence"));
  checks = [];
  run = (env) =>
    baselines(
      "fx",
      {
        check: (name, ok, detail) => checks.push({ name, ok, detail }),
        evidencePath: (name) => join(dir, "evidence", name),
      },
      { root: pathToFileURL(join(dir, "baselines") + "/"), env },
    );
});
afterEach(() => rm(dir, { recursive: true, force: true }));

test("a capture without an approved picture fails until it is blessed", async () => {
  await run().match(target(card(160, 30)), "detail");
  expect(checks.at(-1).ok).toBe(false);
  expect(checks.at(-1).detail).toContain("UPDATE_BASELINES=fx");

  await run("fx").match(target(card(160, 30)), "detail");
  expect(checks.at(-1).ok).toBe(true);
  expect(await readFile(join(dir, "baselines/fx/detail.png"))).toEqual(card(160, 30));
});

test("an unchanged picture passes and a collapsed one fails with a diff", async () => {
  await run("fx").match(target(card(160, 30)), "detail");

  const same = run();
  await same.match(target(card(160, 30)), "detail");
  await same.finish();
  expect(checks.slice(-2).map((c) => c.ok)).toEqual([true, true]);

  // The content shrinks to a sliver, as a card sized to nothing does.
  await run().match(target(card(26, 30)), "detail");
  expect(checks.at(-1).ok).toBe(false);
  expect(checks.at(-1).detail).toMatch(/^\d+ px differ/);
  const diff = PNG.sync.read(await readFile(join(dir, "evidence/detail.diff.png")));
  expect([diff.width, diff.height]).toEqual([200, 100]);
});

test("imperceptible colour noise passes, a moved edge does not", async () => {
  await run("fx").match(target(card(160, 30)), "detail");
  // Every lit pixel a shade off, as a different GPU's blending might draw it.
  const noisy = PNG.sync.read(card(160, 30));
  for (let i = 0; i < noisy.data.length; i += 4) if (noisy.data[i] === 120) noisy.data[i] = 123;
  await run().match(target(PNG.sync.write(noisy)), "detail");
  expect(checks.at(-1).ok).toBe(true);
  await run().match(target(card(160, 40)), "detail");
  expect(checks.at(-1).ok).toBe(false);
});

test("one clearly changed pixel fails: an unchanged capture is identical", async () => {
  await run("fx").match(target(card(160, 30)), "detail");
  const touched = PNG.sync.read(card(160, 30));
  touched.data.set([255, 0, 0, 255], (50 * 200 + 180) * 4);
  await run().match(target(PNG.sync.write(touched)), "detail");
  expect(checks.at(-1)).toMatchObject({ ok: false, detail: expect.stringMatching(/^1 px differ/) });
});

test("a capture resized beyond two pixels fails whatever its pixels", async () => {
  await run("fx").match(target(card(160, 30)), "detail");
  const wide = new PNG({ width: 203, height: 100 });
  await run().match(target(PNG.sync.write(wide)), "detail");
  expect(checks.at(-1)).toMatchObject({ ok: false, detail: "size 203×100, baseline 200×100" });
});

test("an approved picture a completed scene no longer captures is stale", async () => {
  const blessed = run("fx");
  await blessed.match(target(card(160, 30)), "kept");
  await blessed.match(target(card(160, 30)), "retired");

  const later = run();
  await later.match(target(card(160, 30)), "kept");
  await later.finish();
  expect(checks.at(-1)).toMatchObject({ ok: false, detail: expect.stringContaining("retired") });
});

test("an unfetched LFS pointer is named, not decoded", async () => {
  await mkdir(join(dir, "baselines/fx"), { recursive: true });
  await writeFile(
    join(dir, "baselines/fx/detail.png"),
    "version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n",
  );
  await run().match(target(card(160, 30)), "detail");
  expect(checks.at(-1)).toMatchObject({ ok: false, detail: expect.stringContaining("lfs pull") });
});

// What a capture may differ from its approved picture by and still look as
// approved: drawn a pixel or two off, never changed.
const WORD = [20, 30, 60, 12];

test("content drawn up to two pixels off still looks as approved", () => {
  const approved = picture(120, 80, [WORD]);
  for (const [dx, dy] of [
    [1, 0],
    [0, 2],
    [-2, 1],
  ]) {
    const moved = picture(120, 80, [[WORD[0] + dx, WORD[1] + dy, WORD[2], WORD[3]]]);
    expect(comparePngs(approved, moved).differing).toBe(0);
  }
});

test("a thin mark drawn where nothing was is a difference", () => {
  // A cursor's stroke: every pixel of it is within a shift of bare ground.
  const approved = picture(120, 80, [WORD]);
  const pointed = picture(120, 80, [WORD, [90, 50, 1, 12]]);
  expect(comparePngs(approved, pointed).differing).toBeGreaterThan(0);
});

test("a thin mark gone from where it was is a difference", () => {
  const approved = picture(120, 80, [WORD, [90, 50, 1, 12]]);
  const missing = picture(120, 80, [WORD]);
  expect(comparePngs(approved, missing).differing).toBeGreaterThan(0);
});

test("the same thing in another colour is a difference", () => {
  const approved = picture(120, 80, [WORD]);
  const amber = picture(120, 80, [[...WORD, [255, 200, 90]]]);
  expect(comparePngs(approved, amber).differing).toBeGreaterThan(0);
});

test("content moved further than two pixels is a difference", () => {
  const approved = picture(120, 80, [WORD]);
  const moved = picture(120, 80, [[WORD[0] + 4, WORD[1], WORD[2], WORD[3]]]);
  expect(comparePngs(approved, moved).differing).toBeGreaterThan(0);
});

test("a capture up to two pixels larger or smaller is compared, beyond that it is not", () => {
  const approved = picture(120, 80, [WORD]);
  const taller = picture(120, 82, [[WORD[0], WORD[1] + 1, WORD[2], WORD[3]]]);
  expect(comparePngs(approved, taller)).toMatchObject({ sizeMismatch: false, differing: 0 });
  const grown = picture(120, 86, [WORD]);
  expect(comparePngs(approved, grown).sizeMismatch).toBe(true);
});

test("an edge blended to another shade between its colours looks as approved", () => {
  // Anti-aliasing: the word's edge band drawn a different mix of lit and dark,
  // two pixels wide so pixelmatch's own anti-aliasing test does not excuse it.
  const [x, y, w, h] = WORD;
  const approved = picture(120, 80, [WORD, [x + w, y, 2, h, [70, 120, 140]]]);
  const reblended = picture(120, 80, [WORD, [x + w, y, 2, h, [100, 170, 200]]]);
  expect(comparePngs(approved, reblended).differing).toBe(0);
});
