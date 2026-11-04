// The main menu's backdrop: live battles on saved battlefields, one scene
// after another, each filmed by its reel's camera moves and graded to the
// menu's amber. Silent (the menu's music plays over it) and inert: input
// belongs to the menu. Each is the capture catalog's battle, so leaving the menu
// releases its worker and scene like any battle's; when a scene's reel ends
// the next scene's battle starts behind the veil (its saved scenario fetched
// while the last one played), and after the last the first starts again.
// Its preparation reports to the menu's loading screen, and the battle holds
// at its warm tick until the menu is shown, so the reel's first cut opens on
// the moment it was cut for.
import { Suspense, useEffect, useRef, useState, useSyncExternalStore, type RefObject } from "react";
import type { ResolvedRules } from "@web/battle/catalog/compose";
import { SessionCatalogScope, useSessionCatalog } from "@web/battle/catalog/context";
import menuCapture from "@fixtures/captures/menu.json";
import { listMaps } from "@web/maps/catalogue";
import { appResources } from "./appResources";
import { gameCamera } from "./gameCamera";
import { filmCamera, validateCapture, type CaptureScene } from "./capture/capture";
import { ScenePlayer } from "./capture/ScenePlayer";
import { savedBattle } from "./savedMaps";
import { buildFailed, useBuiltScenario } from "./useBuiltScenario";
import { useLabLoading } from "./LabLoading";
import {
  graphicsProfiles,
  graphicsSettings,
  resolveEffectiveTier,
  type GraphicsTier,
} from "./graphicsSettings";
import {
  createCanonicalFrameRunner,
  graphicsDetectionState,
  registerGraphicsDetectionRunner,
  subscribeGraphicsDetection,
} from "./graphicsDetection";

const SCENES = validateCapture(menuCapture, listMaps()).scenes;
const FILM_CAMERA = filmCamera(
  gameCamera.config,
  SCENES.map((s) => s.reel),
);

/** Each scene's saved scenario under the capture catalog's `rules`, fetched
 *  once: the next scene's is asked for while the current one plays, so the
 *  cut to it waits on starting and warming its battle, not on the download. */
const scenarios = new Map<CaptureScene, Promise<string>>();
function sceneScenario(scene: CaptureScene, rules: ResolvedRules): Promise<string> {
  let s = scenarios.get(scene);
  if (!s) {
    s = savedBattle(scene.map, scene.encounter, rules).then((b) => b.scenario);
    // A failed preparation is asked again next time, not remembered.
    s.catch(() => scenarios.delete(scene));
    scenarios.set(scene, s);
  }
  return s;
}

interface BackdropProps {
  plate: RefObject<HTMLElement | null>;
  /** Settles when the menu is shown. */
  shown: Promise<void>;
  /** Releases the first-launch detection cover after the reset scene is ready. */
  onDetectionSceneReady?: () => void;
}

/** The backdrop's battles run on the capture catalog. */
export function MenuBackdrop(props: BackdropProps) {
  return (
    <Suspense fallback={null}>
      <SessionCatalogScope set="capture">
        <BackdropScenes {...props} />
      </SessionCatalogScope>
    </Suspense>
  );
}

function BackdropScenes({ plate, shown, onDetectionSceneReady }: BackdropProps) {
  const { rules } = useSessionCatalog();
  // A refused page GPU leaves the menu its plain background, with nothing to wait for.
  const refused = useSyncExternalStore(appResources.subscribe, appResources.error);
  const graphics = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  const detection = useSyncExternalStore(subscribeGraphicsDetection, graphicsDetectionState);
  const [probeTier, setProbeTier] = useState<GraphicsTier | null>(null);
  const [probeGeneration, setProbeGeneration] = useState(0);
  const [reelGeneration, setReelGeneration] = useState(0);
  const detectionWasRunning = useRef(false);
  const frameListeners = useRef(new Set<(timestampMs: number) => void>());
  const readyWaiter = useRef<{ resolve: () => void; reject: (reason: unknown) => void } | null>(
    null,
  );
  useEffect(() => {
    if (detection.status === "running") detectionWasRunning.current = true;
    if (detection.status === "complete" && detectionWasRunning.current) {
      // Detection ran over the live reel while the loading card covered it.
      // Start a fresh scene before lifting that card so the player always sees
      // the first framing, never a camera left halfway through a shot.
      detectionWasRunning.current = false;
      setReelGeneration((generation) => generation + 1);
    }
  }, [detection.status]);
  useEffect(() => {
    const runner = createCanonicalFrameRunner({
      setTier: (tier, signal) => {
        setProbeTier(tier);
        setProbeGeneration((generation) => generation + 1);
        return new Promise<void>((resolve, reject) => {
          const abort = () => reject(new DOMException("Detection cancelled", "AbortError"));
          if (signal.aborted) return abort();
          signal.addEventListener("abort", abort, { once: true });
          readyWaiter.current = {
            resolve: () => {
              signal.removeEventListener("abort", abort);
              resolve();
            },
            reject: (reason) => {
              signal.removeEventListener("abort", abort);
              reject(reason);
            },
          };
        });
      },
      subscribeFrames: (listener) => {
        frameListeners.current.add(listener);
        return () => frameListeners.current.delete(listener);
      },
      timeoutMs: 120_000,
    });
    const unregister = registerGraphicsDetectionRunner(async (tier, signal) => {
      try {
        return await runner(tier, signal);
      } finally {
        setProbeTier(null);
      }
    });
    return () => {
      unregister();
      readyWaiter.current?.reject(new DOMException("Detection cancelled", "AbortError"));
      readyWaiter.current = null;
      frameListeners.current.clear();
    };
  }, []);
  useLabLoading("renderer", refused ? true : null);
  // Scenes played so far: the one playing is `played` modulo their count.
  const [played, setPlayed] = useState(0);
  const scene = SCENES[played % SCENES.length];
  const quality =
    graphicsProfiles[probeTier ?? resolveEffectiveTier(graphics.mode, graphics.detectedMode)];
  const next = SCENES[(played + 1) % SCENES.length];
  const battle = useBuiltScenario(scene, (_, s) => sceneScenario(s, rules));
  const failed = buildFailed(battle) ? battle.error : null;
  useEffect(() => {
    if (failed) console.error(`The menu backdrop could not be prepared: ${failed}`);
  }, [failed]);
  if (refused || !battle || buildFailed(battle)) return null;
  return (
    <ScenePlayer
      key={`${played}:${reelGeneration}`}
      scene={scene}
      scenario={battle}
      cards={false}
      start={shown}
      onWarm={() => {
        void sceneScenario(next, rules).catch(() => {});
        if (detection.status === "complete") onDetectionSceneReady?.();
      }}
      onEnd={() => setPlayed((n) => n + 1)}
      className="menu-backdrop"
      filmClassName="menu-backdrop-film"
      plate={plate}
      cameraConfig={FILM_CAMERA}
      quality={quality}
      viewportKey={`graphics-probe-${probeGeneration}`}
      onViewportReady={() => readyWaiter.current?.resolve()}
      onFrame={(now) => frameListeners.current.forEach((listener) => listener(now))}
    >
      <div className="menu-backdrop-grade" />
    </ScenePlayer>
  );
}
