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

  // Track reelReadyTime and audioStartTime from the battle component
  const [reelReadyTime, setReelReadyTime] = useState<number | null>(null);
  const [audioStartTime, setAudioStartTime] = useState<number | null>(null);

  useEffect(() => {
    exposeCaptureToWindow(state, downloadBlob, {
      duration,
      audioBlob: audioBlob ?? undefined,
      startAudioOnly: startAudioOnlyRef.current ?? undefined,
      fireEvents,
      followUnitId,
      reelReadyTime,
      audioStartTime,
    });
  }, [state, downloadBlob, duration, audioBlob, fireEvents, followUnitId, reelReadyTime, audioStartTime]);

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
        onAudioStart={setAudioStartTime}
        isTrackedVehicle={isTrackedVehicle}
      />
      <CaptureOverlay state={state} onDownload={downloadBlob} hidden={settings.mode === "page"} />
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
  onAudioStart,
  isTrackedVehicle,
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
  onAudioStart?: (time: number) => void;
  isTrackedVehicle: (kind: string) => boolean;
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
  /** Frame count since pilot started; reel is ready after at least one frame rendered. */
  const reelFrameCountRef = useRef(0);
  /** Timestamp when the reel-ready signal was sent (first pose applied + frame rendered). */
  const reelReadyTimeRef = useRef<number | null>(null);
  /** Timestamp when audio recording started (performance.now()). */
  const audioStartTimeRef = useRef<number | null>(null);
  /** Whether we've scheduled the recording stop (to prevent duplicate timeouts). */
  const stoppingRef = useRef(false);

  // Refs for values accessed in the frame callback to avoid stale closures
  const stateRef = useRef(state);
  const onFireEventsRef = useRef(onFireEvents);
  const onFollowUnitIdRef = useRef(onFollowUnitId);
  const onReelReadyRef = useRef(onReelReady);
  const reportedFollowRef = useRef<number | null | undefined>(undefined);
  stateRef.current = state;
  onFireEventsRef.current = onFireEvents;
  onFollowUnitIdRef.current = onFollowUnitId;
  onReelReadyRef.current = onReelReady;

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
            // Record audio start time for sync with video cut
            const audioStart = performance.now();
            audioStartTimeRef.current = audioStart;
            onAudioStart?.(audioStart);
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

  const pilot: ViewportPilot = {
    pose(now) {
      if (pilotStartRef.current === null) {
        pilotStartRef.current = now;
        // Don't start recording yet - wait for follow to initialize and first frame to render
      }

      const elapsed = (now - pilotStartRef.current) / 1000;
      const sample = sampleReel(reel, elapsed);

      if (sample.done && recorderRef.current?.state === "recording" && !stoppingRef.current) {
        // Add a small tail (0.5s) to ensure the last frame is captured
        stoppingRef.current = true;
        setTimeout(() => stopRecording(), 500);
      }
      if (sample.done) {
        return sample.pose;
      }

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
            trackedRef.current = live ? { follow: followedId, at: [live[0], live[1]], now } : null;
            if (live) followInitializedRef.current = true;
          }
          if (trackedRef.current) {
            if (live) {
              const k = 1 - Math.exp(-(now - trackedRef.current.now) / 1000 / trackLag);
              trackedRef.current.at = [
                trackedRef.current.at[0] + (live[0] - trackedRef.current.at[0]) * k,
                trackedRef.current.at[1] + (live[1] - trackedRef.current.at[1]) * k,
              ];
            }
            trackedRef.current.now = now;
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

      return finalPose;
    },
    frame({ now, camera }) {
      const presentation = sim.client?.presentation;
      if (audio && presentation) {
        const listener = cameraListener(camera, game.presentation.audio.listener_eye_share);
        audio.updateWithListener(presentation.wasm, listener);
      }
      // Track frames rendered with the reel pose applied
      if (pilotStartRef.current !== null) {
        reelFrameCountRef.current++;
        // Signal reel-ready only after:
        // 1. Follow unit is resolved and smoothed position is initialized
        // 2. At least one frame has rendered with the correct pose
        const canBeReady = followInitializedRef.current && reelFrameCountRef.current >= 1;
        if (canBeReady && reelReadyTimeRef.current === null) {
          reelReadyTimeRef.current = now;
          onReelReadyRef.current?.(now);
          // Now that the pose is correct and rendered, start recording
          if (warmed && stateRef.current.status === "warming") {
            startRecording();
          }
        }
      }
      if (pilotStartRef.current === null || stateRef.current.status !== "recording") return;
      const elapsed = (now - pilotStartRef.current) / 1000;
      setState({ status: "recording", elapsed, duration });

      // Track weapon fire events for the sidecar JSON
      // Use sim.latest.current for fresh observation data
      const observation = sim.latest.current;
      if (fireTracker && observation) {
        const sample = sampleReel(reel, elapsed);
        // Use the same resolution as the camera for consistency
        const followedId = resolveFollow(sample.follow);
        // Report the follow unit ID once (on first resolution)
        if (reportedFollowRef.current === undefined && followedId !== null) {
          reportedFollowRef.current = followedId;
          onFollowUnitIdRef.current?.(followedId);
        }
        fireTracker.setFollowedUnit(followedId);
        const newEvents = fireTracker.update(observation, now);
        if (newEvents.length > 0) {
          onFireEventsRef.current?.(fireTracker.getEvents());
        }
      }
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
    </div>
  );
}
