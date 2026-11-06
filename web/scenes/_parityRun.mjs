// The native parity control run, shared by the scenes that film one: the
// graphics test's reel with control plates (`/labs/graphics-test?plates=1`),
// at 1280×720 (or the size asked for) on a visible page, unattended. For
// each scene it writes
//
//   throwaway/native-parity/<run>/<scene>/{capture.json, manifest.json,
//     overlays.bin, cards.bin, plate-<index>.png, plain-<index>.png}
//
// the capture (its publications and frame records), the indicators each
// frame drew (`overlayCapture.ts`; with `?indicators=1`), the unit info
// cards each frame showed (`cardCapture.ts`; with `?cards=1` or `open`), the
// `native-visual-presentation` manifest (the battle's identity, the lens and
// layers, and each plate's grass, building-module, scenery, posed-model and
// effect counts), and the plates read back in their own frames with each
// frame lit without its grass. The world (ground, buildings, units, effects,
// light) travels as the repository itself: Rust resolves it. It waits on the
// route's stage, never on a timer.
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { plainPlateFile } from "../src/battle/benchmark/presentationCapture.ts";
import {
  OVERLAY_CAPTURE_FILE,
  decodeOverlayCapture,
} from "../src/battle/benchmark/overlayCapture.ts";
import { CARD_CAPTURE_FILE, decodeCardCapture } from "../src/battle/benchmark/cardCapture.ts";
import { writeVisualPresentation } from "../src/battle/benchmark/visualPresentation.ts";

const ROOT = new URL("../../", import.meta.url);
const OUT = new URL("throwaway/native-parity/", ROOT);
const VIEWPORT = { width: 1280, height: 720 };

/** The page size `<w>x<h>` names (PARITY_VIEWPORT), or the run's default. */
export function parityViewport(value) {
  if (!value) return VIEWPORT;
  const match = /^(\d+)x(\d+)$/.exec(value);
  if (!match) throw new Error(`PARITY_VIEWPORT=${value}: expected <width>x<height>, as 1920x1080`);
  return { width: Number(match[1]), height: Number(match[2]) };
}

/** Each kind of card a scene's frames showed, and in how many frames: the
 *  cases a reviewer looks for (own, ally, identified enemy, contact, a
 *  remembered contact retiring). */
function cardCases(file) {
  const kinds = new Map();
  for (const frame of file.frames)
    for (const kind of new Set(frame.map(([card]) => file.cards[card].replace(/-\d+/, ""))))
      kinds.set(kind, (kinds.get(kind) ?? 0) + 1);
  return [...kinds].map(([kind, frames]) => `${kind} in ${frames}`).join(", ") || "none";
}

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
 *  counts, `viewport` sizes the page, and `pageShots` also screenshots the
 *  live page about once a second (`page-<reel ms>.png`, the page's HUD and
 *  cards over its canvas), for comparing with the frame native draws at
 *  that reel time; the shots cost the run its timing. Answers the run's
 *  directory, or null when the run did not complete (a failed check says
 *  why). */
export async function filmParityRun(
  ctx,
  url,
  { counts = false, viewport: asked = VIEWPORT, pageShots = false } = {},
) {
  await prerequisites();
  const started = performance.now();
  const page = await ctx.newPage({ viewport: asked });
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
  const shots = [];
  for (;;) {
    const state = await page.evaluate(() => ({
      stage: window.__graphicsTest?.stage,
      progress: document.querySelector(".graphics-test-progress span")?.textContent ?? "",
    }));
    if (state.stage === "results") break;
    if (pageErrors.length) throw new Error(`the page failed: ${pageErrors[0]}`);
    if (pageShots) {
      // The reel time either side of the shot: the frame native draws there.
      const reel = () => page.evaluate(() => window.__graphicsTestReelMs?.() ?? null);
      const before = await reel();
      const png = before === null ? null : await page.screenshot();
      const after = await reel();
      if (png && after !== null)
        shots.push({ map: state.progress.split("· ").at(-1), before, after, png });
    }
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

  // Plates cross from the page as base64 PNGs, overlays and cards as base64 bytes.
  const { plates, counted, overlays, cards } = await page.evaluate(async () => {
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
      overlays: await Promise.all(
        state.overlays.map(async ({ scene, bytes }) => ({
          scene,
          base64: await base64(new Blob([bytes])),
        })),
      ),
      cards: await Promise.all(
        state.cards.map(async ({ scene, bytes }) => ({
          scene,
          base64: await base64(new Blob([await bytes])),
        })),
      ),
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
    // The indicators, in lockstep with the capture's frames, where the
    // manifest names its `overlays` layer.
    const drewOverlays = report.drawnPresentations[i]?.layers.includes("overlays") ?? false;
    const recorded = overlays.find((o) => o.scene === map);
    if (drewOverlays) {
      const bytes = recorded && Buffer.from(recorded.base64, "base64");
      const overlayFrames = bytes ? decodeOverlayCapture(bytes).frames.length : 0;
      const frameCount = JSON.parse(captureJson).frames.length;
      ctx.check(
        `${map}: the indicators are recorded with every frame`,
        overlayFrames === frameCount,
        `${overlayFrames} overlay frames for ${frameCount} frame records`,
      );
      if (bytes) {
        await writeFile(new URL(OVERLAY_CAPTURE_FILE, dir), bytes);
        console.log(
          `METRIC parity-run ${map} indicators: ${(bytes.length / 2 ** 20).toFixed(1)} MiB over ${overlayFrames} frames`,
        );
      }
    }
    for (const shot of shots.filter((s) => s.map === map)) {
      const file = `page-${Math.round(shot.before)}.png`;
      await writeFile(new URL(file, dir), shot.png);
      console.log(
        `METRIC parity-run ${map} page shot ${file}: reel ${shot.before.toFixed(0)}–${shot.after.toFixed(0)} ms`,
      );
    }
    // The cards, likewise, where the manifest names its `cards` layer.
    if (report.drawnPresentations[i]?.layers.includes("cards") ?? false) {
      const filmed = cards.find((c) => c.scene === map);
      const bytes = filmed && Buffer.from(filmed.base64, "base64");
      const decoded = bytes ? decodeCardCapture(bytes) : null;
      const frameCount = JSON.parse(captureJson).frames.length;
      const cardFrames = decoded?.file.frames.length ?? 0;
      ctx.check(
        `${map}: the cards are recorded with every frame`,
        cardFrames === frameCount,
        `${cardFrames} card frames for ${frameCount} frame records`,
      );
      if (bytes && decoded) {
        await writeFile(new URL(CARD_CAPTURE_FILE, dir), bytes);
        console.log(
          `METRIC parity-run ${map} cards: ${(bytes.length / 2 ** 20).toFixed(1)} MiB, ${decoded.file.parts.length} contents, ${decoded.file.sprites.length} sprites in a ${decoded.atlas.width}×${decoded.atlas.height} atlas; frames showing each: ${cardCases(decoded.file)}`,
        );
      }
    }
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
      `${map}: frames, plates and plain plates are ${asked.width}×${asked.height}`,
      viewport.width === asked.width &&
        viewport.height === asked.height &&
        sizes.every((s) => s === `${asked.width}×${asked.height}`),
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
