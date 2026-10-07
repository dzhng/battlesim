import { useSessionCatalog } from "@web/battle/catalog/context";
import { Link } from "react-router";
import { useLabLoading } from "../LabLoading";
// /benchmark: choose a run length, run the scripted battle while the camera
// flies its tour, then read the results. The page publishes its state on
// `window.__benchmark` for the scene harness.
import { useEffect, useState } from "react";
import game from "@fixtures/game.json";
import { frameCostRow, type BenchmarkReport } from "@web/battle/benchmark/report";
import {
  benchmarkPreset,
  type BenchmarkPreset,
  type BenchmarkLength,
} from "@web/battle/benchmark/presets";
import { BattleView } from "../BattleView";
import { BenchmarkResults } from "../benchmark/BenchmarkResults";
import { createBenchmarkRun, type BenchmarkRun } from "../benchmark/run";
import { useBuiltScenario } from "../useBuiltScenario";
import { gameCamera } from "../gameCamera";
import { prepareBenchmark, type BenchmarkBattle as BuiltBenchmark } from "../benchmark/prepare";
import { buildFailed } from "../useBuiltScenario";

type Stage =
  | { kind: "choose" }
  | { kind: "run"; length: BenchmarkLength; n: number }
  | { kind: "results"; report: BenchmarkReport };

declare global {
  interface Window {
    __benchmark?: {
      stage: Stage["kind"];
      preset: string | null;
      report: BenchmarkReport | null;
      frameCostRow: (slice: string) => string | null;
    };
  }
}

export default function BenchmarkPage() {
  const scenario = benchmarkPreset(new URLSearchParams(window.location.search).get("preset"));
  const [stage, setStage] = useState<Stage>({ kind: "choose" });
  useLabLoading(
    "renderer",
    stage.kind === "run" ? null : true,
    scenario ? null : "Unknown benchmark preset.",
  );
  const [runs, setRuns] = useState(0);
  const start = (length: BenchmarkLength) => {
    setRuns((n) => n + 1);
    setStage({ kind: "run", length, n: runs + 1 });
  };
  useEffect(() => {
    const report = stage.kind === "results" ? stage.report : null;
    window.__benchmark = {
      stage: stage.kind,
      preset: scenario?.id ?? null,
      report,
      frameCostRow: (slice) => (report ? frameCostRow(report, slice) : null),
    };
  }, [stage, scenario]);

  if (!scenario)
    return (
      <main className="lab-rejected" data-testid="error">
        Unknown benchmark preset.
      </main>
    );

  if (stage.kind === "results")
    return <BenchmarkResults report={stage.report} onAgain={() => start(stage.report.length)} />;
  if (stage.kind === "run")
    return (
      <BenchmarkBattle
        key={stage.n}
        length={stage.length}
        workload={scenario}
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
          Local-contact stress on the complete generated world: Metro Large seed 4, with 100 units
          a side in the simulation&apos;s central city arena and both sides on its seeded orders.
          Player transit is a separate workload.{" "}
          Timing starts at tick {scenario.startTick.toLocaleString("en-US")}. The camera then flies
          a fixed tour: strategic, pan, zoom, ground, combined and return. Input is off while it
          runs.
        </p>
        <div className="menu-choices">
          <button type="button" onClick={() => start("full")}>
            Full run <span>{s(scenario.durationMs.full)}</span>
          </button>
          <button type="button" onClick={() => start("short")}>
            Short run <span>{s(scenario.durationMs.short)}</span>
          </button>
        </div>
        <p className="menu-foot">
          {scenario.id} v{scenario.version} · seed {scenario.seed} · tour{" "}
          {scenario.tour} ·{" "}
          <Link to="/">Main menu</Link>
        </p>
      </div>
    </main>
  );
}

function BenchmarkBattle({
  length,
  workload,
  onDone,
}: {
  length: BenchmarkLength;
  workload: BenchmarkPreset;
  onDone: (report: BenchmarkReport) => void;
}) {
  const { rules } = useSessionCatalog();
  const built = useBuiltScenario(workload, (wasm, w, signal) =>
    prepareBenchmark(wasm, w, signal, rules),
  );
  if (!built) return null;
  if (buildFailed(built))
    return (
      <main className="lab-rejected" data-testid="error">
        The benchmark could not be prepared: {built.error}
      </main>
    );
  return <BenchmarkSession battle={built} length={length} onDone={onDone} />;
}

function BenchmarkSession({
  battle,
  length,
  onDone,
}: {
  battle: BuiltBenchmark;
  length: BenchmarkLength;
  onDone: (report: BenchmarkReport) => void;
}) {
  const { workload, prepared } = battle;
  // One run per mount: the page remounts for another.
  const [run] = useState<BenchmarkRun>(() =>
    createBenchmarkRun(workload, length, game.tick_hz, onDone, prepared?.report),
  );
  return (
    <BattleView
      fixture="benchmark"
      scenario={battle.scenario}
      prepared={prepared}
      seed={workload.seed}
      scripted={run.scripted}
      camera={gameCamera.opening()}
      cameraConfig={
        prepared && gameCamera.forMap(prepared.report.size, prepared.report.extents.rendered)
      }
      status={(session) => (
        <Progress
          run={run}
          startTick={workload.startTick}
          tick={session.sim.observation?.tick ?? 0}
          error={session.sim.error}
        />
      )}
      diagnostics={() => ({ preparation: () => prepared?.report ?? null })}
    />
  );
}

function Progress({
  run,
  startTick,
  tick,
  error,
}: {
  run: BenchmarkRun;
  startTick: number;
  tick: number;
  error: string | null;
}) {
  const status = run.status();
  useEffect(() => {
    if (error) run.fail(error);
  }, [error, run]);
  return (
    <div data-testid="benchmark-progress" role="status" className="bench-progress">
      {status.stage === "preparing" ? (
        <span>
          Preparing the battle · tick {tick.toLocaleString("en-US")} /{" "}
          {startTick.toLocaleString("en-US")}
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
