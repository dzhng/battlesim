// /benchmark: choose a run length, run the scripted battle while the camera
// flies its tour, then read the results. The page publishes its state on
// `window.__benchmark` for the scene harness.
import { useEffect, useState } from "react";
import game from "@fixtures/game.json";
import { villageScenario } from "../savedMaps";
import { frameCostRow, type BenchmarkReport } from "@web/battle/benchmark/report";
import { VILLAGE_CONTACT, type BenchmarkLength } from "@web/battle/benchmark/scenario";
import { BattleView } from "../BattleView";
import { BenchmarkResults } from "../benchmark/BenchmarkResults";
import { createBenchmarkRun, type BenchmarkRun } from "../benchmark/run";
import { useBuiltScenario } from "../useBuiltScenario";
import { gameCamera } from "../gameCamera";

const SCENARIO = VILLAGE_CONTACT;

type Stage =
  | { kind: "choose" }
  | { kind: "run"; length: BenchmarkLength; n: number }
  | { kind: "results"; report: BenchmarkReport };

declare global {
  interface Window {
    __benchmark?: {
      stage: Stage["kind"];
      report: BenchmarkReport | null;
      frameCostRow: (slice: string) => string | null;
    };
  }
}

export default function BenchmarkPage() {
  const [stage, setStage] = useState<Stage>({ kind: "choose" });
  const [runs, setRuns] = useState(0);
  const start = (length: BenchmarkLength) => {
    setRuns((n) => n + 1);
    setStage({ kind: "run", length, n: runs + 1 });
  };
  useEffect(() => {
    const report = stage.kind === "results" ? stage.report : null;
    window.__benchmark = {
      stage: stage.kind,
      report,
      frameCostRow: (slice) => (report ? frameCostRow(report, slice) : null),
    };
  }, [stage]);

  if (stage.kind === "results")
    return <BenchmarkResults report={stage.report} onAgain={() => start(stage.report.length)} />;
  if (stage.kind === "run")
    return (
      <BenchmarkBattle
        key={stage.n}
        length={stage.length}
        onDone={(report) => setStage({ kind: "results", report })}
      />
    );
  const s = (ms: number) => (ms >= 120_000 ? `${ms / 60_000} minutes` : `${ms / 1000} seconds`);
  return (
    <main className="menu">
      <div className="menu-body">
        <p className="bench-eyebrow">Battle</p>
        <h1>Benchmark</h1>
        <p className="menu-lede">
          The village battle is stepped to heavy contact (tick{" "}
          {SCENARIO.startTick.toLocaleString("en-US")}), with blue on its supported script against
          the defender. The camera then flies a fixed tour: strategic, pan, zoom, ground, combined
          and return. Input is off while it runs.
        </p>
        <div className="menu-choices">
          <button type="button" onClick={() => start("full")}>
            Full run <span>{s(SCENARIO.durationMs.full)}</span>
          </button>
          <button type="button" onClick={() => start("short")}>
            Short run <span>{s(SCENARIO.durationMs.short)}</span>
          </button>
        </div>
        <p className="menu-foot">
          {SCENARIO.id} v{SCENARIO.version} · seed {SCENARIO.seed} · tour {SCENARIO.tour.version} ·{" "}
          <a href="/">Main menu</a>
        </p>
      </div>
    </main>
  );
}

function BenchmarkBattle({
  length,
  onDone,
}: {
  length: BenchmarkLength;
  onDone: (report: BenchmarkReport) => void;
}) {
  const scenario = useBuiltScenario(SCENARIO.variant, villageScenario);
  // One run per mount: the page remounts for another.
  const [run] = useState<BenchmarkRun>(() =>
    createBenchmarkRun(SCENARIO, length, game.tick_hz, onDone),
  );
  if (!scenario) return null;
  if (typeof scenario !== "string")
    return (
      <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
        the village scenario could not be built: {scenario.error}
      </main>
    );
  return (
    <BattleView
      fixture="benchmark"
      scenario={scenario}
      seed={SCENARIO.seed}
      scripted={run.scripted}
      camera={gameCamera.opening()}
      status={(session) => (
        <Progress run={run} tick={session.sim.observation?.tick ?? 0} error={session.sim.error} />
      )}
    />
  );
}

function Progress({ run, tick, error }: { run: BenchmarkRun; tick: number; error: string | null }) {
  const status = run.status();
  useEffect(() => {
    if (error) run.fail(error);
  }, [error, run]);
  return (
    <div data-testid="benchmark-progress" role="status" className="bench-progress">
      {status.stage === "preparing" ? (
        <span>
          Preparing the battle · tick {tick.toLocaleString("en-US")} /{" "}
          {SCENARIO.startTick.toLocaleString("en-US")}
        </span>
      ) : (
        <span>
          {Math.floor(status.elapsedMs / 1000)} / {status.durationMs / 1000} s · {status.phase}
        </span>
      )}
      <button type="button" onClick={run.cancel} disabled={status.stage === "finishing"}>
        Cancel
      </button>
    </div>
  );
}
