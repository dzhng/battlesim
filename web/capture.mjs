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
  ffprobe,
  receiveRecording,
  recordingVerdict,
  recordTimeoutMs,
  TAB_CAPTURE_BROWSER,
  videoFrameTimes,
} from "./capture/recording.mjs";

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

/** Play one scene on the route and write its WebM; resolves `{audio}` as
 * ffprobe finds the WebM, or throws a CaptureError. */
async function recordScene(browser, base, job, { width, height }) {
  const page = await openScenePage(browser, { viewport: { width, height } });
  try {
    console.log(`scene ${job.scene}: recording ${job.seconds} s from ${job.url}`);
    const timeoutMs = recordTimeoutMs(job.seconds);
    const { result, bytes } = await receiveRecording(page, {
      url: `${base}${job.url}`,
      file: job.webm,
      timeoutMs,
      label: `scene ${job.scene}`,
      fatal: fatalGpuMessage,
    });
    const verdict = recordingVerdict({ scene: job.scene, result, bytes, timeoutMs });
    if (verdict instanceof Error) throw verdict;
    const audio = streamTypes(job.webm).includes("audio");
    if (!audio) {
      throw new CaptureError(
        "TabCaptureNoAudio",
        `scene ${job.scene}: the WebM has no audio stream`,
      );
    }
    return { audio };
  } finally {
    await page.close();
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

const streamTypes = (file) => ffprobe(file, ["-show_entries", "stream=codec_type"]).split("\n");

const mediaSeconds = (file) => Number(ffprobe(file, ["-show_entries", "format=duration"]));

main().catch((error) => {
  console.error(`capture failed: ${error.name}: ${error.message}`);
  process.exit(1);
});
