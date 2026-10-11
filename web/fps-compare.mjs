// The browser and native clients' frame timing from one run folder, compared
// on the same numbers from the same code (`frameComparison.ts`):
//
//   node fps-compare.mjs <run dir>
//
// It reads every `browser-*.json` the graphics-fps scene wrote and every
// native bench report (`report.json` with a `bench`) below the folder, and
// writes `<run dir>/summary.md`: one row per client and scene, with each
// run's conditions.
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { compareRuns } from "./src/battle/benchmark/frameComparison.ts";

async function* files(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else yield path;
  }
}

/** Each scene of a browser file, and a native bench report, as runs. */
function runsOf(path, json) {
  if (json.client === "browser")
    return json.scenes.map((s) => ({
      client: "browser",
      scene: s.scene,
      intervalsMs: s.intervalsMs,
      cpuMs: s.cpuMs,
      gpuMs: s.gpu?.meanMs ?? null,
      conditions: { ...json.conditions, warmupFrames: s.warmupFrames },
      path,
    }));
  if (json.schema === "godot-render-report" && json.bench)
    return [
      {
        client: "native",
        scene: json.identity.scene,
        intervalsMs: json.bench.intervalsMs,
        cpuMs: json.bench.cpuMs,
        gpuMs: json.bench.gpuMs,
        conditions: {
          ...json.bench.conditions,
          build: `godot ${json.renderer.build}`,
          pace: json.bench.pace,
          warmupFrames: json.bench.warmupFrames,
        },
        path,
      },
    ];
  return [];
}

const fixed = (v, digits = 1) => (v === null || v === undefined ? "n/a" : v.toFixed(digits));

export async function main([dir] = process.argv.slice(2)) {
  if (!dir) throw new Error("usage: node fps-compare.mjs <run dir>");
  const runs = [];
  for await (const path of files(dir))
    if (path.endsWith(".json"))
      runs.push(...runsOf(path, JSON.parse(await readFile(path, "utf8"))));
  if (runs.length === 0) throw new Error(`${dir}: no browser file or native bench report`);
  const rows = compareRuns(runs);
  const lines = [
    "| client | scene | runs | avg FPS (median; min–max) | 1% low | min FPS | frame ms p50 / p95 / p99 / max | CPU ms p50 | GPU ms mean |",
    "|---|---|---|---|---|---|---|---|---|",
    ...rows.map(
      (r) =>
        `| ${r.client} | ${r.scene} | ${r.runs} | ${fixed(r.averageFps.median)} (${fixed(r.averageFps.min)}–${fixed(r.averageFps.max)}) | ${fixed(r.low1Fps)} | ${fixed(r.minimumFps)} | ${[r.frameMs.p50, r.frameMs.p95, r.frameMs.p99, r.frameMs.max].map((v) => fixed(v, 2)).join(" / ")} | ${fixed(r.cpuMsP50, 2)} | ${fixed(r.gpuMsMean, 2)} |`,
    ),
    "",
    "Conditions per run:",
    "",
    ...runs.map((r) => `- ${r.client} ${r.scene}: ${JSON.stringify(r.conditions)} (${r.path})`),
    "",
  ];
  const summary = lines.join("\n");
  await writeFile(join(dir, "summary.md"), summary);
  console.log(summary);
  return 0;
}

main().then(
  (code) => process.exit(code),
  (error) => {
    console.error(error.message);
    process.exit(2);
  },
);
