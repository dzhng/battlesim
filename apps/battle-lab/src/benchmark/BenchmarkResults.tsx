import { Link } from "react-router";
// The benchmark's results screen: the headline frame numbers, the frame chart,
// percentiles per camera phase, then the simulation, memory and machine the
// numbers came from. Everything shown is read from the JSON report.
import type { BenchmarkReport } from "@web/battle/benchmark/report";
import type { Summary } from "@web/battle/benchmark/recording";
import { FrameChart } from "./FrameChart";

const fmt = (v: number | null | undefined, digits = 1) =>
  v === null || v === undefined ? "—" : v.toFixed(digits);
const mib = (bytes: number | null | undefined) =>
  bytes === null || bytes === undefined ? "—" : `${(bytes / 2 ** 20).toFixed(1)} MiB`;

function exportReport(report: BenchmarkReport) {
  const url = URL.createObjectURL(new Blob([JSON.stringify(report)], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = `benchmark-${report.scenario.id}-v${report.scenario.version}-${report.length}.json`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

const TITLE: Record<BenchmarkReport["outcome"]["status"], string> = {
  complete: "",
  cancelled: "Partial result",
  failed: "Run failed",
};

export function BenchmarkResults({
  report,
  onAgain,
}: {
  report: BenchmarkReport;
  onAgain: () => void;
}) {
  const { outcome, scenario, simulation, ticks, memory, identity } = report;
  const title =
    TITLE[outcome.status] ||
    (report.length === "short" ? "Short run result" : "Five-minute result");
  const headline: [string, string, string?][] = [
    ["Average FPS", fmt(report.averageFps)],
    ["Median frame", fmt(report.frameMs?.p50), "ms"],
    ["95th percentile", fmt(report.frameMs?.p95), "ms"],
    ["99th percentile", fmt(report.frameMs?.p99), "ms"],
    ["GPU frame, mean", fmt(report.gpu?.meanMs, 2), "ms"],
    ["Frames over 33 ms", String(report.framesOver33ms)],
  ];
  const cell = (s: Summary | null, pick: (s: Summary) => number, digits = 1) =>
    s ? fmt(pick(s), digits) : "—";
  return (
    <main className="bench-results" data-testid="benchmark-results">
      <div className="bench-body">
        <header>
          <p className="bench-eyebrow">Battle benchmark</p>
          <h1>{title}</h1>
          <p className="bench-sub">
            {outcome.reason} · {(report.recordedMs / 1000).toFixed(1)} s recorded · {scenario.id} v
            {scenario.version} · camera tour {scenario.cameraScript}
          </p>
        </header>

        <dl className="bench-tiles">
          {headline.map(([label, value, unit]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>
                {value}
                {unit && value !== "—" && <small> {unit}</small>}
              </dd>
            </div>
          ))}
        </dl>

        <FrameChart report={report} />
        <p className="bench-note">{report.measurement}</p>

        <h2>By camera phase</h2>
        <table className="bench-table">
          <thead>
            <tr>
              <th scope="col">Phase</th>
              <th scope="col">Frames</th>
              <th scope="col">Avg FPS</th>
              <th scope="col">p50 ms</th>
              <th scope="col">p95 ms</th>
              <th scope="col">p99 ms</th>
              <th scope="col">CPU p95 ms</th>
              <th scope="col">GPU frame ms</th>
            </tr>
          </thead>
          <tbody>
            {report.phases.map((p) => (
              <tr key={p.name}>
                <th scope="row">{p.name}</th>
                <td>{p.frameMs?.count ?? 0}</td>
                <td>{fmt(p.averageFps)}</td>
                <td>{cell(p.frameMs, (s) => s.p50)}</td>
                <td>{cell(p.frameMs, (s) => s.p95)}</td>
                <td>{cell(p.frameMs, (s) => s.p99)}</td>
                <td>{cell(p.cpuMs, (s) => s.p95, 2)}</td>
                <td>{fmt(p.gpu?.meanMs, 2)}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="bench-facts">
          <section>
            <h2>Simulation</h2>
            <dl>
              <dt>Ticks</dt>
              <dd>
                {simulation.startTick.toLocaleString("en-US")} →{" "}
                {simulation.endTick.toLocaleString("en-US")} ({fmt(simulation.simulatedSeconds)} s
                simulated)
              </dd>
              <dt>Step time</dt>
              <dd>
                p50 {cell(ticks.stepMs, (s) => s.p50, 2)} · p95{" "}
                {cell(ticks.stepMs, (s) => s.p95, 2)} · max {cell(ticks.stepMs, (s) => s.max, 2)} ms
              </dd>
              <dt>Publication</dt>
              <dd>
                {ticks.publicationBytes
                  ? `${Math.round(ticks.publicationBytes.mean).toLocaleString("en-US")} B mean · ${ticks.publicationBytes.max.toLocaleString("en-US")} B max`
                  : "—"}
              </dd>
              <dt>Battle</dt>
              <dd>
                {scenario.variant} · seed {scenario.seed} · blue{" "}
                <span className="bench-nowrap">{scenario.blue}</span> · red {scenario.red}
              </dd>
            </dl>
          </section>
          <section>
            <h2>Memory</h2>
            <dl>
              <dt>GPU buffers</dt>
              <dd>
                {mib(memory.last?.bufferBytes)} in {memory.last?.buffers ?? "—"} (peak{" "}
                {mib(memory.peakBufferBytes)})
              </dd>
              <dt>GPU textures</dt>
              <dd>
                {mib(memory.last?.textureBytes)} in {memory.last?.textures ?? "—"}
              </dd>
              <dt>JS heap</dt>
              <dd>
                {memory.peakHeapBytes === null
                  ? "not reported"
                  : `${mib(memory.last?.heapBytes)} at the end (peak ${mib(memory.peakHeapBytes)})`}
              </dd>
              <dt>Main thread</dt>
              <dd>
                CPU p50 {cell(report.cpuMs, (s) => s.p50, 2)} · p95{" "}
                {cell(report.cpuMs, (s) => s.p95, 2)} ms per frame
              </dd>
            </dl>
          </section>
          <section>
            <h2>Machine</h2>
            <dl>
              <dt>GPU</dt>
              <dd>{identity?.adapter || "—"}</dd>
              <dt>Window</dt>
              <dd>{identity ? `${identity.viewport.join(" × ")} at DPR ${identity.dpr}` : "—"}</dd>
              <dt>GPU timing</dt>
              <dd>{identity?.timestampQuery ? "timestamp-query frame total" : "unavailable"}</dd>
            </dl>
          </section>
        </div>
      </div>
      <nav className="bench-actions" aria-label="Benchmark actions">
        <button type="button" onClick={onAgain}>
          Run again
        </button>
        <button type="button" onClick={() => exportReport(report)}>
          Export JSON
        </button>
        <Link to="/">Main menu</Link>
      </nav>
    </main>
  );
}
