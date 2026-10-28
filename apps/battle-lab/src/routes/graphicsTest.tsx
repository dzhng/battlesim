import { useEffect, useRef, useState } from "react";
import { Link, useSearchParams } from "react-router";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { REELS, type ReelWorkload } from "@web/battle/benchmark/reels";
import type { FrameCount, PresentationLayer } from "@web/battle/benchmark/visualPresentation";
import { listMaps } from "@web/maps/catalogue";
import game from "@fixtures/game.json";
import { filmCamera, validateCapture, type CaptureScene } from "../capture/capture";
import { savedBattle } from "../savedMaps";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { LabViewport } from "../LabViewport";
import { useLabLoading } from "../LabLoading";
import { appResources } from "../appResources";
import { createReelRun, type ReelPlate, type ReelResult } from "../benchmark/reelRun";
import { createReelReport, type ReelReport } from "../benchmark/reelReport";
import { FrameChart } from "../benchmark/FrameChart";
import { graphicsProfiles, type GraphicsTier } from "../graphicsSettings";

/** What each reel plays, for the page's lead. */
const REEL_SUBJECTS: Record<keyof typeof REELS, string> = {
  menu: "the complete main-menu battle sequence",
  water: "the water reel on the wetlands map",
};

/** The reel `?capture=<name>` names (the menu's without it), its scenes
 *  admitted against the map catalogue, or the refusal naming what the test
 *  films. */
function chooseReel(name: string | null) {
  const id = name ?? "menu";
  if (!Object.hasOwn(REELS, id))
    return `The graphics test films the ${Object.keys(REELS).join(" or ")} reel; not ${id}.`;
  const workload: ReelWorkload = REELS[id as keyof typeof REELS];
  const scenes = validateCapture({ scenes: workload.scenes }, listMaps()).scenes;
  return {
    workload,
    subject: REEL_SUBJECTS[id as keyof typeof REELS],
    scenes,
    // The rig films every scene's closest framing, however few scenes run.
    cameraConfig: filmCamera(
      gameCamera.config,
      scenes.map((s) => s.reel),
    ),
    // A renderer or resource stall must produce a terminal result, while a
    // valid reel gets its authored shot duration plus one minute of setup margin.
    timeoutMs:
      scenes.reduce(
        (total, scene) =>
          total + scene.reel.shots.reduce((seconds, shot) => seconds + shot.seconds, 0),
        0,
      ) *
        1000 +
      60_000,
  };
}
type Reel = Exclude<ReturnType<typeof chooseReel>, string>;

/** What the reel's viewport is handed: the battle, with no orders, and with
 *  no fog of war unless `?fog=1` asks for the side's. */
const REEL_LAYERS: readonly PresentationLayer[] = [
  "sky",
  "terrain",
  "water",
  "grass",
  "scenery",
  "props",
  "structures",
  "buildings",
  "models",
  "effects",
  "scars",
];

/** The scenes of `reel` that `?scenes=<map,…>` names, in that order (every
 *  scene without it), or the refusal naming a map the reel does not film. */
function chooseScenes(reel: Reel | string, names: string | null): readonly CaptureScene[] | string {
  if (typeof reel === "string") return reel;
  const { scenes } = reel;
  if (names === null) return scenes;
  const chosen = names.split(",").map((name) => scenes.find((s) => s.map === name) ?? name);
  const unknown = chosen.filter((s) => typeof s === "string");
  if (unknown.length > 0 || chosen.length === 0)
    return `The graphics test films ${scenes.map((s) => s.map).join(", ")}; not ${unknown.join(", ")}.`;
  return chosen as CaptureScene[];
}

/** The page's state for scripted runs. */
interface GraphicsTestState {
  stage: "ready" | "running" | "results" | "refused";
  report: ReelReport | null;
  /** Each scene's control plates (`?plates=1`), in run order. */
  plates: readonly (ReelPlate & { scene: string })[];
  /** Each scene's counted frames (`?counts=1`), in run order. */
  counts: readonly { scene: string; frames: readonly FrameCount[] }[];
}

declare global {
  interface Window {
    __graphicsTest?: GraphicsTestState;
  }
}

export default function GraphicsTest() {
  const [params] = useSearchParams();
  const [reel] = useState(() => chooseReel(params.get("capture")));
  const [chosen] = useState(() => chooseScenes(reel, params.get("scenes")));
  const [plates] = useState(() => params.get("plates") === "1");
  const [counts] = useState(() => params.get("counts") === "1");
  const [fog] = useState(() => params.get("fog") === "1");
  const [quality] = useState<GraphicsTier | undefined>(() => {
    const value = params.get("quality");
    return value === "low" || value === "medium" || value === "high" ? value : undefined;
  });
  const [stage, setStage] = useState<"ready" | "running" | "results">("ready");
  const [results, setResults] = useState<ReelResult[]>([]);
  const [failure, setFailure] = useState<string | null>(null);
  const run = typeof chosen === "string" ? [] : chosen;
  const reportOf = (done: readonly ReelResult[], reason: string | null) =>
    createReelReport(done, {
      reel: typeof reel === "string" ? undefined : reel.workload,
      maps: run.map((s) => s.map),
      plates,
      layers: fog ? [...REEL_LAYERS, "fog"] : REEL_LAYERS,
      interruption: reason ? { status: "failed", reason } : undefined,
    });
  const report = stage === "results" ? reportOf(results, failure) : null;
  const publish = (
    state: Omit<GraphicsTestState, "plates" | "counts">,
    done: readonly ReelResult[],
  ) => {
    window.__graphicsTest = {
      ...state,
      plates: done.flatMap((r) => r.plates.map((p) => ({ ...p, scene: r.scene.map }))),
      counts: done.map((r) => ({ scene: r.scene.map, frames: r.counts })),
    };
  };
  useLabLoading("renderer", stage !== "running" ? true : null);
  useEffect(() => {
    publish({ stage: typeof chosen === "string" ? "refused" : stage, report }, results);
  });
  // A resource refusal replaces this page with the app's failure screen
  // before it can render again: the run's failure is published as it lands.
  useEffect(() => {
    const unsubscribe = appResources.subscribe(() => {
      const reason = appResources.error();
      if (reason) publish({ stage: "results", report: reportOf(results, reason) }, results);
    });
    return () => void unsubscribe();
  });
  const start = () => {
    setResults([]);
    setFailure(null);
    setStage("running");
  };
  const done = (result: ReelResult) => {
    const next = [...results, result];
    setResults(next);
    if (result.outcome.status !== "complete" || next.length === run.length) setStage("results");
  };
  const timeoutMs = typeof reel === "string" ? 0 : reel.timeoutMs;
  useEffect(() => {
    if (stage !== "running") return;
    const timer = window.setTimeout(() => {
      setFailure(`Graphics test did not complete within ${timeoutMs / 1000} s.`);
      setStage("results");
    }, timeoutMs);
    return () => window.clearTimeout(timer);
  }, [stage, timeoutMs]);
  if (stage === "running")
    return (
      <ReelScene
        key={results.length}
        scene={run[results.length]}
        cameraConfig={(reel as Reel).cameraConfig}
        plates={plates}
        counts={counts}
        fog={fog}
        quality={quality}
        onDone={done}
        onFailure={(reason) => {
          setFailure(reason);
          setStage("results");
        }}
      />
    );
  const format = (value?: number) => (value === undefined ? "—" : value.toFixed(1));
  return (
    <main className="bench-results" data-testid="graphics-test">
      <div className="bench-body">
        <header>
          <p className="bench-eyebrow">Battle · Graphics</p>
          <h1>
            {report
              ? failure || report.outcome.status !== "complete"
                ? "Incomplete test"
                : "Graphics test result"
              : "Graphics test"}
          </h1>
        </header>
        <p className="bench-sub">
          Play {typeof reel === "string" ? "a reel" : reel.subject} at your current resolution.
          Loading between scenes is excluded.
        </p>
        {typeof chosen === "string" && <p role="alert">{chosen}</p>}
        {failure && <p role="alert">{failure}</p>}
        {report && (
          <>
            <dl className="bench-tiles">
              {(
                [
                  ["Average FPS", report.frameRate?.average],
                  ["1% low FPS", report.frameRate?.low1],
                  ["Minimum FPS", report.frameRate?.minimum],
                  ["Maximum FPS", report.frameRate?.maximum],
                ] as const
              ).map(([label, value]) => (
                <div key={label}>
                  <dt>{label}</dt>
                  <dd>{format(value)}</dd>
                </div>
              ))}
            </dl>
            <FrameChart report={report} />
            <p className="bench-note">{report.measurement}</p>
            <button type="button" onClick={() => downloadReelEvidence(report)}>
              Download evidence JSON
            </button>
          </>
        )}
      </div>
      <nav className="bench-actions" aria-label="Graphics test actions">
        {typeof chosen !== "string" && (
          <button type="button" onClick={start}>
            {report ? "Run again" : "Run graphics test"}
          </button>
        )}
        <Link to="/">Main menu</Link>
      </nav>
    </main>
  );
}

function ReelScene({
  scene,
  cameraConfig,
  plates,
  counts,
  fog,
  quality,
  onDone,
  onFailure,
}: {
  scene: CaptureScene;
  cameraConfig: Reel["cameraConfig"];
  plates: boolean;
  counts: boolean;
  fog: boolean;
  quality?: GraphicsTier;
  onDone: (r: ReelResult) => void;
  onFailure: (reason: string) => void;
}) {
  const { rules } = useSessionCatalog();
  const built = useBuiltScenario(
    scene,
    async (_, s) => (await savedBattle(s.map, s.encounter, rules)).scenario,
  );
  const error = buildFailed(built) ? built.error : null;
  useEffect(() => {
    if (error) onFailure(error);
  }, [error, onFailure]);
  if (!built || buildFailed(built)) return null;
  return (
    <ReelBattle
      scene={scene}
      cameraConfig={cameraConfig}
      scenario={built}
      plates={plates}
      counts={counts}
      fog={fog}
      quality={quality}
      onDone={onDone}
    />
  );
}

function ReelBattle({
  scene,
  cameraConfig,
  scenario,
  plates,
  counts,
  fog,
  quality,
  onDone,
}: {
  scene: CaptureScene;
  cameraConfig: Reel["cameraConfig"];
  scenario: string;
  plates: boolean;
  /** Count what chosen frames drew (`createReelRun`'s `counts`). */
  counts: boolean;
  /** The side's fog of war (the reel films without it). */
  fog: boolean;
  quality?: GraphicsTier;
  onDone: (r: ReelResult) => void;
}) {
  const veil = useRef<HTMLDivElement>(null);
  const [run] = useState(() => createReelRun(scene, game.tick_hz, onDone, { plates, counts }));
  const session = useBattleSession({
    scenario,
    seed: scene.seed,
    scripted: run.scripted,
    inputEnabled: false,
    xray: false,
  });
  run.subject.unitAt = (id) =>
    session.sim.latest.current?.own.find((u) => u.id === id)?.position ?? null;
  useEffect(() => {
    if (session.sim.error) run.fail(session.sim.error);
  }, [run, session.sim.error]);
  useEffect(() => {
    const interrupt = () => {
      if (document.hidden) run.fail("The test was interrupted when the window became hidden.");
    };
    document.addEventListener("visibilitychange", interrupt);
    return () => document.removeEventListener("visibilitychange", interrupt);
  }, [run]);
  useLabLoading("renderer", !!session.meshes);
  const world = useFeed(session.meshes);
  const [pilot] = useState(() => ({
    ...run.pilot,
    pose(now: number) {
      if (veil.current) veil.current.style.opacity = String(run.opacity(now));
      return run.pilot.pose(now);
    },
  }));
  const first = scene.reel.shots[0].from;
  if (!session.meshes || session.sim.error) return null;
  return (
    <>
      <LabViewport
        fixture="graphics-test"
        inputEnabled={false}
        world={world}
        {...session.known}
        {...session.viewport}
        fog={fog ? session.fogFeed : undefined}
        initialCamera={{
          ...first,
          target: [first.target[0], first.target[1], 0],
          ...gameCamera.lens,
        }}
        groundAt={session.surfaceZ}
        pilot={pilot}
        cameraConfig={cameraConfig}
        quality={quality ? graphicsProfiles[quality] : undefined}
      />
      <div className="menu-backdrop-veil" ref={veil} style={{ opacity: 1 }} />
      <div role="status" className="bench-progress graphics-test-progress">
        <span>Graphics test · {scene.map}</span>
        <button type="button" onClick={run.cancel}>
          Cancel
        </button>
      </div>
    </>
  );
}

function downloadReelEvidence(report: ReelReport): void {
  const body = JSON.stringify(report, null, 2);
  const url = URL.createObjectURL(new Blob([body], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = `graphics-test-${report.canonical.fingerprint}.json`;
  document.body.appendChild(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}
