// The native parity control run, shared by the scenes that film one: the
// graphics test's reel with control plates (`/labs/graphics-test?plates=1`),
// at 1280×720 on a visible page, unattended. For each scene it writes
//
//   throwaway/native-parity/<run>/<scene>/{capture.json, manifest.json,
//     plate-<index>.png, plain-<index>.png}
//
// the capture (its publications and frame records), the
// `native-visual-presentation` manifest (the battle's identity, the lens and
// layers, and each plate's grass, building-module, scenery, posed-model and
// effect counts), and the plates read back in their own frames with each
// frame lit without its grass. The world (ground, buildings, units, effects,
// light) travels as the repository itself: Rust resolves it. It waits on the
// route's stage, never on a timer.
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { plainPlateFile } from "../src/battle/benchmark/presentationCapture.ts";
import { writeVisualPresentation } from "../src/battle/benchmark/visualPresentation.ts";

const ROOT = new URL("../../", import.meta.url);
const OUT = new URL("throwaway/native-parity/", ROOT);
const VIEWPORT = { width: 1280, height: 720 };

/** Refuse to start without what the route needs, naming how to build it. */
async function prerequisites() {
  const target = process.env.CARGO_TARGET_DIR ?? "target";
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

/** Film the graphics test at `url` (`/labs/graphics-test?plates=1` and
 *  its other parameters) and write each scene's parity run to
 *  `throwaway/native-parity/<run>/<scene>/`; `counts` says the url asked for
 *  counts. Answers the run's directory, or null when the run did not
 *  complete (a failed check says why). */
export async function filmParityRun(ctx, url, { counts = false } = {}) {
  await prerequisites();
  const started = performance.now();
  const page = await ctx.newPage({ viewport: VIEWPORT });
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
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
  if (report.outcome.status !== "complete") return null;

  // Plates cross from the page as base64 PNGs.
  const { plates, counted } = await page.evaluate(async () => {
    const base64 = async (blob) => {
      const data = await new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result);
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(blob);
      });
      return data.slice(data.indexOf(",") + 1);
    };
    const state = window.__graphicsTest;
    return {
      counted: state.counts,
      plates: await Promise.all(
        state.plates.map(async ({ scene, file, png, plain, grass, drawn, models, effects }) => ({
          scene,
          file,
          base64: await base64(await png),
          plain: await base64(await plain),
          cells: await grass,
          drawn,
          models,
          effects,
        })),
      ),
    };
  });
  const runDir = new URL(`${new Date().toISOString().replace(/[:.]/g, "-")}/`, OUT);
  for (const [i, result] of report.scenes.entries()) {
    const { map, reel } = result.scene;
    const dir = new URL(`${map}/`, runDir);
    await mkdir(dir, { recursive: true });
    const captureJson = report.presentationCaptureJson[i];
    await writeFile(new URL("capture.json", dir), captureJson);
    const mapBytes = await readFile(new URL(`fixtures/maps/${map}/map.json`, ROOT));
    const own = plates.filter((p) => p.scene === map);
    const manifest = await writeVisualPresentation({
      captureJson,
      mapBytes,
      drawn: report.drawnPresentations[i],
      grassCounts: own.map(({ file, cells }) => ({ plate: file, cells })),
      instanceCounts: own.map(({ file, drawn }) => ({ plate: file, ...drawn })),
      modelCounts: own.map(({ file, models }) => ({ plate: file, ...models })),
      effectCounts: own.map(({ file, effects }) => ({ plate: file, ...effects })),
      frameCounts: counted.find((c) => c.scene === map)?.frames ?? [],
    });
    await writeFile(new URL("manifest.json", dir), manifest);
    const sizes = [];
    for (const plate of own)
      for (const [file, base64] of [
        [plate.file, plate.base64],
        [plainPlateFile(plate.file), plate.plain],
      ]) {
        const bytes = Buffer.from(base64, "base64");
        sizes.push(pngSize(bytes).join("×"));
        await writeFile(new URL(file, dir), bytes);
      }
    const { frames, tickHz } = JSON.parse(captureJson);
    const named = frames.filter((f) => f.plate !== null).map((f) => f.plate);
    ctx.check(
      `${map}: one plate per shot, each named by its frame`,
      own.length === reel.shots.length && named.join() === own.map((p) => p.file).join(),
      `${own.length} plates for ${reel.shots.length} shots; frames name ${named.join(", ")}`,
    );
    const parsed = JSON.parse(manifest);
    const { viewport } = parsed;
    ctx.check(
      `${map}: frames, plates and plain plates are ${VIEWPORT.width}×${VIEWPORT.height}`,
      viewport.width === VIEWPORT.width &&
        viewport.height === VIEWPORT.height &&
        sizes.every((s) => s === `${VIEWPORT.width}×${VIEWPORT.height}`),
      `viewport ${viewport.width}×${viewport.height}; plates ${[...new Set(sizes)].join(", ")}`,
    );
    const grown = parsed.grassCounts.map((g) => g.cells.reduce((a, b) => a + b, 0));
    const sum = (list) => list.reduce((a, b) => a + b, 0);
    const modules = parsed.instanceCounts.map((c) => sum(c.buildings));
    const props = parsed.instanceCounts.map((c) => sum(c.props.map((p) => sum(p.levels))));
    const scenery = parsed.instanceCounts.map((c) =>
      ["forest", "understorey", "backdrop", "dressing"].map((p) => sum(c.scenery[p])).join("/"),
    );
    // Every reel map has scenery, at least past its edge: a scene that drew
    // none placed none (its art never reached the presentation).
    ctx.check(
      `${map}: the scenery is drawn`,
      parsed.instanceCounts.some((c) => sum(Object.values(c.scenery).flat()) > 0),
      `forest/understorey/backdrop/dressing per plate ${scenery.join(", ")}`,
    );
    const posed = parsed.modelCounts.map((c) =>
      [c.soldiers, c.vehicles, c.aircraft, c.corpses, c.fading].join("/"),
    );
    console.log(
      `METRIC parity-run ${map} units: soldiers/vehicles/aircraft/corpses/fading per plate ${posed.join(", ")}`,
    );
    console.log(
      `METRIC parity-run ${map}: ${frames.length} frames, ${own.length} plates, grass per plate ${grown.join(", ")}; building modules per plate ${modules.join(", ")}; forest/understorey/backdrop/dressing per plate ${scenery.join(", ")}; props per plate ${props.join(", ")} → ${dir.pathname}`,
    );
    const effected = parsed.effectCounts.map(
      (c) => `${sum(Object.values(c.running))}/${sum(Object.values(c.instances))}/${c.lights}`,
    );
    console.log(
      `METRIC parity-run ${map} effects: running/instances/lights per plate ${effected.join(", ")}`,
    );
    if (counts) {
      const boundaries = parsed.frameCounts.filter(({ frame }) => {
        const t = frames[frame].simTime;
        return Math.round(t * tickHz) / tickHz === t;
      }).length;
      console.log(
        `METRIC parity-run ${map} counted frames: ${parsed.frameCounts.length}, ${boundaries} on a tick boundary`,
      );
    }
  }
  console.log(
    `METRIC parity-run: ${report.scenes.length} scene(s) in ${((performance.now() - started) / 1000).toFixed(0)} s → ${runDir.pathname}`,
  );
  return runDir;
}
