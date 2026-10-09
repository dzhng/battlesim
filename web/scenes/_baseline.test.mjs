// @vitest-environment node
import { mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { PNG } from "pngjs";
import { afterEach, beforeEach, expect, test } from "vitest";
import { baselines } from "./_baseline.mjs";

/** A 200×100 dark card, with a lit `w`×`h` block at its top left: a stand-in
 *  for a HUD element, the block its content. */
function card(w, h) {
  const png = new PNG({ width: 200, height: 100 });
  for (let y = 0; y < 100; y++)
    for (let x = 0; x < 200; x++) {
      const i = (y * 200 + x) * 4;
      const lit = x < w && y < h;
      png.data.set(lit ? [120, 220, 255, 255] : [16, 24, 32, 255], i);
    }
  return PNG.sync.write(png);
}

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
  expect(checks.at(-1).detail).toMatch(/^4020 px differ/);
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

test("a resized capture fails whatever its pixels", async () => {
  await run("fx").match(target(card(160, 30)), "detail");
  const wide = new PNG({ width: 201, height: 100 });
  await run().match(target(PNG.sync.write(wide)), "detail");
  expect(checks.at(-1)).toMatchObject({ ok: false, detail: "size 201×100, baseline 200×100" });
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
