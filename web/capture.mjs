#!/usr/bin/env node
// Film a capture file into MP4s: bun run --cwd web capture <file.json> [flags]
// (`--help` is not a flag; any unknown one prints the usage).
//
// Each scene plays on the /labs/capture route in headful Chromium, which
// records its stage, cards and sound with tab capture and streams the WebM
// here. ffmpeg turns each into a constant-rate H.264/AAC MP4, joins them
// through a dip to black when there are several, and `<name>.report.json` says how each
// scene recorded. The pure stages live in capture/; this file is their IO.
//
// Recording is wall-clock real time on the user's Mac: never headless, never
// CI. It starts its own Vite server unless VERIFY_URL names one, and holds the
// repository's GPU lock while recording, so don't wrap it in `lockf` too.
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { open } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { launchBrowser } from "./browser.mjs";
import { ensureNativeTools, fatalGpuMessage, openScenePage, startServer } from "./scene.mjs";
import { parseCaptureArgs, REPO_ROOT } from "./capture/args.mjs";
import { planCapture } from "./capture/plan.mjs";
import { transcodeArgs } from "./capture/transcode.mjs";
import { joinArgs } from "./capture/join.mjs";
import { buildReport, frameStats } from "./capture/report.mjs";
import {
  CaptureError,
  recordingVerdict,
  recordTimeoutMs,
  TAB_CAPTURE_BROWSER,
} from "./capture/recording.mjs";

const PROGRESS_MS = 10_000;
const fromRoot = (file) => relative(REPO_ROOT, file);

async function main() {
  const args = parseCaptureArgs(process.argv.slice(2));
  const capture = JSON.parse(readFileSync(args.file, "utf8"));
  const plan = planCapture(capture, args);
  for (const tool of ["ffmpeg", "ffprobe"]) {
    try {
      execFileSync(tool, ["-version"], { stdio: "ignore" });
    } catch {
      throw new CaptureError("FfmpegMissing", `${tool} is not on PATH`);
    }
  }
  mkdirSync(args.output, { recursive: true });
  console.log(
    `capture ${plan.name}: ${plan.jobs.length} scene(s) at ${args.width}x${args.height} ${args.fps} fps, cards ${args.cards ? "on" : "off"}`,
  );

  await ensureNativeTools();
  const server = await startServer(false);
  const recordings = [];
  try {
    const release = await holdGpuLock();
    const browser = await launchBrowser(TAB_CAPTURE_BROWSER);
    try {
      for (const job of plan.jobs)
        recordings.push(await recordScene(browser, server.url, job, args));
    } finally {
      await browser.close();
      release();
    }
  } finally {
    await server.close();
  }

  const outputs = plan.jobs.map((job, i) => {
    console.log(`scene ${job.scene}: transcoding to ${fromRoot(job.mp4)}`);
    ffmpeg(transcodeArgs({ ...args, input: job.webm, output: job.mp4 }));
    return {
      scene: job.scene,
      file: fromRoot(job.mp4),
      seconds: mediaSeconds(job.mp4),
      ...frameStats(videoFrameTimes(job.webm)),
      audio: recordings[i].audio,
    };
  });
  if (plan.joined) {
    console.log(`joining ${outputs.length} scenes into ${fromRoot(plan.joined)}`);
    ffmpeg(
      joinArgs({
        inputs: plan.jobs.map((job, i) => ({ file: job.mp4, seconds: outputs[i].seconds })),
        output: plan.joined,
        fps: args.fps,
      }),
    );
  }
  const report = buildReport({
    capture: fromRoot(args.file),
    fps: args.fps,
    outputs,
    joined: plan.joined && fromRoot(plan.joined),
  });
  writeFileSync(plan.report, `${JSON.stringify(report, null, 2)}\n`);

  for (const output of outputs) {
    console.log(
      `  ${output.file}  ${output.seconds.toFixed(1)} s  avg ${output.avgFps.toFixed(1)} fps  worst gap ${Math.round(output.worstGapMs)} ms`,
    );
  }
  if (report.joined) console.log(`  ${report.joined}  (joined)`);
  for (const warning of report.warnings) console.warn(`  warning: ${warning}`);
  console.log(`report: ${fromRoot(plan.report)}`);
}

/** Play one scene on the route and write its WebM; resolves with the route's
 * `{frames, worstGapMs, seconds, audio}` or throws a CaptureError. */
async function recordScene(browser, base, job, { width, height }) {
  const page = await openScenePage(browser, { viewport: { width, height } });
  const webm = await open(job.webm, "w");
  let bytes = 0;
  let written = Promise.resolve();
  let finish;
  const finished = new Promise((resolve) => (finish = resolve));
  const timeoutMs = recordTimeoutMs(job.seconds);
  const timer = setTimeout(() => finish(undefined), timeoutMs);
  const started = Date.now();
  const progress = setInterval(() => {
    const s = Math.round((Date.now() - started) / 1000);
    console.log(`  scene ${job.scene}: ${s} s, ${(bytes / 1e6).toFixed(1)} MB recorded`);
  }, PROGRESS_MS);
  try {
    // In play order: each write waits for the one before it. The page's
    // promise resolves to nothing: a write's result holds the chunk, which
    // Playwright would serialize back into the page byte by byte.
    await page.exposeFunction("captureChunk", async (base64) => {
      const chunk = Buffer.from(base64, "base64");
      bytes += chunk.length;
      written = written.then(() => webm.write(chunk));
      await written;
    });
    await page.exposeFunction("captureDone", (result) => finish(result));
    page.on("crash", () => finish({ error: "the page crashed" }));
    page.on("pageerror", (error) => console.error(`  [page error] ${error.message}`));
    page.on("console", (message) => {
      if (message.type() !== "error") return;
      console.error(`  [page] ${message.text()}`);
      if (fatalGpuMessage(message.text())) finish({ error: message.text() });
    });
    console.log(`scene ${job.scene}: recording ${job.seconds} s from ${job.url}`);
    await page.goto(`${base}${job.url}`);
    const result = await finished;
    await written;
    const verdict = recordingVerdict({ scene: job.scene, result, bytes, timeoutMs });
    if (verdict instanceof Error) throw verdict;
    if (!streamTypes(job.webm).includes("audio")) {
      throw new CaptureError(
        "TabCaptureNoAudio",
        `scene ${job.scene}: the WebM has no audio stream`,
      );
    }
    return verdict;
  } finally {
    clearTimeout(timer);
    clearInterval(progress);
    await page.close();
    await webm.close();
  }
}

/** Take the repository's GPU lock (shared by every checkout's rendering jobs)
 * in a child that holds it until released or this process dies. */
async function holdGpuLock() {
  const common = execFileSync("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  }).trim();
  const lock = join(dirname(common), "throwaway", "gpu.lock");
  mkdirSync(dirname(lock), { recursive: true });
  const [tool, ...flags] = process.platform === "darwin" ? ["lockf", "-k", lock] : ["flock", lock];
  const holder = spawn(
    tool,
    [...flags, process.execPath, "-e", 'console.log("held"); process.stdin.resume()'],
    { stdio: ["pipe", "pipe", "inherit"] },
  );
  const waiting = setTimeout(() => console.log(`waiting for the GPU lock ${lock}`), 1000);
  try {
    await new Promise((resolve, reject) => {
      holder.stdout.once("data", resolve);
      holder.once("error", reject);
      holder.once("exit", (code) => reject(new Error(`${tool} exited with ${code}`)));
    });
  } finally {
    clearTimeout(waiting);
  }
  return () => holder.stdin.end();
}

function ffmpeg(argv) {
  execFileSync("ffmpeg", argv, { stdio: ["ignore", "inherit", "inherit"] });
}

function ffprobe(file, entries) {
  return execFileSync("ffprobe", ["-v", "error", ...entries, "-of", "csv=p=0", file], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
}

const streamTypes = (file) => ffprobe(file, ["-show_entries", "stream=codec_type"]).split("\n");

const mediaSeconds = (file) => Number(ffprobe(file, ["-show_entries", "format=duration"]));

/** Presentation times of the recorded video frames, ascending. */
function videoFrameTimes(file) {
  return ffprobe(file, ["-select_streams", "v:0", "-show_entries", "packet=pts_time"])
    .split("\n")
    .filter(Boolean)
    .map(Number)
    .filter(Number.isFinite)
    .sort((a, b) => a - b);
}

main().catch((error) => {
  console.error(`capture failed: ${error.name}: ${error.message}`);
  process.exit(1);
});
