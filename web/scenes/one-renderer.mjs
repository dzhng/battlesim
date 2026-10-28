// Browser proof for the one-renderer spike. The page runs all ownership
// variants sequentially on the same adapter and publishes the measured rows.
import { mkdir, writeFile } from "node:fs/promises";
import { decode } from "./_png.mjs";

function meanDiff(a, b) {
  if (a.width !== b.width || a.height !== b.height) return Number.POSITIVE_INFINITY;
  let sum = 0;
  for (let i = 0; i < a.data.length; i += 4)
    sum +=
      Math.abs(a.data[i] - b.data[i]) +
      Math.abs(a.data[i + 1] - b.data[i + 1]) +
      Math.abs(a.data[i + 2] - b.data[i + 2]);
  return sum / (a.width * a.height * 3);
}

export async function run(ctx) {
  const capture = process.env.ONE_RENDERER_CAPTURE === "1";
  const dynamic = process.env.ONE_RENDERER_DYNAMIC === "1";
  if (dynamic && !capture) throw new Error("changing-frame comparison requires capture mode");
  const plate = process.env.ONE_RENDERER_PLATE ?? "plate-660.png";
  const evidencePath = (name) => ctx.evidencePath(capture ? `${plate}/${name}` : name);
  if (capture) await mkdir(ctx.evidencePath(plate), { recursive: true });
  const page = await ctx.newPage({ viewport: { width: 1280, height: 2300 } });
  const url = new URL(ctx.url);
  if (capture) {
    url.searchParams.set("capture", "1");
    url.searchParams.set("plate", plate);
  }
  if (process.env.ONE_RENDERER_TIMING === "1") url.searchParams.set("timing", "1");
  if (dynamic) url.searchParams.set("dynamic", "1");
  await page.goto(url.href);
  await page.getByRole("button", { name: "Run comparison" }).waitFor({ timeout: 60_000 });
  await page.getByRole("button", { name: "Run comparison" }).click();
  try {
    await page.waitForFunction(
      (count) => document.querySelectorAll("tbody tr").length === count,
      capture ? 2 : 11,
      {
        timeout: 60_000,
      },
    );
  } catch (error) {
    const stages = await page
      .locator("canvas")
      .evaluateAll((canvases) => canvases.map((canvas) => canvas.dataset.stage ?? null));
    throw new Error(`renderer comparison stalled: ${JSON.stringify(stages)}`, { cause: error });
  }
  const result = await page.evaluate(() =>
    [...document.querySelectorAll("tbody tr")].map((row) =>
      [...row.querySelectorAll("td")].map((cell) => cell.textContent?.trim() ?? ""),
    ),
  );
  await writeFile(evidencePath("result.json"), JSON.stringify(result, null, 2));
  const samples = await page
    .locator("tbody tr")
    .evaluateAll((rows) => rows.flatMap((row) => JSON.parse(row.dataset.samples ?? "[]")));
  await writeFile(evidencePath("samples.json"), JSON.stringify(samples, null, 2));
  const recovery = await page
    .locator("canvas")
    .nth(11)
    .evaluate((canvas) => ({
      before: canvas.dataset.recoveryBefore,
      after: canvas.dataset.recoveryAfter,
    }));
  for (const [phase, data] of Object.entries(recovery)) {
    if (data)
      await writeFile(
        evidencePath(`recovery-${phase}.png`),
        Buffer.from(data.split(",")[1], "base64"),
      );
  }
  await page.screenshot({ path: evidencePath("comparison.png"), fullPage: true });
  if (capture) {
    const browser = await page.locator("canvas").nth(9).screenshot();
    const shared = await page.locator("canvas").nth(11).screenshot();
    await writeFile(evidencePath("captured-browser.png"), browser);
    await writeFile(evidencePath("captured-shared.png"), shared);
    const nativeCaptures = {};
    for (const [name, index] of [
      ["browser", 9],
      ["shared", 11],
    ]) {
      const source = await page
        .locator("canvas")
        .nth(index)
        .evaluate((canvas) => canvas.dataset.finalFrame);
      if (!source)
        throw new Error(`${name} did not retain the drawn capture before device disposal`);
      const bytes = Buffer.from(source.split(",")[1], "base64");
      nativeCaptures[name] = decode(bytes);
      await writeFile(evidencePath(`captured-${name}-native.png`), bytes);
    }
    await writeFile(
      evidencePath("visual.json"),
      JSON.stringify(
        {
          captured_shared_vs_browser: meanDiff(decode(shared), decode(browser)),
          native_shared_vs_browser: meanDiff(nativeCaptures.shared, nativeCaptures.browser),
        },
        null,
        2,
      ),
    );
    ctx.check(
      "captured source pixels match between browser backends",
      meanDiff(nativeCaptures.shared, nativeCaptures.browser) < 1,
      JSON.stringify({
        meanRgbDifference: meanDiff(nativeCaptures.shared, nativeCaptures.browser),
      }),
    );
    const workload = JSON.parse(result[1]?.[5] ?? "null");
    ctx.check(
      "captured controls complete",
      result.every((row) => row[6] === "ok"),
      JSON.stringify(result),
    );
    ctx.check(
      "fresh shared renderer receives a complete fog snapshot",
      workload?.fogInitialUploadComplete === true,
      JSON.stringify(workload),
    );
    ctx.check(
      "shared renderer consumes a fresh fog record each drawn frame",
      workload?.fogSightUploads >= Number(result[1]?.[4]),
      JSON.stringify(workload),
    );
    ctx.check(
      "captured shared renderer admits models and recovers the device",
      workload?.presentationModelRuns > 0 && workload?.deviceRecoveries >= 1,
      JSON.stringify(workload),
    );
    ctx.check(
      "captured frame pixels survive device recreation",
      workload?.recoveryPixelsMatch === true,
      JSON.stringify(workload),
    );
    const browserWorkload = JSON.parse(result[0]?.[5] ?? "null");
    ctx.check(
      dynamic
        ? "changing capture inputs and final state match"
        : "frozen capture prepares large streams once",
      dynamic
        ? browserWorkload?.changing?.frames === 120 &&
            workload?.changing?.frames === 120 &&
            browserWorkload.changing.publications > 1 &&
            JSON.stringify(browserWorkload.changing) === JSON.stringify(workload.changing)
        : workload?.preparation?.modelPreparations === 1 &&
            workload?.preparation?.effectPreparations === 1,
      JSON.stringify({
        browser: browserWorkload?.changing,
        shared: workload?.changing,
        preparation: workload?.preparation,
      }),
    );
    if (dynamic) {
      ctx.check(
        "changing canonical frames prepare new model and effect streams",
        workload?.preparation?.modelPreparations === 121 &&
          workload?.preparation?.effectPreparations === 121,
        JSON.stringify(workload?.preparation),
      );
      ctx.check(
        "changing capture final pixels match between renderers",
        meanDiff(decode(shared), decode(browser)) < 1,
        JSON.stringify({ meanDiff: meanDiff(decode(shared), decode(browser)) }),
      );
    }
    return;
  }
  const canvasShots = [];
  for (let i = 0; i < 12; i++) {
    const shot = await page.locator("canvas").nth(i).screenshot();
    await writeFile(evidencePath(`canvas-${i}.png`), shot);
    canvasShots.push(decode(shot));
  }
  const visual = {
    exported_vs_typegpu: meanDiff(canvasShots[1], canvasShots[0]),
    rust_vs_exported: meanDiff(canvasShots[2], canvasShots[1]),
    real_rust_vs_browser: meanDiff(canvasShots[8], canvasShots[5]),
    model_rust_vs_browser: meanDiff(canvasShots[7], canvasShots[6]),
    battle_rust_vs_browser: meanDiff(canvasShots[10], canvasShots[9]),
  };
  await writeFile(evidencePath("visual.json"), JSON.stringify(visual, null, 2));
  const failures = result.filter((row) => row[6] !== "ok");
  ctx.check("all renderer paths complete", failures.length === 0, JSON.stringify(result));
  ctx.check(
    "each path rendered the requested frames",
    result.every((row) => row[4] === "120"),
    JSON.stringify(result),
  );
  ctx.check(
    "Rust and browser paths keep the same pixels",
    visual.rust_vs_exported < 1,
    JSON.stringify(visual),
  );
  ctx.check(
    "real presentation terrain matches between backends",
    visual.real_rust_vs_browser < 1,
    JSON.stringify(visual),
  );
  ctx.check(
    "authored model workload matches between backends",
    visual.model_rust_vs_browser < 0.1,
    JSON.stringify(visual),
  );
  ctx.check(
    "production BattleFrame and Rust input rows complete",
    result[8]?.[6] === "ok" && result[9]?.[6] === "ok",
    JSON.stringify(result.slice(8)),
  );
  const sharedWorkload = JSON.parse(result[10]?.[5] ?? "null");
  ctx.check(
    "shared browser row reports shared target/depth/world owners",
    sharedWorkload?.sharedTargets === true &&
      sharedWorkload?.sharedDepth === true &&
      sharedWorkload?.sharedWorld === true,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row survives the resize round trip",
    sharedWorkload?.resizes === 2 &&
      sharedWorkload?.width === 640 &&
      sharedWorkload?.height === 360,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row exercises surface recovery",
    sharedWorkload?.surfaceRecoveries >= 1,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row recreates the device and resources",
    sharedWorkload?.deviceLost >= 1 && sharedWorkload?.deviceRecoveries >= 1,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row restores complete frame pixels after device recreation",
    sharedWorkload?.recoveryPixelsMatch === true,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row borrows the frozen presentation records",
    sharedWorkload?.presentationPreparations >= 1 &&
      sharedWorkload?.presentationBound === true &&
      sharedWorkload?.presentationCameraWords === 40 &&
      sharedWorkload?.presentationEnvironmentWords > 0 &&
      sharedWorkload?.presentationFogWords > 0 &&
      sharedWorkload?.presentationOverlayWords === 4,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row owns shadow resources",
    sharedWorkload?.sharedShadow === true &&
      sharedWorkload?.shadowPasses > 0 &&
      sharedWorkload?.shadowModelPasses > 0 &&
      sharedWorkload?.sharedFogSight === true &&
      Array.isArray(sharedWorkload?.shadowCasterBatches) &&
      sharedWorkload.shadowCasterBatches.length === 4,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row owns the environment bind group",
    sharedWorkload?.sharedEnvironment === true,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row executes an admitted presentation model run",
    sharedWorkload?.sharedModelPrepass === true && sharedWorkload?.presentationModelRuns > 0,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row owns fog compute pipelines",
    sharedWorkload?.sharedFogCompute === true,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row consumes current fog sight records",
    sharedWorkload?.fogSightUploads >= 1 && sharedWorkload?.fogHeightWords >= 3,
    JSON.stringify(sharedWorkload),
  );
}
