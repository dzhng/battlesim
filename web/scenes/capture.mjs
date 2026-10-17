// The capture route records its stage: the short example capture plays at
// 1080p in headful Chromium with tab capture auto-accepted and sound on, as
// the capture CLI films it, and streams its WebM back through the CLI's
// handoff (`captureChunk`, then `captureDone`). ffprobe must find a VP9 video
// stream at the viewport's size and a stereo Opus stream. The scene owns its
// browser (the CLI's, `TAB_CAPTURE_BROWSER`) because the runner's is headless
// and muted. Mac only, like the CLI.
import { execFileSync } from "node:child_process";
import { open } from "node:fs/promises";
import { launchBrowser } from "../browser.mjs";
import { TAB_CAPTURE_BROWSER } from "../capture/recording.mjs";
import { openScenePage } from "../scene.mjs";

const VIEWPORT = { width: 1920, height: 1080 };
const CAPTURE = "short";
/** The short capture's reel (the menu's first shot), and how long its page may take to start. */
const REEL_S = 8;
const TIMEOUT_MS = 120_000;

export async function run(ctx) {
  const browser = await launchBrowser(TAB_CAPTURE_BROWSER);
  const file = ctx.evidencePath(`${CAPTURE}.webm`);
  const webm = await open(file, "w");
  try {
    const page = await openScenePage(browser, { viewport: VIEWPORT });
    page.on("pageerror", (error) => console.log(`  [capture page error] ${error.message}`));
    let bytes = 0;
    let written = Promise.resolve();
    let finish;
    const done = new Promise((resolve) => (finish = resolve));
    await page.exposeFunction("captureChunk", (base64) => {
      const chunk = Buffer.from(base64, "base64");
      bytes += chunk.length;
      // Resolve to nothing: whatever this returns is serialized back into the
      // page, and `write`'s result holds the chunk itself, a stall per chunk.
      written = written.then(async () => {
        await webm.write(chunk);
      });
      return written;
    });
    await page.exposeFunction("captureDone", (result) => finish(result));
    const url = new URL("/labs/capture", ctx.url);
    url.search = new URLSearchParams({ capture: CAPTURE, scene: "0", cards: "1", record: "1" });
    await page.goto(url.href);
    const timer = setTimeout(
      () => finish({ error: `no captureDone in ${TIMEOUT_MS} ms` }),
      TIMEOUT_MS,
    );
    const result = await done;
    clearTimeout(timer);
    await written;
    await page.close();

    ctx.check(
      "the route recorded without error",
      !result.error,
      result.error ?? JSON.stringify(result),
    );
    if (result.error) return;
    const fps = result.frames / result.seconds;
    ctx.check(
      "the recording lasts the reel",
      Math.abs(result.seconds - REEL_S) < 1.5,
      `${result.seconds.toFixed(2)} s, ${result.frames} frames (${fps.toFixed(1)} fps), worst gap ${Math.round(result.worstGapMs)} ms, ${bytes} bytes`,
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
    await webm.close();
    await browser.close();
  }
}
