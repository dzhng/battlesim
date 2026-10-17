// The capture route records its stage: the short example capture plays at
// 1080p in headful Chromium with tab capture auto-accepted and sound on, as
// the capture CLI films it, and streams its WebM back through the CLI's own
// receiver (`receiveRecording`). ffprobe must find a VP9 video stream at the
// viewport's size and a stereo Opus stream, and measures the frame rate as the
// CLI's report does. The scene owns its browser (the CLI's,
// `TAB_CAPTURE_BROWSER`) because the runner's is headless and muted. Mac only,
// like the CLI.
import { execFileSync } from "node:child_process";
import { launchBrowser } from "../browser.mjs";
import {
  receiveRecording,
  recordingVerdict,
  recordTimeoutMs,
  TAB_CAPTURE_BROWSER,
  videoFrameTimes,
} from "../capture/recording.mjs";
import { frameStats } from "../capture/report.mjs";
import { fatalGpuMessage, openScenePage } from "../scene.mjs";

const VIEWPORT = { width: 1920, height: 1080 };
const CAPTURE = "short";
/** The short capture's reel: the menu's first shot. */
const REEL_S = 8;

export async function run(ctx) {
  const browser = await launchBrowser(TAB_CAPTURE_BROWSER);
  try {
    const file = ctx.evidencePath(`${CAPTURE}.webm`);
    const url = new URL("/labs/capture", ctx.url);
    url.search = new URLSearchParams({ capture: CAPTURE, scene: "0", cards: "1", record: "1" });
    const timeoutMs = recordTimeoutMs(REEL_S);
    const page = await openScenePage(browser, { viewport: VIEWPORT });
    const { result, bytes } = await receiveRecording(page, {
      url: url.href,
      file,
      timeoutMs,
      label: CAPTURE,
      fatal: fatalGpuMessage,
    }).finally(() => page.close());

    const verdict = recordingVerdict({ scene: 0, result, bytes, timeoutMs });
    const failed = verdict instanceof Error;
    ctx.check(
      "the route recorded without error",
      !failed,
      failed ? `${verdict.name}: ${verdict.message}` : JSON.stringify(verdict),
    );
    if (failed) return;
    const { avgFps, worstGapMs } = frameStats(videoFrameTimes(file));
    ctx.check(
      "the recording lasts the reel",
      Math.abs(verdict.seconds - REEL_S) < 1.5,
      `${verdict.seconds.toFixed(2)} s, avg ${avgFps.toFixed(1)} fps, worst gap ${Math.round(worstGapMs)} ms, ${bytes} bytes`,
    );
    const streams = JSON.parse(
      execFileSync(
        "ffprobe",
        [
          "-v",
          "error",
          "-show_entries",
          "stream=codec_type,codec_name,width,height,channels,sample_rate",
          "-of",
          "json",
          file,
        ],
        { encoding: "utf8" },
      ),
    ).streams;
    const video = streams.find((s) => s.codec_type === "video");
    const audio = streams.find((s) => s.codec_type === "audio");
    ctx.check(
      "a VP9 video stream at the viewport's size",
      video?.codec_name === "vp9" &&
        video.width === VIEWPORT.width &&
        video.height === VIEWPORT.height,
      JSON.stringify(video),
    );
    ctx.check(
      "a stereo Opus audio stream",
      audio?.codec_name === "opus" && audio.channels === 2,
      JSON.stringify(audio),
    );
  } finally {
    await browser.close();
  }
}
