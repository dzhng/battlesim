// The native parity control run: the graphics test's reel with control plates
// (`/graphics-test?plates=1`), at 1280×720 on a visible page, unattended.
// For each scene it writes
//
//   throwaway/native-parity/<run>/<scene>/{capture.json, manifest.json,
//     plate-<index>.png, plain-<index>.png,
//     terrain-<table>.u32, grass-<table>.u32,
//     buildings-<table>.u32, scenery-<table>.u32, units-<table>.u32}
//
// the capture and its frame records, the `native-visual-presentation`
// manifest (with each plate's grass, building-module, scenery, posed-model
// and effect counts), the plates read back in their own frames with each
// frame lit without its grass, and the tables the manifest hashes: the ground's and
// grass's GPU tables, the buildings' and scenery's instances, and the units'
// clip tables.
// PARITY_SCENES=<map,…> films only those scenes (`?scenes=`). Run it under
// the GPU lock; it waits on the route's stage, never on a timer.
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

  // Bytes cross from the page as base64: plates as PNGs, GPU tables as
  // their words (little-endian, as the page holds them).
  const { plates, uploaded } = await page.evaluate(async () => {
    const base64 = async (blob) => {
      const data = await new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result);
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(blob);
      });
      return data.slice(data.indexOf(",") + 1);
    };
    const encode = async (group) =>
      Object.fromEntries(
        await Promise.all(
          Object.entries(group).map(async ([name, { words, layout }]) => [
            name,
            { base64: await base64(new Blob([words])), layout },
          ]),
        ),
      );
    const state = window.__graphicsTest;
    return {
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
      uploaded: await Promise.all(
        state.tables.map(async ({ scene, tables: { terrain, grass, buildings, scenery } }) => ({
          scene,
          terrain: await encode(terrain),
          grass: { ...grass, tables: await encode(grass.tables) },
          buildings: { ...buildings, tables: await encode(buildings.tables) },
          scenery: { ...scenery, tables: await encode(scenery.tables) },
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
    const tables = uploaded.find((t) => t.scene === map);
    if (!tables) throw new Error(`${map}: the page exported no GPU tables`);
    const decode = (group) =>
      Object.fromEntries(
        Object.entries(group).map(([name, { base64, layout }]) => {
          const bytes = Buffer.from(base64, "base64");
          // A copy: a Buffer's bytes need not start on a word boundary.
          return [name, { words: new Uint32Array(new Uint8Array(bytes).buffer), layout }];
        }),
      );
    const { manifest, artifacts } = await writeVisualPresentation({
      captureJson,
      mapBytes,
      drawn: report.drawnPresentations[i],
      tables: {
        terrain: decode(tables.terrain),
        grass: { ...tables.grass, tables: decode(tables.grass.tables) },
        buildings: { ...tables.buildings, tables: decode(tables.buildings.tables) },
        scenery: { ...tables.scenery, tables: decode(tables.scenery.tables) },
      },
      grassCounts: own.map(({ file, cells }) => ({ plate: file, cells })),
      instanceCounts: own.map(({ file, drawn }) => ({ plate: file, ...drawn })),
      modelCounts: own.map(({ file, models }) => ({ plate: file, ...models })),
      effectCounts: own.map(({ file, effects }) => ({ plate: file, ...effects })),
    });
    await writeFile(new URL("manifest.json", dir), manifest);
    for (const { file, bytes } of artifacts) await writeFile(new URL(file, dir), bytes);
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
    const frames = JSON.parse(captureJson).frames;
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
    const heights = parsed.terrain.find((t) => t.name === "heights").layout;
    const shapes = parsed.grass.tables.find((t) => t.name === "shapes").layout;
    const grown = parsed.grassCounts.map((g) => g.cells.reduce((a, b) => a + b, 0));
    const sum = (list) => list.reduce((a, b) => a + b, 0);
    const rows = (group, name) => parsed[group].tables.find((t) => t.name === name).layout.count;
    const modules = parsed.instanceCounts.map((c) => sum(c.buildings));
    const known = parsed.instanceCounts.map(
      (c) => c.knownProps.map((p) => `${p.appearance} ${sum(p.levels)}`).join(" + ") || "0",
    );
    const scenery = parsed.instanceCounts.map((c) =>
      ["forest", "understorey", "backdrop", "dressing"].map((p) => sum(c.scenery[p])).join("/"),
    );
    const posed = parsed.modelCounts.map((c) =>
      [c.soldiers, c.vehicles, c.aircraft, c.corpses, c.fading].join("/"),
    );
    console.log(
      `METRIC native-parity-capture ${map} units: ${Object.keys(parsed.units.types).length} unit types, ${Object.keys(parsed.units.appearances).length} appearances, ${parsed.units.tables.length} clip tables; soldiers/vehicles/aircraft/corpses/fading per plate ${posed.join(", ")}`,
    );
    console.log(
      `METRIC native-parity-capture ${map}: ${frames.length} frames, ${own.length} plates, heights ${heights.nx}×${heights.ny} at ${heights.spacing} m, grass kinds ${parsed.grass.kinds.join(", ")}, ${shapes.count} shape vertices, grass per plate ${grown.join(", ")}; ${rows("buildings", "buildings")} buildings, ${rows("buildings", "intact")} intact modules of ${parsed.buildings.modules.length} kinds, modules per plate ${modules.join(", ")}; ${["forest", "understorey", "backdrop", "dressing"].map((p) => `${rows("scenery", p)} ${p}`).join(", ")}, forest/understorey/backdrop/dressing per plate ${scenery.join(", ")}; known props per plate ${known.join(", ")} → ${dir.pathname}`,
    );
    const effected = parsed.effectCounts.map(
      (c) => `${sum(Object.values(c.running))}/${sum(Object.values(c.instances))}/${c.lights}`,
    );
    console.log(
      `METRIC native-parity-capture ${map} effects: ${Object.keys(parsed.effects.smokes).length} smoking prop kinds, ${Object.keys(parsed.effects.wrecks).length} wrecks; running/instances/lights per plate ${effected.join(", ")}`,
    );
  }
  console.log(
    `METRIC native-parity-capture: ${report.scenes.length} scene(s) in ${((performance.now() - started) / 1000).toFixed(0)} s → ${runDir.pathname}`,
  );
}
