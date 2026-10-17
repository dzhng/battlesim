// ffmpeg argv: a recorded VP9/Opus WebM → constant-frame-rate H.264/AAC MP4.
//
// MediaRecorder writes variable-rate frames at whatever the page delivered;
// `fps` + `-fps_mode cfr` resample them onto the target grid (duplicating a
// frame where the page dropped one), so speed and sync follow wall-clock time.
// The crop already matches the stage, so scale/pad only fixes an odd pixel.

/** x264 at `slow`/CRF 18: visually lossless for marketing footage, well
 * inside a social upload's bitrate limits, and a slower preset only buys size. */
export const X264 = ["-c:v", "libx264", "-preset", "slow", "-crf", "18", "-profile:v", "high"];
export const AAC = ["-c:a", "aac", "-b:a", "192k", "-ar", "48000"];

/** `-vf` chain fitting any input into exactly width×height at `fps`. */
export function frameFilter({ width, height, fps }) {
  return [
    `scale=${width}:${height}:force_original_aspect_ratio=decrease`,
    `pad=${width}:${height}:(ow-iw)/2:(oh-ih)/2`,
    "setsar=1",
    `fps=${fps}`,
  ].join(",");
}

export function transcodeArgs({ input, output, width, height, fps }) {
  return [
    "-y",
    "-hide_banner",
    "-loglevel",
    "error",
    "-i",
    input,
    "-vf",
    frameFilter({ width, height, fps }),
    "-fps_mode",
    "cfr",
    "-r",
    String(fps),
    ...X264,
    "-pix_fmt",
    "yuv420p",
    ...AAC,
    "-movflags",
    "+faststart",
    output,
  ];
}
