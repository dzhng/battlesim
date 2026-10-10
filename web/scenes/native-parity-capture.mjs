// The native parity control run: the graphics test's reel with control plates
// (`/graphics-test?plates=1`), at 1280×720 on a visible page, unattended.
// For each scene it writes
//
//   throwaway/native-parity/<run>/<scene>/{capture.json, manifest.json, plate-<index>.png}
//
// the capture and its frame records, the `native-visual-presentation`
// manifest, and the plates read back from the canvas in their own frames.
// PARITY_SCENES=<map,…> films only those scenes (`?scenes=`). Run it under
// the GPU lock; it waits on the route's stage, never on a timer.
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { writeVisualPresentation } from "../src/battle/benchmark/visualPresentation.ts";

const ROOT = new URL("../../", import.meta.url);
const OUT = new URL("throwaway/native-parity/", ROOT);
const VIEWPORT = { width: 1280, height: 720 };

/** Refuse to start without what the route needs, naming how to build it. */
async function prerequisites() {
  const target = process.env.CARGO_TARGET_DIR ?? "throwaway/target";
  const needs = [
    [new URL("web/src/wasm/game_wasm_bg.wasm", ROOT), "the WebAssembly (bun run build:wasm)"],
    [
      new URL(`${target.replace(/\/?$/, "/")}debug/examples/mechanics_validate`, ROOT),
      "the mechanics validator (bun run build:mechanics)",
    ],
  ];
  for (const [file, what] of needs)
    await access(file).catch(() => {
      throw new Error(`missing ${what}: ${file.pathname}`);
    });
}

/** A PNG's pixel size, from its header. */
function pngSize(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return [view.getUint32(16), view.getUint32(20)];
}

export async function run(ctx) {
  await prerequisites();
  const started = performance.now();
  const page = await ctx.newPage({ viewport: VIEWPORT });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const scenes = process.env.PARITY_SCENES;
  const url = scenes ? `${ctx.url}&scenes=${encodeURIComponent(scenes)}` : ctx.url;
  await page.goto(url);
  await page.waitForFunction(() => window.__graphicsTest?.stage, undefined, { timeout: 60_000 });
  const refusal = await page.evaluate(() =>
    window.__graphicsTest.stage === "refused"
      ? document.querySelector("[role=alert]")?.textContent
      : null,
  );
  if (refusal) throw new Error(`the graphics test refused the run: ${refusal}`);
  await page.getByRole("button", { name: "Run graphics test" }).click();

  // Progress: one line per scene as it starts; out on a result or a page error.
  let shown = "";
  for (;;) {
    const state = await page.evaluate(() => ({
      stage: window.__graphicsTest?.stage,
      progress: document.querySelector(".graphics-test-progress span")?.textContent ?? "",
    }));
    if (state.stage === "results") break;
    if (pageErrors.length) throw new Error(`the page failed: ${pageErrors[0]}`);
    if (state.progress && state.progress !== shown) {
      shown = state.progress;
      console.log(`  ${shown} (${((performance.now() - started) / 1000).toFixed(0)} s)`);
    }
    await page.waitForTimeout(1000);
  }
  const report = await page.evaluate(() => window.__graphicsTest.report);
  ctx.check(
    "the capture run completed every scene",
    report.outcome.status === "complete",
    report.outcome.reason,
  );
  if (report.outcome.status !== "complete") return;

  const plates = await page.evaluate(() =>
    Promise.all(
      window.__graphicsTest.plates.map(async ({ scene, file, png }) => {
        const blob = await png;
        const data = await new Promise((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(reader.result);
          reader.onerror = () => reject(reader.error);
          reader.readAsDataURL(blob);
        });
        return { scene, file, base64: data.slice(data.indexOf(",") + 1) };
      }),
    ),
  );
  const runDir = new URL(`${new Date().toISOString().replace(/[:.]/g, "-")}/`, OUT);
  for (const [i, result] of report.scenes.entries()) {
    const { map, reel } = result.scene;
    const dir = new URL(`${map}/`, runDir);
    await mkdir(dir, { recursive: true });
    const captureJson = report.presentationCaptureJson[i];
    await writeFile(new URL("capture.json", dir), captureJson);
    const mapBytes = await readFile(new URL(`fixtures/maps/${map}/map.json`, ROOT));
    const manifest = await writeVisualPresentation({
      captureJson,
      mapBytes,
      drawn: report.drawnPresentations[i],
    });
    await writeFile(new URL("manifest.json", dir), manifest);
    const own = plates.filter((p) => p.scene === map);
    const sizes = [];
    for (const plate of own) {
      const bytes = Buffer.from(plate.base64, "base64");
      sizes.push(pngSize(bytes).join("×"));
      await writeFile(new URL(plate.file, dir), bytes);
    }
    const frames = JSON.parse(captureJson).frames;
    const named = frames.filter((f) => f.plate !== null).map((f) => f.plate);
    ctx.check(
      `${map}: one plate per shot, each named by its frame`,
      own.length === reel.shots.length && named.join() === own.map((p) => p.file).join(),
      `${own.length} plates for ${reel.shots.length} shots; frames name ${named.join(", ")}`,
    );
    const { viewport } = JSON.parse(manifest);
    ctx.check(
      `${map}: frames and plates are ${VIEWPORT.width}×${VIEWPORT.height}`,
      viewport.width === VIEWPORT.width &&
        viewport.height === VIEWPORT.height &&
        sizes.every((s) => s === `${VIEWPORT.width}×${VIEWPORT.height}`),
      `viewport ${viewport.width}×${viewport.height}; plates ${[...new Set(sizes)].join(", ")}`,
    );
    console.log(
      `METRIC native-parity-capture ${map}: ${frames.length} frames, ${own.length} plates → ${dir.pathname}`,
    );
  }
  console.log(
    `METRIC native-parity-capture: ${report.scenes.length} scene(s) in ${((performance.now() - started) / 1000).toFixed(0)} s → ${runDir.pathname}`,
  );
}
