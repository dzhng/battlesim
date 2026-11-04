// ffmpeg argv: the native client's HDR frames + WAV → HLG HEVC Main 10 MP4
// (or H.264 High 10), the format X and iPhone play as HDR.
//
// The frames already hold the HLG signal in BT.2020 primaries (the renderer's
// HDR look encodes it), so ffmpeg only converts RGB to 10-bit BT.2020 YUV and
// tags the stream; it applies no transfer function of its own.

import { AAC } from "./transcode.mjs";

const HLG_TAGS = [
  "-pix_fmt",
  "yuv420p10le",
  "-color_primaries",
  "bt2020",
  "-colorspace",
  "bt2020nc",
  "-color_trc",
  "arib-std-b67",
  "-color_range",
  "tv",
];
/** `hvc1` and repeated headers are what QuickTime and iOS need to play HEVC. */
const HLG_HEVC = [
  "-c:v",
  "libx265",
  "-preset",
  "slow",
  "-crf",
  "18",
  "-profile:v",
  "main10",
  ...HLG_TAGS,
  "-tag:v",
  "hvc1",
  "-x265-params",
  "repeat-headers=1",
];
const HLG_H264 = [
  "-c:v",
  "libx264",
  "-preset",
  "slow",
  "-crf",
  "18",
  "-profile:v",
  "high10",
  ...HLG_TAGS,
];

/** Frames `framePattern` (from 0) at `fps`, fitted into width×height, with
 * `audio` when given. */
export function hdrEncodeArgs({ framePattern, audio, output, fps, width, height, hevc = true }) {
  const vf = [
    `scale=${width}:${height}:force_original_aspect_ratio=decrease:out_color_matrix=bt2020:out_range=tv`,
    `pad=${width}:${height}:(ow-iw)/2:(oh-ih)/2`,
    "setsar=1",
    "setparams=color_primaries=bt2020:color_trc=arib-std-b67:colorspace=bt2020nc",
  ].join(",");
  return [
    "-y",
    "-hide_banner",
    "-loglevel",
    "error",
    "-framerate",
    String(fps),
    "-i",
    framePattern,
    ...(audio ? ["-i", audio] : []),
    "-vf",
    vf,
    "-fps_mode",
    "cfr",
    "-r",
    String(fps),
    ...(hevc ? HLG_HEVC : HLG_H264),
    ...(audio ? AAC : ["-an"]),
    "-movflags",
    "+faststart",
    output,
  ];
}

/** ffprobe argv reporting the first video stream's codec and colour tags as JSON. */
export function hdrProbeArgs(file) {
  return [
    "-v",
    "error",
    "-select_streams",
    "v:0",
    "-show_entries",
    "stream=codec_name,profile,pix_fmt,color_primaries,color_space,color_transfer,color_range,width,height",
    "-of",
    "json",
    file,
  ];
}

/** What in an ffprobe report (`hdrProbeArgs`) keeps its first stream from
 * being 10-bit HLG in BT.2020; empty when nothing does. */
export function hlgTagErrors(probe) {
  const stream = probe?.streams?.[0];
  if (!stream) return ["no video stream"];
  const expected = {
    codec_name: ["hevc", "h264"],
    pix_fmt: ["yuv420p10le"],
    color_primaries: ["bt2020"],
    color_space: ["bt2020nc"],
    color_transfer: ["arib-std-b67"],
    color_range: ["tv"],
  };
  return Object.entries(expected)
    .filter(([key, values]) => !values.includes(stream[key]))
    .map(([key, values]) => `${key}: expected ${values.join(" or ")}, got ${stream[key]}`);
}
