/** Cinematic capture route: records the menu backdrop battle with a scripted camera.
 *  Navigate to /capture?capture=<preset> to start recording.
 *  The recording stops automatically when the camera path completes.
 *
 *  URL parameters:
 *    capture=<preset>   - Camera preset (default, top-down, ground-level, orbit)
 *    capture-width=N    - Output width (default 1920)
 *    capture-height=N   - Output height (default 1080)
 *    capture-fps=N      - Frame rate (default 60)
 *    capture-audio=bool - Include audio (default true)
 *    capture-cards=bool - Show info cards (default true)
 *    capture-max=N      - Max duration in seconds (default 120)
 */

import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { SessionCatalogScope, useSessionCatalog } from "@web/battle/catalog/context";
import backdrop from "@fixtures/menu-backdrop.json";
import { listMaps } from "@web/maps/catalogue";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { appResources } from "../appResources";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { LabViewport, type ViewportPilot, type ViewportGpu } from "../LabViewport";
import { validateBackdrop, sampleReel, type BackdropScene } from "../menuReel";
import { savedBattle } from "../savedMaps";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { useBattleSession } from "../useBattleSession";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import { useLabLoading } from "../LabLoading";
import type { ScriptedSim } from "../useSimSession";
import game from "@fixtures/game.json";
import { graphicsProfiles } from "../graphicsSettings";
import {
  parseCaptureParams,
  getCapturePreset,
  reelDuration,
  WeaponFireTracker,
  resolveFollowId,
  type CaptureSettings,
  type WeaponFireEvent,
} from "../capture/captureConfig";
import {
  CaptureOverlay,
  exposeCaptureToWindow,
  type CaptureState,
} from "../capture/CinematicCapture";

const SCENES = validateBackdrop(backdrop, listMaps()).scenes;

export default function CapturePage() {
  const settings = parseCaptureParams(window.location.search);
  if (!settings) {
    return (
      <main className="lab-rejected" data-testid="error">
        Capture mode requires ?capture=&lt;preset&gt; URL parameter.
        <br />
        Available presets: default, top-down, ground-level, orbit
      </main>
    );
  }

  return (
    <Suspense fallback={null}>
      <SessionCatalogScope set="menu">
        <CaptureSession settings={settings} />
      </SessionCatalogScope>
    </Suspense>
  );
}

function CaptureSession({ settings }: { settings: CaptureSettings }) {
  const { rules, units } = useSessionCatalog();
  const refused = appResources.error();
  const [state, setState] = useState<CaptureState>({
    status: "warming",
    message: "Preparing battle...",
  });
  const [audioBlob, setAudioBlob] = useState<Blob | null>(null);
  const [fireEvents, setFireEvents] = useState<WeaponFireEvent[]>([]);
  const [followUnitId, setFollowUnitId] = useState<number | null>(null);
  const startAudioOnlyRef = useRef<(() => void) | null>(null);
  const fireTrackerRef = useRef(new WeaponFireTracker());

  // Check if a unit kind is a tracked vehicle (tank, etc.)
  const isTrackedVehicle = useCallback(
    (kind: string) => {
      try {
        const vehicleClass = units.type(kind).vehicle_class;
        return vehicleClass?.startsWith("tracked") ?? false;
      } catch {
        return false;
      }
    },
    [units],
  );

  // Set up the weapon kind resolver using the unit catalog
  useEffect(() => {
    fireTrackerRef.current.setResolver({
      getWeaponKind(unitKind: string, mountIndex: number): string | null {
        try {
          const type = units.type(unitKind);
          const mount = type.mounts[mountIndex];
          return mount?.weapons[0] ?? null;
        } catch {
          return null;
        }
      },
    });
  }, [units]);

  const scene = SCENES[0];
  const preset = getCapturePreset(settings.preset);
  const reel = preset.reel;
  const duration = reelDuration(reel);

  const battle = useBuiltScenario(scene, (_, s) =>
    savedBattle(s.map, s.encounter, rules).then((b) => b.scenario),
  );

  const failed = buildFailed(battle) ? battle.error : null;
  useEffect(() => {
    if (failed) {
      setState({ status: "error", message: `Failed to prepare: ${failed}` });
    }
  }, [failed]);

  useLabLoading("renderer", !refused && state.status !== "error");

  const downloadBlob = useCallback(() => {
    if (state.status !== "complete") return;
    const url = URL.createObjectURL(state.blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = state.filename;
    a.click();
    URL.revokeObjectURL(url);
  }, [state]);

  // Track reelReadyTime, reelEndTime, audioStartTime, and audioLatency from the battle component
  const [reelReadyTime, setReelReadyTime] = useState<number | null>(null);
  const [reelEndTime, setReelEndTime] = useState<number | null>(null);
  const [audioStartTime, setAudioStartTime] = useState<number | null>(null);
  const [audioLatency, setAudioLatency] = useState<number>(0);
  // Calibration bursts: arrays of timestamps for start and end calibration
  const [calibrationTimes, setCalibrationTimes] = useState<number[]>([]);
  const [endCalibrationTimes, setEndCalibrationTimes] = useState<number[]>([]);
  // Ref to the triggerCalibrationBurst function provided by CaptureBattle
  const triggerCalibrationBurstRef = useRef<((count: number, intervalMs: number) => Promise<number[]>) | null>(null);
  // Deterministic mode function refs
  const stepFrameRef = useRef<((virtualTimeMs: number) => Promise<void>) | null>(null);
  const enterDeterministicModeRef = useRef<(() => void) | null>(null);
  const stopAudioRef = useRef<(() => void) | null>(null);
  const getRecordedAudioCommandsRef = useRef<(() => unknown[]) | null>(null);
  const getAudioRenderParamsRef = useRef<(() => { sampleRate: number; presentation: unknown; catalog: unknown } | null) | null>(null);
  const renderOfflineAudioRef = useRef<((durationS: number) => Promise<Blob | null>) | null>(null);

  useEffect(() => {
    exposeCaptureToWindow(state, downloadBlob, {
      duration,
      audioBlob: audioBlob ?? undefined,
      startAudioOnly: startAudioOnlyRef.current ?? undefined,
      stopAudio: stopAudioRef.current ?? undefined,
      fireEvents,
      followUnitId,
      reelReadyTime,
      reelEndTime,
      audioStartTime,
      audioLatency,
      triggerCalibrationBurst: triggerCalibrationBurstRef.current ?? undefined,
      calibrationTimes,
      endCalibrationTimes,
      stepFrame: stepFrameRef.current ?? undefined,
      enterDeterministicMode: enterDeterministicModeRef.current ?? undefined,
      getRecordedAudioCommands: getRecordedAudioCommandsRef.current ?? undefined,
      getAudioRenderParams: getAudioRenderParamsRef.current ?? undefined,
      renderOfflineAudio: renderOfflineAudioRef.current ?? undefined,
    });
  }, [state, downloadBlob, duration, audioBlob, fireEvents, followUnitId, reelReadyTime, reelEndTime, audioStartTime, audioLatency, calibrationTimes, endCalibrationTimes]);

  if (refused || !battle || buildFailed(battle)) return null;

  return (
    <>
      <CaptureBattle
        scene={scene}
        scenario={battle}
        settings={settings}
        reel={reel}
        duration={duration}
        state={state}
        setState={setState}
        onAudioBlob={setAudioBlob}
        onStartAudioOnly={(fn) => {
          startAudioOnlyRef.current = fn;
        }}
        fireTracker={fireTrackerRef.current}
        onFireEvents={setFireEvents}
        onFollowUnitId={setFollowUnitId}
        onReelReady={setReelReadyTime}
        onReelEndTime={setReelEndTime}
        onAudioStart={(time, latency) => {
          setAudioStartTime(time);
          setAudioLatency(latency);
        }}
        onCalibrationBurst={(fn) => {
          triggerCalibrationBurstRef.current = fn;
        }}
        onCalibrationTimes={setCalibrationTimes}
        onEndCalibrationTimes={setEndCalibrationTimes}
        isTrackedVehicle={isTrackedVehicle}
        onStepFrame={(fn) => {
          stepFrameRef.current = fn;
        }}
        onEnterDeterministicMode={(fn) => {
          enterDeterministicModeRef.current = fn;
        }}
        onStopAudio={(fn) => {
          stopAudioRef.current = fn;
        }}
        onGetRecordedAudioCommands={(fn) => {
          getRecordedAudioCommandsRef.current = fn;
        }}
        onGetAudioRenderParams={(fn) => {
          getAudioRenderParamsRef.current = fn;
        }}
        onRenderOfflineAudio={(fn) => {
          renderOfflineAudioRef.current = fn;
        }}
      />
      <CaptureOverlay state={state} onDownload={downloadBlob} hidden={settings.mode === "page" || settings.mode === "deterministic"} />
    </>
  );
}

function CaptureBattle({
  scene,
  scenario,
  settings,
  reel,
  duration,
  state,
  setState,
  onAudioBlob,
  onStartAudioOnly,
  fireTracker,
  onFireEvents,
  onFollowUnitId,
  onReelReady,
  onReelEndTime,
  onAudioStart,
  onCalibrationBurst,
  onCalibrationTimes,
  onEndCalibrationTimes,
  isTrackedVehicle,
  onStepFrame,
  onEnterDeterministicMode,
  onStopAudio,
  onGetRecordedAudioCommands,
  onGetAudioRenderParams,
  onRenderOfflineAudio,
}: {
  scene: BackdropScene;
  scenario: string;
  settings: CaptureSettings;
  reel: ReturnType<typeof getCapturePreset>["reel"];
  duration: number;
  state: CaptureState;
  setState: (state: CaptureState) => void;
  onAudioBlob?: (blob: Blob) => void;
  onStartAudioOnly?: (fn: () => void) => void;
  fireTracker?: WeaponFireTracker;
  onFireEvents?: (events: WeaponFireEvent[]) => void;
  onFollowUnitId?: (id: number | null) => void;
  onReelReady?: (time: number) => void;
  onReelEndTime?: (time: number) => void;
  onAudioStart?: (time: number, latency: number) => void;
  onCalibrationBurst?: (fn: (count: number, intervalMs: number) => Promise<number[]>) => void;
  onCalibrationTimes?: (times: number[]) => void;
  onEndCalibrationTimes?: (times: number[]) => void;
  isTrackedVehicle: (kind: string) => boolean;
  // Deterministic mode callbacks
  onStepFrame?: (fn: (virtualTimeMs: number) => Promise<void>) => void;
  onEnterDeterministicMode?: (fn: () => void) => void;
  onStopAudio?: (fn: () => void) => void;
  onGetRecordedAudioCommands?: (fn: () => unknown[]) => void;
  onGetAudioRenderParams?: (fn: () => { sampleRate: number; presentation: unknown; catalog: unknown } | null) => void;
  onRenderOfflineAudio?: (fn: (durationS: number) => Promise<Blob | null>) => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const gpuRef = useRef<ViewportGpu | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const [warmed, setWarmed] = useState(false);
  const recordingStartedRef = useRef(false);
  const pilotStartRef = useRef<number | null>(null);
  const audioStreamRef = useRef<MediaStream | null>(null);
  const audioBlobRef = useRef<Blob | null>(null);
  const isPageMode = settings.mode === "page";
  const isDeterministicMode = settings.mode === "deterministic";
  /** Frame duration in ms (1000 / fps), used for deterministic mode smoothing. */
  const frameDurationMs = 1000 / settings.fps;
  /** Frame count since pilot started; reel is ready after at least one frame rendered. */
  const reelFrameCountRef = useRef(0);
  /** Timestamp when the reel-ready signal was sent (first pose applied + frame rendered). */
  const reelReadyTimeRef = useRef<number | null>(null);
  /** Timestamp when the reel ended (sample.done became true for the first time). */
  const reelEndTimeRef = useRef<number | null>(null);
  /** Timestamp when audio recording started (performance.now()). */
  const audioStartTimeRef = useRef<number | null>(null);
  /** Whether we've scheduled the recording stop (to prevent duplicate timeouts). */
  const stoppingRef = useRef(false);
  /** Ref to the calibration overlay element. */
  const calibrationOverlayRef = useRef<HTMLDivElement | null>(null);
  
  // Deterministic mode state
  /** Whether deterministic mode is active (virtual clock). */
  const deterministicActiveRef = useRef(false);
  /** Current virtual time in deterministic mode (ms). */
  const virtualTimeRef = useRef<number>(0);
  /** Resolve function for the current stepFrame() call. */
  const stepFrameResolveRef = useRef<(() => void) | null>(null);

  // Refs for values accessed in the frame callback to avoid stale closures
  const stateRef = useRef(state);
  const onFireEventsRef = useRef(onFireEvents);
  const onFollowUnitIdRef = useRef(onFollowUnitId);
  const onReelReadyRef = useRef(onReelReady);
  const onReelEndTimeRef = useRef(onReelEndTime);
  const reportedFollowRef = useRef<number | null | undefined>(undefined);
  stateRef.current = state;
  onFireEventsRef.current = onFireEvents;
  onFollowUnitIdRef.current = onFollowUnitId;
  onReelReadyRef.current = onReelReady;
  onReelEndTimeRef.current = onReelEndTime;

  useLabLoading("renderer", warmed);

  const scripted: ScriptedSim = {
    warmTo: Math.round(scene.warm_s * game.tick_hz),
    hold: () => Promise.resolve(),
    onWarm: () => setWarmed(true),
    onTick: () => {},
  };

  const session = useBattleSession({
    scenario,
    seed: scene.seed,
    scripted,
    inputEnabled: false,
    xray: false,
    sound: settings.audio,
  });

  const { sim, audio, hear } = session;

  // Calibration burst function: fires count flash/click pairs at intervalMs apart
  // Returns array of performance.now() timestamps for each flash
  const triggerCalibrationBurst = useCallback(
    async (count: number, intervalMs: number): Promise<number[]> => {
      const overlay = calibrationOverlayRef.current;
      const timestamps: number[] = [];

      for (let i = 0; i < count; i++) {
        const timestamp = performance.now();
        timestamps.push(timestamp);

        // Show white overlay for 2 frames (~33ms at 60fps)
        if (overlay) {
          overlay.style.display = "block";
          setTimeout(() => {
            overlay.style.display = "none";
          }, 33);
        }

        // Play calibration click through the same audio graph
        if (audio) {
          audio.playCalibrationClick();
        }

        // Wait for interval before next flash (except after last one)
        if (i < count - 1) {
          await new Promise((r) => setTimeout(r, intervalMs));
        }
      }

      return timestamps;
    },
    [audio],
  );

  // Provide the calibration burst function to parent
  useEffect(() => {
    onCalibrationBurst?.(triggerCalibrationBurst);
  }, [onCalibrationBurst, triggerCalibrationBurst]);

  // Deterministic mode: step frame to a specific virtual time
  const stepFrame = useCallback(
    (virtualTimeMs: number): Promise<void> => {
      return new Promise((resolve) => {
        virtualTimeRef.current = virtualTimeMs;
        stepFrameResolveRef.current = resolve;
        // The next RAF frame will pick up the new virtual time and resolve when done
      });
    },
    [],
  );

  // Deterministic mode: enter deterministic mode (use virtual clock)
  // This advances the sim to cover the entire duration before frame capture begins
  const enterDeterministicMode = useCallback(async () => {
    deterministicActiveRef.current = true;
    virtualTimeRef.current = 0;
    // Enable audio recording mode
    if (audio) {
      audio.enableRecording();
    }
    // Advance the sim to cover the entire duration
    // Duration is in seconds, tick rate is game.tick_hz (30)
    const ticksNeeded = Math.ceil(duration * game.tick_hz);
    console.log(`[deterministic] Advancing sim ${ticksNeeded} ticks for ${duration}s duration`);
    if (sim.client) {
      sim.client.pause();
      await sim.client.advance(ticksNeeded);
      console.log(`[deterministic] Sim advanced, ready for capture`);
    }
    // Mark ready immediately in deterministic mode
    if (reelReadyTimeRef.current === null) {
      reelReadyTimeRef.current = performance.now();
      onReelReadyRef.current?.(reelReadyTimeRef.current);
    }
  }, [audio, duration, sim.client]);

  // Deterministic mode: stop audio recording
  const stopAudio = useCallback(() => {
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    }
  }, []);

  // Provide deterministic mode functions to parent
  useEffect(() => {
    onStepFrame?.(stepFrame);
  }, [onStepFrame, stepFrame]);

  useEffect(() => {
    onEnterDeterministicMode?.(enterDeterministicMode);
  }, [onEnterDeterministicMode, enterDeterministicMode]);

  useEffect(() => {
    onStopAudio?.(stopAudio);
  }, [onStopAudio, stopAudio]);

  // Deterministic mode: get recorded audio commands
  const getRecordedAudioCommands = useCallback(() => {
    return audio?.getRecordedCommands() ?? [];
  }, [audio]);

  useEffect(() => {
    onGetRecordedAudioCommands?.(getRecordedAudioCommands);
  }, [onGetRecordedAudioCommands, getRecordedAudioCommands]);

  // Deterministic mode: get audio rendering parameters
  const getAudioRenderParams = useCallback(() => {
    if (!audio) return null;
    return {
      sampleRate: 48000, // Standard sample rate
      presentation: audio.getPresentation(),
      catalog: audio.getCatalog(),
    };
  }, [audio]);

  useEffect(() => {
    onGetAudioRenderParams?.(getAudioRenderParams);
  }, [onGetAudioRenderParams, getAudioRenderParams]);

  // Deterministic mode: render audio offline
  const renderOfflineAudio = useCallback(
    async (durationS: number): Promise<Blob | null> => {
      if (!audio) return null;
      return audio.renderOffline(durationS);
    },
    [audio],
  );

  useEffect(() => {
    onRenderOfflineAudio?.(renderOfflineAudio);
  }, [onRenderOfflineAudio, renderOfflineAudio]);

  // Get a unit's position by its ID
  const unitPosition = useCallback(
    (id: number) => sim.latest.current?.own.find((u) => u.id === id)?.position ?? null,
    [sim.latest],
  );

  // Resolve a follow value to actual unit ID, using resolveFollowId for consistency
  const resolveFollow = useCallback(
    (follow: number | null) => {
      const observation = sim.latest.current;
      if (!observation) return null;
      return resolveFollowId(follow, observation, isTrackedVehicle);
    },
    [sim.latest, isTrackedVehicle],
  );

  const startRecording = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || recordingStartedRef.current) return;
    recordingStartedRef.current = true;

    try {
      const fps = settings.fps;

      // In page mode, only capture audio (video is captured via CDP screencast)
      if (isPageMode) {
        if (settings.audio && audio) {
          const audioTap = audio.createCaptureTap();
          if (audioTap) {
            audioStreamRef.current = audioTap;
            const audioStream = new MediaStream(audioTap.getAudioTracks());
            const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
              ? "audio/webm;codecs=opus"
              : "audio/webm";

            const recorder = new MediaRecorder(audioStream, {
              mimeType,
              audioBitsPerSecond: 192_000,
            });

            chunksRef.current = [];

            recorder.ondataavailable = (e) => {
              if (e.data.size > 0) {
                chunksRef.current.push(e.data);
              }
            };

            recorder.onstop = () => {
              audio?.releaseCaptureTap();
              const blob = new Blob(chunksRef.current, { type: mimeType });
              audioBlobRef.current = blob;
              onAudioBlob?.(blob);
              setState({ status: "complete", blob, filename: "audio.webm" });
            };

            recorder.onerror = (e) => {
              audio?.releaseCaptureTap();
              setState({
                status: "error",
                message: `Audio recording error: ${(e as ErrorEvent).message ?? "unknown"}`,
              });
            };

            recorderRef.current = recorder;
            // Record audio start time and output latency for sync with video cut
            const audioStart = performance.now();
            const audioLatency = audio.getOutputLatency();
            audioStartTimeRef.current = audioStart;
            onAudioStart?.(audioStart, audioLatency);
            recorder.start(1000);
          }
        }
        setState({ status: "recording", elapsed: 0, duration });
        return;
      }

      // Browser mode: capture both video and audio
      const videoStream = canvas.captureStream(fps);

      let combinedStream: MediaStream;

      if (settings.audio && audio) {
        const audioTap = audio.createCaptureTap();
        if (audioTap) {
          audioStreamRef.current = audioTap;
          const tracks = [...videoStream.getVideoTracks(), ...audioTap.getAudioTracks()];
          combinedStream = new MediaStream(tracks);
        } else {
          combinedStream = videoStream;
        }
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
        audioBitsPerSecond: 192_000,
      });

      chunksRef.current = [];

      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) {
          chunksRef.current.push(e.data);
        }
      };

      recorder.onstop = () => {
        setState({ status: "encoding" });
        audio?.releaseCaptureTap();
        setTimeout(() => {
          const blob = new Blob(chunksRef.current, { type: mimeType });
          const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
          const filename = `capture-${settings.preset}-${timestamp}.webm`;
          setState({ status: "complete", blob, filename });
        }, 100);
      };

      recorder.onerror = (e) => {
        audio?.releaseCaptureTap();
        setState({
          status: "error",
          message: `Recording error: ${(e as ErrorEvent).message ?? "unknown"}`,
        });
      };

      recorderRef.current = recorder;
      recorder.start(1000);
      setState({ status: "recording", elapsed: 0, duration });
    } catch (err) {
      audio?.releaseCaptureTap();
      setState({
        status: "error",
        message: `Failed to start recording: ${err instanceof Error ? err.message : String(err)}`,
      });
    }
  }, [settings, duration, setState, audio, isPageMode, onAudioBlob]);

  // In page mode, expose startRecording for CDP to call
  useEffect(() => {
    if (isPageMode && onStartAudioOnly) {
      onStartAudioOnly(startRecording);
    }
  }, [isPageMode, onStartAudioOnly, startRecording]);

  const stopRecording = useCallback(() => {
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    }
  }, []);

  const trackLag = 0.5;
  const trackedRef = useRef<{ follow: number; at: [number, number]; now: number } | null>(null);
  const lastSubjectRef = useRef<readonly [number, number]>([0, 0]);
  /** True once the follow unit is resolved and the smoothed position is initialized. */
  const followInitializedRef = useRef(false);
  /** The last fully-computed pose (with follow tracking applied), frozen when reel ends. */
  const heldPoseRef = useRef<CameraPose | null>(null);
  /** Reel start time: set to reelReadyTime when ready, so reel clock starts at ready signal. */
  const reelStartTimeRef = useRef<number | null>(null);
  /** Whether end calibration has been triggered. */
  const endCalibTriggeredRef = useRef(false);
  /** Ref to callbacks for triggering end calibration from within setTimeout. */
  const onEndCalibrationTimesRef = useRef(onEndCalibrationTimes);
  onEndCalibrationTimesRef.current = onEndCalibrationTimes;

  const pilot: ViewportPilot = {
    pose(now) {
      // In deterministic mode, use virtual time instead of wall clock
      const effectiveNow = deterministicActiveRef.current
        ? (reelReadyTimeRef.current ?? performance.now()) + virtualTimeRef.current
        : now;
      
      // Track first pose call for initialization, but use reelStartTime for elapsed
      if (pilotStartRef.current === null) {
        pilotStartRef.current = effectiveNow;
        // Don't start recording yet - wait for follow to initialize and first frame to render
      }

      // Use reelStartTime (set at reel-ready) for elapsed calculation
      // This removes the drift between first pose() and reel-ready
      const startTime = reelStartTimeRef.current ?? pilotStartRef.current;
      const elapsed = deterministicActiveRef.current
        ? virtualTimeRef.current / 1000  // Deterministic: virtual time is already relative
        : (effectiveNow - startTime) / 1000;
      const sample = sampleReel(reel, elapsed);

      // When reel ends: record time, trigger end calibration, then schedule stop
      if (sample.done) {
        // Record reelEndTime on first done frame (independent of recorder state)
        if (reelEndTimeRef.current === null) {
          reelEndTimeRef.current = now;
          onReelEndTimeRef.current?.(now);
          console.log(`[capture] reel done at ${now.toFixed(1)}ms (elapsed=${elapsed.toFixed(2)}s)`);
        }
        // Trigger end calibration burst once, then schedule stop after it completes
        // Extended tail: 1.5s total = ~500ms for end calibration + buffer
        if (!stoppingRef.current) {
          stoppingRef.current = true;
          console.log(`[capture] scheduling end calibration burst + recorder stop`);
          // Wait 200ms for any in-flight audio to settle, then trigger end calibration
          setTimeout(async () => {
            try {
              const endTimes = await triggerCalibrationBurst(5, 150);
              console.log(`[capture] end calibration burst fired: ${endTimes.map((t) => t.toFixed(0)).join(", ")}`);
              onEndCalibrationTimesRef.current?.(endTimes);
            } catch (e) {
              console.error(`[capture] end calibration failed:`, e);
            }
            // Wait 500ms after end calibration for audio to record, then stop
            setTimeout(() => {
              console.log(`[capture] recorder stop triggered`);
              stopRecording();
            }, 500);
          }, 200);
        }
        // Return the HELD pose (frozen last tracked position) - never recompute
        if (heldPoseRef.current !== null) {
          return heldPoseRef.current;
        }
        // Fallback: if no held pose yet, return sample.pose (shouldn't happen normally)
        return sample.pose;
      }

      // Reel still running: compute follow-tracked pose and save it
      const [x, y] = sample.pose.target;
      let finalPose = sample.pose;

      if (sample.follow !== null) {
        // Resolve the follow value to an actual unit ID
        const followedId = resolveFollow(sample.follow);
        if (followedId !== null) {
          const live = unitPosition(followedId);
          const tracked = trackedRef.current;
          if (tracked?.follow !== followedId) {
            // Initialize smoothed position directly to live position (no lag on first frame)
            trackedRef.current = live ? { follow: followedId, at: [live[0], live[1]], time: effectiveNow } : null;
            if (live) followInitializedRef.current = true;
          }
          if (trackedRef.current) {
            if (live) {
              // In deterministic mode, use virtual time delta (1/60s per frame)
              // In realtime mode, use wall clock delta
              const timeDelta = deterministicActiveRef.current
                ? frameDurationMs / 1000 // Exactly 1/60s per frame in deterministic mode
                : (effectiveNow - trackedRef.current.time) / 1000;
              const k = 1 - Math.exp(-timeDelta / trackLag);
              trackedRef.current.at = [
                trackedRef.current.at[0] + (live[0] - trackedRef.current.at[0]) * k,
                trackedRef.current.at[1] + (live[1] - trackedRef.current.at[1]) * k,
              ];
            }
            trackedRef.current.time = effectiveNow;
            const unit = trackedRef.current.at;
            lastSubjectRef.current = unit;
            finalPose = { ...sample.pose, target: [unit[0] + x, unit[1] + y] };
          }
        } else {
          lastSubjectRef.current = [x, y];
        }
      } else {
        // No follow needed - mark as initialized
        followInitializedRef.current = true;
        lastSubjectRef.current = [x, y];
      }

      // Save computed pose for holding when reel ends
      heldPoseRef.current = finalPose;

      return finalPose;
    },
    frame({ now, camera }) {
      const presentation = sim.client?.presentation;
      if (audio && presentation) {
        const listener = cameraListener(camera, game.presentation.audio.listener_eye_share);
        // In deterministic mode, set virtual time and record (don't play real-time)
        if (deterministicActiveRef.current) {
          const virtualTimeS = virtualTimeRef.current / 1000;
          audio.setVirtualTime(virtualTimeS);
        }
        audio.updateWithListener(presentation.wasm, listener);
      }
      // Track frames rendered with the reel pose applied
      if (pilotStartRef.current !== null) {
        reelFrameCountRef.current++;
        // Signal reel-ready only after:
        // 1. Follow unit is resolved and smoothed position is initialized
        // 2. At least one frame has rendered with the correct pose
        const canBeReady = followInitializedRef.current && reelFrameCountRef.current >= 1;
        if (canBeReady && reelReadyTimeRef.current === null && !deterministicActiveRef.current) {
          reelReadyTimeRef.current = now;
          // Start reel clock at ready time (removes drift between first pose and ready)
          reelStartTimeRef.current = now;
          console.log(`[capture] reel ready at ${now.toFixed(1)}ms, reel clock starts now`);
          onReelReadyRef.current?.(now);
          // Now that the pose is correct and rendered, start recording
          if (warmed && stateRef.current.status === "warming") {
            startRecording();
          }
        }
      }
      
      // Calculate elapsed time consistently for both modes
      const startTime = reelStartTimeRef.current ?? pilotStartRef.current;
      const elapsed = deterministicActiveRef.current
        ? virtualTimeRef.current / 1000
        : (startTime !== null ? (now - startTime) / 1000 : 0);

      // Track weapon fire events for the sidecar JSON (both modes)
      // In deterministic mode, use virtual time for event timestamps
      const observation = sim.latest.current;
      if (fireTracker && observation) {
        const sample = sampleReel(reel, elapsed);
        const followedId = resolveFollow(sample.follow);
        if (reportedFollowRef.current === undefined && followedId !== null) {
          reportedFollowRef.current = followedId;
          onFollowUnitIdRef.current?.(followedId);
        }
        fireTracker.setFollowedUnit(followedId);
        // Use virtual time in deterministic mode, wall clock in realtime
        const eventTime = deterministicActiveRef.current
          ? virtualTimeRef.current
          : now;
        const newEvents = fireTracker.update(observation, eventTime);
        if (newEvents.length > 0) {
          onFireEventsRef.current?.(fireTracker.getEvents());
        }
      }

      // In deterministic mode, resolve the stepFrame promise after each frame
      if (deterministicActiveRef.current && stepFrameResolveRef.current) {
        const resolve = stepFrameResolveRef.current;
        stepFrameResolveRef.current = null;
        resolve();
        return; // Skip UI state updates in deterministic mode
      }
      
      if (pilotStartRef.current === null || stateRef.current.status !== "recording") return;
      setState({ status: "recording", elapsed, duration });
    },
  };

  const initialCamera: Camera3DParams = {
    target: [reel.shots[0].from.target[0], reel.shots[0].from.target[1], 0],
    distance: reel.shots[0].from.distance,
    yaw: reel.shots[0].from.yaw,
    pitch: reel.shots[0].from.pitch,
    ...gameCamera.lens,
  };

  useEffect(() => {
    if (sim.error) {
      setState({ status: "error", message: `Battle error: ${sim.error}` });
    }
  }, [sim.error, setState]);

  useEffect(() => {
    // Recording now starts from the frame() callback after the first reel frame is rendered.
    // This useEffect is kept for safety but should not trigger before the frame callback.
    if (warmed && state.status === "warming" && reelReadyTimeRef.current !== null) {
      startRecording();
    }
  }, [warmed, state.status, startRecording]);

  useEffect(() => {
    return () => {
      audio?.releaseCaptureTap();
    };
  }, [audio]);

  const world = useFeed(session.meshes);
  const observation = sim.observation;
  if (!session.meshes || sim.error) return null;

  return (
    <div
      className="menu-backdrop"
      style={{
        width: settings.width,
        height: settings.height,
        position: "relative",
      }}
    >
      <div
        className={`menu-backdrop-film${isPageMode ? " capture-mode" : ""}`}
        ref={(el) => {
          if (el) {
            const canvas = el.querySelector("canvas");
            if (canvas) canvasRef.current = canvas;
          }
        }}
      >
        <LabViewport
          fixture="capture"
          inputEnabled={false}
          world={world}
          {...session.known}
          {...session.viewport}
          initialCamera={initialCamera}
          groundAt={session.surfaceZ}
          onReady={(gpu) => {
            gpuRef.current = gpu;
            session.viewport.onReady(gpu);
          }}
          pilot={pilot}
          cameraConfig={gameCamera.config}
          quality={graphicsProfiles.high}
          onFrame={settings.showCards ? session.placePanels : undefined}
        />
      </div>
      {settings.showCards && (
        <ReadoutLayer
          own={observation?.own ?? []}
          rules={session.rules}
          selected={[]}
          handle={session.readouts}
        />
      )}
      {/* Calibration overlay: white flash for sync measurement */}
      <div
        ref={calibrationOverlayRef}
        style={{
          display: "none",
          position: "absolute",
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          backgroundColor: "#ffffff",
          zIndex: 9999,
          pointerEvents: "none",
        }}
      />
    </div>
  );
}
