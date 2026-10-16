/** Cinematic capture mode for recording marketing clips.
 *  @see ./README.md for usage documentation. */

export {
  parseCaptureParams,
  isCaptureMode,
  getCapturePreset,
  reelDuration,
  CAPTURE_PRESETS,
  DEFAULT_CAPTURE,
  DEFAULT_CINEMATIC_REEL,
  type CaptureSettings,
  type CapturePreset,
} from "./captureConfig";

export {
  useCaptureHooks,
  CaptureOverlay,
  exposeCaptureToWindow,
  type CaptureState,
  type CaptureHooks,
} from "./CinematicCapture";
