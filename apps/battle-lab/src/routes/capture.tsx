// The capture route: one scene of a capture file played through the menu's
// `ScenePlayer`, as the player would see it at full quality: high graphics,
// full motion and the battle's sound, whatever this browser has saved, and
// the cuts dipping to black. With `record=1` it records the stage and hands
// the WebM to the capture CLI.
//
//   /labs/capture?capture=<name>&scene=<i>&cards=0|1[&record=1&fps=<n>]
//
// `<name>` is a file stem under fixtures/captures/; the stage fills the
// viewport, so the viewport is the film's size. When recording, the CLI
// exposes two functions before the page loads:
//   captureChunk(base64)  each WebM chunk, in order; the next waits for it;
//   captureDone(result)   once: `{frames, worstGapMs, seconds, audio}`, or
//                         `{error}` naming what failed.
// Recording starts while the battle holds at its warm tick, behind the veil,
// and stops on the reel's end, behind it again.
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { listMaps } from "@web/maps/catalogue";
import { appResources } from "../appResources";
import { filmCamera, validateCapture, type CaptureScene } from "../capture/capture";
import { ScenePlayer } from "../capture/ScenePlayer";
import { recordTab, type TabRecording } from "../capture/tabRecorder";
import { gameCamera } from "../gameCamera";
import { graphicsProfiles } from "../graphicsSettings";
import { savedBattle } from "../savedMaps";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";

const CAPTURES = import.meta.glob<unknown>("../../../../fixtures/captures/*.json", {
  import: "default",
});

/** The frame rate asked of the tab capture without `fps`: the most the page draws. */
const RECORD_FPS = 60;

type CaptureResult = (TabRecording & { audio: boolean }) | { error: string };

interface CaptureHandoff {
  captureChunk?: (base64: string) => Promise<void>;
  captureDone?: (result: CaptureResult) => void;
}
const handoff = () => window as CaptureHandoff;

interface CaptureParams {
  capture: string;
  scene: number;
  cards: boolean;
  record: boolean;
  fps: number;
}

function captureParams(search: string): CaptureParams {
  const params = new URLSearchParams(search);
  return {
    capture: params.get("capture") ?? "",
    scene: Number(params.get("scene") ?? 0),
    cards: params.get("cards") !== "0",
    record: params.get("record") === "1",
    fps: Number(params.get("fps") ?? RECORD_FPS),
  };
}

/** Scene `index` of fixtures/captures/`name`.json, or a refusal naming why not. */
async function captureScene(name: string, index: number): Promise<CaptureScene> {
  const load = CAPTURES[`../../../../fixtures/captures/${name}.json`];
  if (!load) throw new Error(`no capture "${name}" in fixtures/captures/`);
  const { scenes } = validateCapture(await load(), listMaps());
  const scene = scenes[index];
  if (!Number.isInteger(index) || !scene)
    throw new Error(`capture ${name} has ${scenes.length} scene(s), not scene ${index}`);
  return scene;
}

/** Tell the CLI how the recording ended. Without it, a failure still has to
 *  be loud. */
function finish(result: CaptureResult) {
  const done = handoff().captureDone;
  if (done) done(result);
  else if ("error" in result) console.error(`The capture failed: ${result.error}`);
}

const describe = (error: unknown) =>
  error instanceof Error ? `${error.name}: ${error.message}` : String(error);

/** `blob`'s bytes as base64, the only thing an exposed function carries well. */
function base64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      // The type has commas of its own (`codecs=vp9,opus`); the data follows the marker.
      const url = reader.result as string;
      resolve(url.slice(url.indexOf(";base64,") + ";base64,".length));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

export default function CapturePage() {
  const [params] = useState(() => captureParams(window.location.search));
  const { rules } = useSessionCatalog();
  const refused = useSyncExternalStore(appResources.subscribe, appResources.error);
  const [battleError, setBattleError] = useState<string | null>(null);
  const built = useBuiltScenario(params, async (_, p) => {
    const scene = await captureScene(p.capture, p.scene);
    const { scenario } = await savedBattle(scene.map, scene.encounter, rules);
    return { scene, scenario };
  });
  const failure = (buildFailed(built) ? built.error : null) ?? refused ?? battleError;
  useEffect(() => {
    if (failure && params.record) finish({ error: failure });
  }, [failure, params.record]);
  if (failure)
    return (
      <main className="lab-rejected" data-testid="error">
        {failure}
      </main>
    );
  if (!built || buildFailed(built)) return null;
  return <CaptureStage {...built} params={params} onError={setBattleError} />;
}

function CaptureStage({
  scene,
  scenario,
  params,
  onError,
}: {
  scene: CaptureScene;
  scenario: string;
  params: CaptureParams;
  onError: (message: string) => void;
}) {
  const stage = useRef<HTMLDivElement>(null);
  const [run] = useState(() => {
    let start = () => {};
    let end = () => {};
    const started = new Promise<void>((resolve) => (start = resolve));
    const ended = new Promise<void>((resolve) => (end = resolve));
    // Only a recording holds the battle at its warm tick.
    if (!params.record) start();
    return { started, start, ended, end, recording: false };
  });
  const record = () => {
    if (!params.record || run.recording) return;
    run.recording = true;
    const { captureChunk } = handoff();
    const recording = !captureChunk
      ? Promise.reject(
          new Error("record=1, but the page has no captureChunk; the CLI exposes it first"),
        )
      : !stage.current
        ? Promise.reject(new Error("the stage is not mounted"))
        : recordTab(stage.current, {
            fps: params.fps,
            until: run.ended,
            onStart: run.start,
            onChunk: async (chunk) => captureChunk(await base64(chunk)),
          });
    // `recordTab` refuses a capture without the tab's sound.
    recording.then(
      (result) => finish({ ...result, audio: true }),
      (error: unknown) => finish({ error: describe(error) }),
    );
  };
  return (
    <ScenePlayer
      scene={scene}
      scenario={scenario}
      cards={params.cards}
      start={run.started}
      onWarm={record}
      onEnd={run.end}
      onError={onError}
      className="capture-stage"
      filmClassName="capture-stage-picture"
      stage={stage}
      cameraConfig={filmCamera(gameCamera.config, [scene.reel])}
      quality={graphicsProfiles.high}
      reducedMotion={false}
      sound
    />
  );
}
