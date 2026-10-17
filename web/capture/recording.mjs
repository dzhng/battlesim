// How long one scene recording may take, and what its outcome means.
//
// The route streams the WebM through `captureChunk` and ends with
// `captureDone(result)`. A recording that never ends is a hang the CLI must
// name, never wait out: `--auto-accept-this-tab-capture` is the only thing
// standing between the page and a permission prompt nobody will answer.

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

/** The scene's recording `{frames, worstGapMs, seconds, audio}`, or a
 * CaptureError: `result` is what `captureDone` sent (undefined on timeout),
 * `bytes` how much WebM arrived. */
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
  if (!result.audio) return new CaptureError("TabCaptureNoAudio", `${at}: no audio track`);
  if (bytes === 0) return new CaptureError("TabCaptureEmpty", `${at}: finished with no video`);
  return result;
}
