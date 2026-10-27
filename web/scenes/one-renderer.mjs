// Browser proof for the one-renderer spike. The page runs all ownership
// variants sequentially on the same adapter and publishes the measured rows.
import { writeFile } from "node:fs/promises";
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
  const page = await ctx.newPage({ viewport: { width: 1280, height: 2300 } });
  await page.goto(ctx.url);
  await page.getByRole("button", { name: "Run comparison" }).waitFor({ timeout: 60_000 });
  await page.getByRole("button", { name: "Run comparison" }).click();
  await page.waitForFunction(() => document.querySelectorAll("tbody tr").length === 11, undefined, {
    timeout: 60_000,
  });
  const result = await page.evaluate(() =>
    [...document.querySelectorAll("tbody tr")].map((row) =>
      [...row.querySelectorAll("td")].map((cell) => cell.textContent?.trim() ?? ""),
    ),
  );
  await writeFile(ctx.evidencePath("result.json"), JSON.stringify(result, null, 2));
  await page.screenshot({ path: ctx.evidencePath("comparison.png"), fullPage: true });
  const canvasShots = [];
  for (let i = 0; i < 12; i++) {
    const shot = await page.locator("canvas").nth(i).screenshot();
    await writeFile(ctx.evidencePath(`canvas-${i}.png`), shot);
    canvasShots.push(decode(shot));
  }
  const visual = {
    exported_vs_typegpu: meanDiff(canvasShots[1], canvasShots[0]),
    rust_vs_exported: meanDiff(canvasShots[2], canvasShots[1]),
    real_rust_vs_browser: meanDiff(canvasShots[8], canvasShots[5]),
    model_rust_vs_browser: meanDiff(canvasShots[7], canvasShots[6]),
    battle_rust_vs_browser: meanDiff(canvasShots[10], canvasShots[9]),
  };
  await writeFile(ctx.evidencePath("visual.json"), JSON.stringify(visual, null, 2));
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
    "shared browser row uploads the frozen presentation records",
    sharedWorkload?.presentationUploads >= 1 &&
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
    "shared browser row executes the exported model prepass",
    sharedWorkload?.sharedModelPrepass === true,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row owns fog compute pipelines",
    sharedWorkload?.sharedFogCompute === true,
    JSON.stringify(sharedWorkload),
  );
  ctx.check(
    "shared browser row uploads frozen fog sight records",
    sharedWorkload?.fogSightUploads >= 1,
    JSON.stringify(sharedWorkload),
  );
}
