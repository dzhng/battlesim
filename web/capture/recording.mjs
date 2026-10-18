// Receiving one scene recording from the capture route, how long it may take,
// and what its outcome means.
//
// The route streams the WebM through `captureChunk` and ends with
// `captureDone(result)`. A recording that never ends is a hang the CLI must
// name, never wait out: `--auto-accept-this-tab-capture` is the only thing
// standing between the page and a permission prompt nobody will answer.
import { execFileSync } from "node:child_process";
import { open } from "node:fs/promises";

const PROGRESS_MS = 10_000;

/** The browser a recording runs in (`launchBrowser`'s options): headful, the
 * tab capture granted without a prompt, autoplay allowed, never throttled in
 * the background, and the tab's own sound, which Playwright mutes by default,
 * left on. */
export const TAB_CAPTURE_BROWSER = {
  args: [
    "--auto-accept-this-tab-capture",
    "--autoplay-policy=no-user-gesture-required",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-renderer-backgrounding",
  ],
  launch: { headless: false, ignoreDefaultArgs: ["--mute-audio"] },
};

/** Startup before the reel's first frame: the page, the battle's assets, the
 * warm-up simulation and the stage. A cold dev server takes about 15 s. */
export const STARTUP_ALLOWANCE_S = 180;

export function recordTimeoutMs(seconds) {
  return (seconds + STARTUP_ALLOWANCE_S) * 1000;
}

/** A failure the CLI exits on, named so a log reader knows which one. */
export class CaptureError extends Error {
  constructor(name, message) {
    super(message);
    this.name = name;
  }
}

/** Open `url` on `page` and write the WebM the route hands off to `file`, in
 * play order: each write waits for the one before it. Resolves `{result, bytes}`
 * once the last write has landed. `result` is what `captureDone` sent,
 * `{error}` when the page crashed or logged a console error `fatal` accepts,
 * or undefined after `timeoutMs`. */
export async function receiveRecording(page, { url, file, timeoutMs, label, fatal = () => false }) {
  const webm = await open(file, "w");
  let bytes = 0;
  let written = Promise.resolve();
  let finish;
  const finished = new Promise((resolve) => (finish = resolve));
  const timer = setTimeout(() => finish(undefined), timeoutMs);
  const started = Date.now();
  const progress = setInterval(() => {
    const s = Math.round((Date.now() - started) / 1000);
    console.log(`  ${label}: ${s} s, ${(bytes / 1e6).toFixed(1)} MB recorded`);
  }, PROGRESS_MS);
  try {
    // The page's promise resolves to nothing: a write's result holds the
    // chunk, which Playwright would serialize back into the page byte by byte.
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
      if (fatal(message.text())) finish({ error: message.text() });
    });
    await page.goto(url);
    const result = await finished;
    await written;
    return { result, bytes };
  } finally {
    clearTimeout(timer);
    clearInterval(progress);
    await webm.close();
  }
}

/** The scene's recording `{seconds}`, or a CaptureError: `result` is what
 * `captureDone` sent (undefined on timeout), `bytes` how much WebM arrived.
 * Whether the WebM has sound is ffprobe's to say, not the page's. */
export function recordingVerdict({ scene, result, bytes, timeoutMs }) {
  const at = `scene ${scene}`;
  if (!result && bytes === 0) {
    return new CaptureError(
      "TabCaptureNotStarted",
      `${at}: no recording after ${timeoutMs / 1000} s. Chromium did not auto-accept ` +
        "the tab capture (--auto-accept-this-tab-capture), or a prompt appeared",
    );
  }
  if (!result) {
    return new CaptureError(
      "TabCaptureTimeout",
      `${at}: the recording did not finish within ${timeoutMs / 1000} s (${bytes} bytes arrived)`,
    );
  }
  if (result.error) return new CaptureError("TabCaptureFailed", `${at}: ${result.error}`);
  if (bytes === 0) return new CaptureError("TabCaptureEmpty", `${at}: finished with no video`);
  return result;
}

/** ffprobe's `entries` of `file`, as CSV without headers. */
export function ffprobe(file, entries) {
  return execFileSync("ffprobe", ["-v", "error", ...entries, "-of", "csv=p=0", file], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
}

/** Presentation times of the recorded video frames, ascending. */
export function videoFrameTimes(file) {
  return ffprobe(file, ["-select_streams", "v:0", "-show_entries", "packet=pts_time"])
    .split("\n")
    .filter(Boolean)
    .map(Number)
    .filter(Number.isFinite)
    .sort((a, b) => a - b);
}
