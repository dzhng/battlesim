// ffmpeg argv: HDR EXR frames + WAV audio → HLG HEVC Main10 MP4.
//
// The native client outputs linear BT.2020 half-float EXR frames. This module
// encodes them to HLG (Hybrid Log-Gamma) for cross-platform compatibility:
// HLG looks correct on both HDR and SDR displays.
//
// Output specs for X/iPhone HDR:
// - libx265 Main 10 profile
// - yuv420p10le pixel format
// - BT.2020 color primaries and matrix
// - ARIB STD-B67 (HLG) transfer function
// - Limited range
// - hvc1 tag for QuickTime/iOS compatibility
// - Repeat headers for robustness

export const HLG_HEVC = [
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
  "-color_primaries",
  "bt2020",
  "-colorspace",
  "bt2020nc",
  "-color_trc",
  "arib-std-b67",
  "-color_range",
  "tv",
  "-tag:v",
  "hvc1",
  "-x265-params",
  "repeat-headers=1",
];

export const HLG_H264 = [
  "-c:v",
  "libx264",
  "-preset",
  "slow",
  "-crf",
  "18",
  "-profile:v",
  "high10",
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

export const AAC = ["-c:a", "aac", "-b:a", "192k", "-ar", "48000"];

/**
 * Build ffmpeg arguments to encode EXR frames to HLG HEVC.
 * @param {Object} options
 * @param {string} options.framePattern - Input frame pattern (e.g., "frames/plate-%04d.exr")
 * @param {string} options.audio - Input audio file (WAV)
 * @param {string} options.output - Output MP4 file
 * @param {number} options.fps - Frame rate
 * @param {number} options.width - Frame width
 * @param {number} options.height - Frame height
 * @param {boolean} [options.hevc=true] - Use HEVC (true) or H.264 (false)
 */
export function hdrEncodeArgs({
  framePattern,
  audio,
  output,
  fps,
  width,
  height,
  hevc = true,
}) {
  const videoCodec = hevc ? HLG_HEVC : HLG_H264;
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
    [
      `scale=${width}:${height}:force_original_aspect_ratio=decrease`,
      `pad=${width}:${height}:(ow-iw)/2:(oh-ih)/2`,
      "setsar=1",
    ].join(","),
    "-fps_mode",
    "cfr",
    "-r",
    String(fps),
    ...videoCodec,
    ...(audio ? AAC : ["-an"]),
    "-movflags",
    "+faststart",
    output,
  ];
}

/**
 * Build ffmpeg probe command to verify HDR metadata.
 * @param {string} file - Video file to probe
 * @returns {string[]} ffprobe arguments
 */
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

/**
 * Verify that the encoded file has correct HDR metadata.
 * @param {Object} probe - Parsed ffprobe JSON output
 * @returns {{valid: boolean, errors: string[]}}
 */
export function verifyHdrMetadata(probe) {
  const errors = [];
  const stream = probe?.streams?.[0];
  if (!stream) {
    return { valid: false, errors: ["no video stream found"] };
  }

  const checks = [
    ["codec_name", ["hevc", "h264"]],
    ["pix_fmt", ["yuv420p10le"]],
    ["color_primaries", ["bt2020"]],
    ["color_space", ["bt2020nc"]],
    ["color_transfer", ["arib-std-b67"]],
    ["color_range", ["tv"]],
  ];

  for (const [key, expected] of checks) {
    const value = stream[key];
    if (!expected.includes(value)) {
      errors.push(`${key}: expected ${expected.join(" or ")}, got ${value}`);
    }
  }

  return { valid: errors.length === 0, errors };
}
