/** Cinematic capture mode: records the game viewport with scripted camera paths.
 *  Activated by URL param ?capture=<preset>. Uses MediaRecorder for video/audio. */

import { useEffect, useRef, useState, type RefObject } from "react";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type { ViewportPilot } from "../LabViewport";
import { sampleReel, type MenuReel } from "../menuReel";
import {
  parseCaptureParams,
  getCapturePreset,
  reelDuration,
  type CaptureSettings,
} from "./captureConfig";

/** State of the capture recording. */
export type CaptureState =
  | { status: "idle" }
  | { status: "warming"; message: string }
  | { status: "recording"; elapsed: number; duration: number }
  | { status: "encoding" }
  | { status: "complete"; blob: Blob; filename: string }
  | { status: "error"; message: string };

/** Hooks for integrating capture mode with a battle or scene. */
export interface CaptureHooks {
  /** Whether capture mode is active. */
  active: boolean;
  /** The capture settings (null when not in capture mode). */
  settings: CaptureSettings | null;
  /** Current capture state. */
  state: CaptureState;
  /** The reel being used for the camera path. */
  reel: MenuReel | null;
  /** Create a ViewportPilot for the capture camera. */
  createPilot: (
    unitAt: (id: number) => ArrayLike<number> | null,
    onEnd: () => void,
  ) => ViewportPilot;
  /** Start recording (call after viewport is ready). */
  startRecording: (canvas: HTMLCanvasElement, audioStream?: MediaStream | null) => void;
  /** Stop recording early. */
  stopRecording: () => void;
}

/** Create capture hooks for integrating with a battle scene.
 *  The hooks are inert (no-ops) when capture mode is not active,
 *  so they can be unconditionally included in components. */
export function useCaptureHooks(): CaptureHooks {
  const settingsRef = useRef<CaptureSettings | null>(null);
  const [state, setState] = useState<CaptureState>({ status: "idle" });
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const startTimeRef = useRef<number>(0);
  const reelRef = useRef<MenuReel | null>(null);

  useEffect(() => {
    settingsRef.current = parseCaptureParams(window.location.search);
    if (settingsRef.current) {
      const preset = getCapturePreset(settingsRef.current.preset);
      reelRef.current = preset.reel;
      setState({ status: "warming", message: "Waiting for scene to warm up..." });
    }
  }, []);

  const createPilot = (
    unitAt: (id: number) => ArrayLike<number> | null,
    onEnd: () => void,
  ): ViewportPilot => {
    const reel = reelRef.current;
    if (!reel) {
      return {
        pose: () => null,
      };
    }

    let startTime: number | null = null;
    let ended = false;

    const trackLag = 0.5;
    let tracked: { follow: number; at: [number, number]; now: number } | null = null;
    let lastSubject: readonly [number, number] = [0, 0];

    const track = (follow: number, now: number): [number, number] | null => {
      const live = unitAt(follow);
      if (tracked?.follow !== follow) {
        tracked = live ? { follow, at: [live[0], live[1]], now } : null;
      }
      if (!tracked) return null;
      if (live) {
        const k = 1 - Math.exp(-(now - tracked.now) / 1000 / trackLag);
        tracked.at = [
          tracked.at[0] + (live[0] - tracked.at[0]) * k,
          tracked.at[1] + (live[1] - tracked.at[1]) * k,
        ];
      }
      tracked.now = now;
      return tracked.at;
    };

    return {
      pose(now) {
        if (startTime === null) startTime = now;
        const elapsed = (now - startTime) / 1000;
        const sample = sampleReel(reel, elapsed);

        if (sample.done && !ended) {
          ended = true;
          onEnd();
        }

        const [x, y] = sample.pose.target;
        let finalPose = sample.pose;

        if (sample.follow !== null) {
          const unit = track(sample.follow, now) ?? lastSubject;
          lastSubject = unit;
          finalPose = { ...sample.pose, target: [unit[0] + x, unit[1] + y] };
        } else {
          lastSubject = [x, y];
        }

        return finalPose;
      },
      frame({ now }) {
        if (startTime === null) return;
        const elapsed = (now - startTime) / 1000;
        const duration = reelDuration(reel);
        if (state.status === "recording") {
          setState({ status: "recording", elapsed, duration });
        }
      },
    };
  };

  const startRecording = (canvas: HTMLCanvasElement, audioStream?: MediaStream | null) => {
    const settings = settingsRef.current;
    if (!settings) return;

    try {
      const fps = settings.fps;
      const videoStream = canvas.captureStream(fps);

      let combinedStream: MediaStream;
      if (audioStream && settings.audio) {
        const tracks = [...videoStream.getVideoTracks(), ...audioStream.getAudioTracks()];
        combinedStream = new MediaStream(tracks);
      } else {
        combinedStream = videoStream;
      }

      const mimeType = MediaRecorder.isTypeSupported("video/webm;codecs=vp9,opus")
        ? "video/webm;codecs=vp9,opus"
        : MediaRecorder.isTypeSupported("video/webm;codecs=vp8,opus")
          ? "video/webm;codecs=vp8,opus"
          : "video/webm";

      const recorder = new MediaRecorder(combinedStream, {
        mimeType,
        videoBitsPerSecond: 16_000_000,
      });

      chunksRef.current = [];
      startTimeRef.current = performance.now();

      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) {
          chunksRef.current.push(e.data);
        }
      };

      recorder.onstop = () => {
        setState({ status: "encoding" });
        const blob = new Blob(chunksRef.current, { type: mimeType });
        const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
        const filename = `capture-${settings.preset}-${timestamp}.webm`;
        setState({ status: "complete", blob, filename });
        recorderRef.current = null;
      };

      recorder.onerror = (e) => {
        setState({
          status: "error",
          message: `Recording error: ${(e as ErrorEvent).message ?? "unknown"}`,
        });
        recorderRef.current = null;
      };

      recorderRef.current = recorder;
      recorder.start(1000);
      setState({
        status: "recording",
        elapsed: 0,
        duration: reelDuration(reelRef.current!),
      });
    } catch (err) {
      setState({
        status: "error",
        message: `Failed to start recording: ${err instanceof Error ? err.message : String(err)}`,
      });
    }
  };

  const stopRecording = () => {
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    }
  };

  return {
    active: settingsRef.current !== null,
    settings: settingsRef.current,
    state,
    reel: reelRef.current,
    createPilot,
    startRecording,
    stopRecording,
  };
}

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

import type { WeaponFireEvent } from "./captureConfig";

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
  /** Timestamps of start calibration bursts (performance.now()). */
  calibrationTimes?: number[];
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
  getAudioRenderParams?: () => { sampleRate: number; presentation: unknown; catalog: unknown } | null;
  /** [Deterministic mode] Render recorded audio offline and return as WAV blob. */
  renderOfflineAudio?: (durationS: number) => Promise<Blob | null>;
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
    calibrationTimes: extras?.calibrationTimes,
    endCalibrationTimes: extras?.endCalibrationTimes,
    stepFrame: extras?.stepFrame,
    enterDeterministicMode: extras?.enterDeterministicMode,
    getRecordedAudioCommands: extras?.getRecordedAudioCommands,
    getAudioRenderParams: extras?.getAudioRenderParams,
    renderOfflineAudio: extras?.renderOfflineAudio,
  };
}
