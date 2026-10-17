// ffmpeg argv: scene MP4s → one MP4 chained with xfade (video) and
// acrossfade (audio). The picture dips through black, as the reel's own cuts
// do: scenes are unrelated shots, and a plain dissolve between them reads as
// a muddy double exposure. Inputs are transcode outputs, so they already share
// size, frame rate, pixel format and sample rate, which xfade requires.
import { AAC, X264 } from "./transcode.mjs";

/** The dip between scenes, in seconds: long enough to read as a cut,
 * short enough not to eat either scene's last or first beat. */
export const CROSSFADE_S = 0.5;

/** Millisecond-rounded, so float noise never reaches the filter graph. */
const ms = (seconds) => Math.round(seconds * 1000) / 1000;

/** Start time of each transition in the joined timeline: transition k begins
 * CROSSFADE_S before the running length of scenes 0..k ends. */
export function joinOffsets(durations) {
  durations.forEach((seconds, i) => {
    if (!(seconds > CROSSFADE_S)) {
      throw new Error(`scene ${i} is ${seconds}s, not longer than the ${CROSSFADE_S}s crossfade`);
    }
  });
  const offsets = [];
  let length = durations[0];
  for (let k = 1; k < durations.length; k++) {
    offsets.push(ms(length - CROSSFADE_S));
    length += durations[k] - CROSSFADE_S;
  }
  return offsets;
}

/** `inputs` are `{file, seconds}` in play order, with each file's measured
 * length (the offsets depend on it). Needs at least two. */
export function joinArgs({ inputs, output, fps }) {
  if (inputs.length < 2) throw new Error("join needs at least two scenes");
  const offsets = joinOffsets(inputs.map((input) => input.seconds));
  const graph = [];
  let video = "0:v";
  let audio = "0:a";
  offsets.forEach((offset, k) => {
    const i = k + 1;
    graph.push(
      `[${video}][${i}:v]xfade=transition=fadeblack:duration=${CROSSFADE_S}:offset=${offset}[v${i}]`,
      `[${audio}][${i}:a]acrossfade=d=${CROSSFADE_S}[a${i}]`,
    );
    video = `v${i}`;
    audio = `a${i}`;
  });
  return [
    "-y",
    "-hide_banner",
    "-loglevel",
    "error",
    ...inputs.flatMap((input) => ["-i", input.file]),
    "-filter_complex",
    graph.join(";"),
    "-map",
    `[${video}]`,
    "-map",
    `[${audio}]`,
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
