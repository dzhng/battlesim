/** Cinematic capture mode: records the game viewport with scripted camera paths.
 *  Activated by URL param ?capture=<preset>. Uses MediaRecorder for video/audio. */

import type { WeaponFireEvent } from "./captureConfig";

/** State of the capture recording. */
export type CaptureState =
  | { status: "idle" }
  | { status: "warming"; message: string }
  | { status: "recording"; elapsed: number; duration: number }
  | { status: "encoding" }
  | { status: "complete"; blob: Blob; filename: string }
  | { status: "error"; message: string };

/** Overlay UI showing capture progress and status. */
export function CaptureOverlay({
  state,
  onDownload,
  hidden,
}: {
  state: CaptureState;
  onDownload?: () => void;
  /** Hide the overlay (used in page capture mode where CLI shows progress). */
  hidden?: boolean;
}) {
  if (state.status === "idle" || hidden) return null;

  return (
    <div className="capture-overlay" data-testid="capture-overlay">
      {state.status === "warming" && (
        <div className="capture-status capture-warming">
          <span className="capture-icon">⏳</span>
          <span>{state.message}</span>
        </div>
      )}
      {state.status === "recording" && (
        <div className="capture-status capture-recording">
          <span className="capture-icon capture-pulse">🔴</span>
          <span>
            Recording: {state.elapsed.toFixed(1)}s / {state.duration.toFixed(1)}s
          </span>
        </div>
      )}
      {state.status === "encoding" && (
        <div className="capture-status capture-encoding">
          <span className="capture-icon">⚙️</span>
          <span>Encoding video...</span>
        </div>
      )}
      {state.status === "complete" && (
        <div className="capture-status capture-complete">
          <span className="capture-icon">✅</span>
          <span>Capture complete: {state.filename}</span>
          {onDownload && (
            <button type="button" onClick={onDownload}>
              Download
            </button>
          )}
        </div>
      )}
      {state.status === "error" && (
        <div className="capture-status capture-error">
          <span className="capture-icon">❌</span>
          <span>{state.message}</span>
        </div>
      )}
    </div>
  );
}

/** Extra data exposed to window for page-level capture mode. */
export interface CaptureExtras {
  /** Total duration of the capture in seconds. */
  duration?: number;
  /** Audio-only blob (for page mode where video is captured via CDP). */
  audioBlob?: Blob;
  /** Start audio-only recording (for page mode). */
  startAudioOnly?: () => void;
  /** Stop audio recording (for deterministic mode). */
  stopAudio?: () => void;
  /** Weapon fire events logged during capture. */
  fireEvents?: WeaponFireEvent[];
  /** The resolved follow unit ID (real unit ID, not index). */
  followUnitId?: number | null;
  /** Timestamp when the reel's first pose was applied and rendered (for dropping early frames). */
  reelReadyTime?: number | null;
  /** Timestamp when the reel ended (sample.done became true), for precise cut timing. */
  reelEndTime?: number | null;
  /** Timestamp when audio recording started (performance.now() for sync calculation). */
  audioStartTime?: number | null;
  /** Audio output latency in seconds (outputLatency + baseLatency from AudioContext). */
  audioLatency?: number;
  /** Trigger calibration burst (count flash/click pairs at intervalMs apart).
   *  Returns array of performance.now() timestamps for each flash. */
  triggerCalibrationBurst?: (count: number, intervalMs: number) => Promise<number[]>;
  /** Timestamps of end-of-reel calibration bursts (performance.now()). */
  endCalibrationTimes?: number[];
  /** [Deterministic mode] Step the render to a specific virtual time (ms).
   *  Returns a promise that resolves after the frame is rendered. */
  stepFrame?: (virtualTimeMs: number) => Promise<void>;
  /** [Deterministic mode] Enter deterministic mode - use virtual clock for timing. */
  enterDeterministicMode?: () => void;
  /** [Deterministic mode] Get recorded audio commands for offline rendering. */
  getRecordedAudioCommands?: () => unknown[];
  /** [Deterministic mode] Get audio rendering parameters for offline rendering. */
  getAudioRenderParams?: () => {
    sampleRate: number;
    presentation: unknown;
    catalog: unknown;
  } | null;
  /** [Deterministic mode] Render recorded audio offline and return as WAV blob. */
  renderOfflineAudio?: (durationS: number) => Promise<Blob | null>;
  /** [Deterministic mode] Get current state for diagnostics. */
  getDeterministicState?: () => { tick: number; frame: number; virtualTimeMs: number } | null;
}

/** Expose capture results to window for CLI automation. */
export function exposeCaptureToWindow(
  state: CaptureState,
  download: () => void,
  extras?: CaptureExtras,
) {
  (window as Window & { __capture?: unknown }).__capture = {
    state: state.status,
    download: state.status === "complete" ? download : undefined,
    blob: state.status === "complete" ? state.blob : undefined,
    filename: state.status === "complete" ? state.filename : undefined,
    error: state.status === "error" ? state.message : undefined,
    duration: extras?.duration,
    audioBlob: extras?.audioBlob,
    startAudioOnly: extras?.startAudioOnly,
    stopAudio: extras?.stopAudio,
    fireEvents: extras?.fireEvents,
    followUnitId: extras?.followUnitId,
    reelReadyTime: extras?.reelReadyTime,
    reelEndTime: extras?.reelEndTime,
    audioStartTime: extras?.audioStartTime,
    audioLatency: extras?.audioLatency,
    triggerCalibrationBurst: extras?.triggerCalibrationBurst,
    endCalibrationTimes: extras?.endCalibrationTimes,
    stepFrame: extras?.stepFrame,
    enterDeterministicMode: extras?.enterDeterministicMode,
    getRecordedAudioCommands: extras?.getRecordedAudioCommands,
    getAudioRenderParams: extras?.getAudioRenderParams,
    renderOfflineAudio: extras?.renderOfflineAudio,
    getDeterministicState: extras?.getDeterministicState,
  };
}
