import { useEffect, useRef, useState } from "react";
import { Link, useSearchParams } from "react-router";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { MENU_REEL_WORKLOAD } from "@web/battle/benchmark/menuReel";
import type { PresentationLayer } from "@web/battle/benchmark/visualPresentation";
import { listMaps } from "@web/maps/catalogue";
import game from "@fixtures/game.json";
import { filmCamera, validateBackdrop, type BackdropScene } from "../menuReel";
import { savedBattle } from "../savedMaps";
import { buildFailed, useBuiltScenario } from "../useBuiltScenario";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { LabViewport } from "../LabViewport";
import { useLabLoading } from "../LabLoading";
import { appResources } from "../appResources";
import type { UploadedTables } from "@packages/battle-renderer/src/frame/wordTables";
import { createReelRun, type ReelPlate, type ReelResult } from "../benchmark/reelRun";
import { createReelReport, type ReelReport } from "../benchmark/reelReport";
import { FrameChart } from "../benchmark/FrameChart";

const scenes = validateBackdrop(MENU_REEL_WORKLOAD, listMaps()).scenes;
// The rig films every scene's closest framing, however few scenes run.
const cameraConfig = filmCamera(
  gameCamera.config,
  scenes.map((s) => s.reel),
);
/** What the reel's viewport is handed: the battle, with no fog overlay and no orders. */
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

/** The scenes `?scenes=<map,…>` names, in that order (every scene without
 *  it), or the refusal naming a map the reel does not film. */
function chooseScenes(names: string | null): readonly BackdropScene[] | string {
  if (names === null) return scenes;
  const chosen = names.split(",").map((name) => scenes.find((s) => s.map === name) ?? name);
  const unknown = chosen.filter((s) => typeof s === "string");
  if (unknown.length > 0 || chosen.length === 0)
    return `The graphics test films ${scenes.map((s) => s.map).join(", ")}; not ${unknown.join(", ")}.`;
  return chosen as BackdropScene[];
}

/** The page's state for scripted runs. */
interface GraphicsTestState {
  stage: "ready" | "running" | "results" | "refused";
  report: ReelReport | null;
  /** Each scene's control plates (`?plates=1`), in run order. */
  plates: readonly (ReelPlate & { scene: string })[];
  /** Each plated scene's ground and grass tables, in run order. */
  tables: readonly { scene: string; tables: UploadedTables }[];
}

declare global {
  interface Window {
    __graphicsTest?: GraphicsTestState;
  }
}

export default function GraphicsTest() {
  const [params] = useSearchParams();
  const [chosen] = useState(() => chooseScenes(params.get("scenes")));
  const [plates] = useState(() => params.get("plates") === "1");
  const [stage, setStage] = useState<"ready" | "running" | "results">("ready");
  const [results, setResults] = useState<ReelResult[]>([]);
  const [failure, setFailure] = useState<string | null>(null);
  const run = typeof chosen === "string" ? [] : chosen;
  const reportOf = (done: readonly ReelResult[], reason: string | null) =>
    createReelReport(done, {
      scenes: run,
      plates,
      layers: REEL_LAYERS,
      interruption: reason ? { status: "failed", reason } : undefined,
    });
  const report = stage === "results" ? reportOf(results, failure) : null;
  const publish = (
    state: Omit<GraphicsTestState, "plates" | "tables">,
    done: readonly ReelResult[],
  ) => {
    window.__graphicsTest = {
      ...state,
      plates: done.flatMap((r) => r.plates.map((p) => ({ ...p, scene: r.scene.map }))),
      tables: done.flatMap((r) => (r.tables ? [{ scene: r.scene.map, tables: r.tables }] : [])),
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
  if (stage === "running")
    return (
      <ReelScene
        key={results.length}
        scene={run[results.length]}
        plates={plates}
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
          Play the complete main-menu battle sequence at your current resolution. Loading between
          scenes is excluded.
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
  plates,
  onDone,
  onFailure,
}: {
  scene: BackdropScene;
  plates: boolean;
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
  return <ReelBattle scene={scene} scenario={built} plates={plates} onDone={onDone} />;
}

function ReelBattle({
  scene,
  scenario,
  plates,
  onDone,
}: {
  scene: BackdropScene;
  scenario: string;
  plates: boolean;
  onDone: (r: ReelResult) => void;
}) {
  const veil = useRef<HTMLDivElement>(null);
  const [run] = useState(() => createReelRun(scene, game.tick_hz, onDone, { plates }));
  const session = useBattleSession({
    scenario,
    seed: scene.seed,
    scripted: run.scripted,
    destroyable: "apart",
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
        structures={session.structures}
        buildings={session.buildingsFeed}
        obstacles={session.cameraObstaclesFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={{
          ...first,
          target: [first.target[0], first.target[1], 0],
          ...gameCamera.lens,
        }}
        groundAt={session.surfaceZ}
        onReady={session.onReady}
        pilot={pilot}
        cameraConfig={cameraConfig}
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
