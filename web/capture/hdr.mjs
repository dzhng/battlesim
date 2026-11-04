// ffmpeg argv: the native client's HDR frames + WAV → an HLG MP4 that X and
// iPhone play as HDR. The default is 8-bit H.264 High, which X's web uploader
// accepts (it hangs on HEVC); `hevc` gives 10-bit HEVC Main 10.
//
// The frames already hold the HLG signal in BT.2020 primaries (the renderer's
// HDR look encodes it), so ffmpeg only converts RGB to BT.2020 YUV and tags
// the stream; it applies no transfer function of its own.

import { AAC } from "./transcode.mjs";

const HLG_TAGS = [
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
  "-pix_fmt",
  "yuv420p10le",
  ...HLG_TAGS,
  "-tag:v",
  "hvc1",
  "-x265-params",
  "repeat-headers=1",
];
/** X's uploader takes 8-bit High at level 4.2, rate-controlled near 12 Mbps;
 * the x264 params write the HLG VUI into the stream itself. */
const HLG_H264 = [
  "-c:v",
  "libx264",
  "-preset",
  "slow",
  "-profile:v",
  "high",
  "-level:v",
  "4.2",
  "-pix_fmt",
  "yuv420p",
  "-b:v",
  "12M",
  "-maxrate",
  "12M",
  "-bufsize",
  "24M",
  ...HLG_TAGS,
  "-x264-params",
  "colorprim=bt2020:transfer=arib-std-b67:colormatrix=bt2020nc",
];

/** Frames `framePattern` (from 0) at `fps`, fitted into width×height, with
 * `audio` when given. */
export function hdrEncodeArgs({ framePattern, audio, output, fps, width, height, hevc = false }) {
  const vf = [
    `scale=${width}:${height}:force_original_aspect_ratio=decrease:out_color_matrix=bt2020:out_range=tv`,
    // Right after the scale, so its BT.2020 matrix makes the YUV, at the encode's depth.
    `format=${hevc ? "yuv420p10le" : "yuv420p"}`,
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
    ...(audio ? [...AAC, "-ac", "2"] : ["-an"]),
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
 * being HLG in BT.2020 as the encode `hevc` picks writes it (8-bit H.264 High
 * by default, 10-bit HEVC Main 10 with `hevc`); empty when nothing does. */
export function hlgTagErrors(probe, { hevc = false } = {}) {
  const stream = probe?.streams?.[0];
  if (!stream) return ["no video stream"];
  const expected = {
    codec_name: [hevc ? "hevc" : "h264"],
    profile: [hevc ? "Main 10" : "High"],
    pix_fmt: [hevc ? "yuv420p10le" : "yuv420p"],
    color_primaries: ["bt2020"],
    color_space: ["bt2020nc"],
    color_transfer: ["arib-std-b67"],
    color_range: ["tv"],
  };
  return Object.entries(expected)
    .filter(([key, values]) => !values.includes(stream[key]))
    .map(([key, values]) => `${key}: expected ${values.join(" or ")}, got ${stream[key]}`);
}
