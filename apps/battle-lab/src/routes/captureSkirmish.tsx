/** Cinematic capture route for skirmish-style battles.
 *  Records a saved encounter battle with a scripted camera.
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
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { appResources } from "../appResources";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { LabViewport, type ViewportPilot, type ViewportGpu } from "../LabViewport";
import { sampleReel } from "../menuReel";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useBattleSession } from "../useBattleSession";
import { useLabLoading } from "../LabLoading";
import game from "@fixtures/game.json";
import { graphicsProfiles } from "../graphicsSettings";
import {
  parseCaptureParams,
  getCapturePreset,
  reelDuration,
  type CaptureSettings,
} from "../capture/captureConfig";
import {
  CaptureOverlay,
  exposeCaptureToWindow,
  type CaptureState,
} from "../capture/CinematicCapture";

export default function CaptureSkirmishPage() {
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
      <SessionCatalogScope set="test">
        <SavedEncounter fixture="street" encounter="advance">
          {(battle) => <CaptureSession settings={settings} battle={battle} />}
        </SavedEncounter>
      </SessionCatalogScope>
    </Suspense>
  );
}

function CaptureSession({
  settings,
  battle,
}: {
  settings: CaptureSettings;
  battle: SavedBattle;
}) {
  const refused = appResources.error();
  const [state, setState] = useState<CaptureState>({
    status: "warming",
    message: "Preparing battle...",
  });

  const preset = getCapturePreset(settings.preset);
  const reel = preset.reel;
  const duration = reelDuration(reel);

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

  useEffect(() => {
    exposeCaptureToWindow(state, downloadBlob);
  }, [state, downloadBlob]);

  if (refused) return null;

  return (
    <>
      <CaptureBattle
        scenario={battle.scenario}
        settings={settings}
        reel={reel}
        duration={duration}
        state={state}
        setState={setState}
      />
      <CaptureOverlay state={state} onDownload={downloadBlob} />
    </>
  );
}

function CaptureBattle({
  scenario,
  settings,
  reel,
  duration,
  state,
  setState,
}: {
  scenario: string;
  settings: CaptureSettings;
  reel: ReturnType<typeof getCapturePreset>["reel"];
  duration: number;
  state: CaptureState;
  setState: (state: CaptureState) => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const gpuRef = useRef<ViewportGpu | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const [warmed, setWarmed] = useState(false);
  const recordingStartedRef = useRef(false);
  const pilotStartRef = useRef<number | null>(null);

  useLabLoading("renderer", warmed);

  const session = useBattleSession({
    scenario,
    seed: game.seed,
    inputEnabled: false,
    xray: false,
    sound: settings.audio,
  });

  const { sim, audio, hear } = session;

  const unitAt = useCallback(
    (id: number) => sim.latest.current?.own.find((u) => u.id === id)?.position ?? null,
    [sim.latest],
  );

  useEffect(() => {
    if (sim.observation && !warmed) {
      setWarmed(true);
    }
  }, [sim.observation, warmed]);

  const startRecording = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || recordingStartedRef.current) return;
    recordingStartedRef.current = true;

    try {
      const fps = settings.fps;
      const videoStream = canvas.captureStream(fps);

      let combinedStream: MediaStream;

      if (settings.audio && audio) {
        const audioTap = audio.createCaptureTap();
        if (audioTap) {
          const tracks = [
            ...videoStream.getVideoTracks(),
            ...audioTap.getAudioTracks(),
          ];
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
          const filename = `capture-skirmish-${settings.preset}-${timestamp}.webm`;
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
  }, [settings, duration, setState, audio]);

  const stopRecording = useCallback(() => {
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    }
  }, []);

  const trackLag = 0.5;
  const trackedRef = useRef<{ follow: number; at: [number, number]; now: number } | null>(null);
  const lastSubjectRef = useRef<readonly [number, number]>([0, 0]);

  const pilot: ViewportPilot = {
    pose(now) {
      if (pilotStartRef.current === null) {
        pilotStartRef.current = now;
        if (warmed && state.status === "warming") {
          startRecording();
        }
      }

      const elapsed = (now - pilotStartRef.current) / 1000;
      const sample = sampleReel(reel, elapsed);

      if (sample.done && recorderRef.current?.state === "recording") {
        stopRecording();
        return sample.pose;
      }

      const [x, y] = sample.pose.target;
      let finalPose = sample.pose;

      if (sample.follow !== null) {
        const live = unitAt(sample.follow);
        const tracked = trackedRef.current;
        if (tracked?.follow !== sample.follow) {
          trackedRef.current = live ? { follow: sample.follow, at: [live[0], live[1]], now } : null;
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

      return finalPose;
    },
    frame({ now }) {
      hear();
      if (pilotStartRef.current === null || state.status !== "recording") return;
      const elapsed = (now - pilotStartRef.current) / 1000;
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
    if (warmed && state.status === "warming" && pilotStartRef.current !== null) {
      startRecording();
    }
  }, [warmed, state.status, startRecording]);

  useEffect(() => {
    return () => {
      audio?.releaseCaptureTap();
    };
  }, [audio]);

  const world = useFeed(session.meshes);
  if (!session.meshes || sim.error) return null;

  return (
    <div
      style={{
        width: settings.width,
        height: settings.height,
        position: "relative",
      }}
    >
      <div
        ref={(el) => {
          if (el) {
            const canvas = el.querySelector("canvas");
            if (canvas) canvasRef.current = canvas;
          }
        }}
        style={{ width: "100%", height: "100%" }}
      >
        <LabViewport
          fixture="capture-skirmish"
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
        />
      </div>
    </div>
  );
}
