// Frame time over the run: each column's slowest and fastest frame as a bar,
// its mean frame and mean GPU total as lines, the tour's phases as bands and
// the 120/60/30 FPS cadences as guides.
import type { BenchmarkReport } from "@web/battle/benchmark/report";

const W = 1000;
const H = 240;
const COLUMNS = 250;
/** Frame-time guides; the 30 FPS line is slice 27's floor. */
const GUIDES = [
  { ms: 1000 / 120, label: "120 FPS" },
  { ms: 1000 / 60, label: "60 FPS" },
  { ms: 1000 / 30, label: "30 FPS floor" },
];
const FLOOR_MS = 1000 / 30;

interface Column {
  min: number;
  max: number;
  mean: number;
}

function columns(report: BenchmarkReport, span: number): (Column | null)[] {
  const acc = Array.from({ length: COLUMNS }, () => ({
    min: Infinity,
    max: 0,
    sum: 0,
    n: 0,
  }));
  for (const f of report.samples.frames) {
    const c = acc[Math.min(COLUMNS - 1, Math.floor((f.elapsedMs / span) * COLUMNS))];
    c.min = Math.min(c.min, f.intervalMs);
    c.max = Math.max(c.max, f.intervalMs);
    c.sum += f.intervalMs;
    c.n++;
  }
  return acc.map((c) => (c.n === 0 ? null : { min: c.min, max: c.max, mean: c.sum / c.n }));
}

export function FrameChart({ report }: { report: BenchmarkReport }) {
  const span = report.durationMs;
  // The scale fits the run's slow frames with headroom, so the data fills
  // the plot; a guide above it is named in the caption instead.
  const top = Math.min(100, Math.max(15, Math.ceil(((report.frameMs?.p99 ?? 0) * 1.6) / 5) * 5));
  const guides = GUIDES.filter((g) => g.ms < top);
  const step = top <= 20 ? 5 : top <= 50 ? 10 : 20;
  const ticks = Array.from({ length: Math.floor(top / step) + 1 }, (_, k) => k * step);
  const over = report.samples.frames.filter((f) => f.intervalMs > top).length;
  const y = (ms: number) => H - (Math.min(ms, top) / top) * H;
  const x = (i: number) => ((i + 0.5) / COLUMNS) * W;
  const cols = columns(report, span);
  const line = (pick: (c: Column) => number | null) =>
    cols
      .flatMap((c, i) => {
        const v = c && pick(c);
        return v === null || v === undefined ? [] : [`${x(i).toFixed(1)},${y(v).toFixed(1)}`];
      })
      .join(" ");
  return (
    <figure className="bench-chart">
      <div className="bench-chart-plot">
        <svg
          viewBox={`0 0 ${W} ${H}`}
          preserveAspectRatio="none"
          role="img"
          aria-label="Frame time over the run"
        >
          {report.tour.phases.map((p, i) => (
            <rect
              key={p.name}
              x={p.from * W}
              y={0}
              width={(p.to - p.from) * W}
              height={H}
              className={i % 2 ? "bench-band-odd" : "bench-band-even"}
            />
          ))}
          {ticks.map((ms) => (
            <line
              key={ms}
              x1={0}
              x2={W}
              y1={y(ms)}
              y2={y(ms)}
              className="bench-grid"
              vectorEffect="non-scaling-stroke"
            />
          ))}
          {cols.map((c, i) =>
            c ? (
              <line
                key={i}
                x1={x(i)}
                x2={x(i)}
                y1={y(c.min)}
                y2={Math.min(y(c.max), y(c.min) - 1)}
                className="bench-range"
                vectorEffect="non-scaling-stroke"
              />
            ) : null,
          )}
          <polyline
            points={line((c) => c.mean)}
            className="bench-mean"
            vectorEffect="non-scaling-stroke"
          />
          <polyline
            points={report.samples.stats
              .flatMap((s) =>
                s.gpu && s.gpu.frames > 0
                  ? [`${((s.elapsedMs / span) * W).toFixed(1)},${y(s.gpu.meanMs).toFixed(1)}`]
                  : [],
              )
              .join(" ")}
            className="bench-gpu"
            vectorEffect="non-scaling-stroke"
          />
          {/* Cadence guides over the data, so they stay visible. */}
          {guides.map((g) => (
            <line
              key={g.label}
              x1={0}
              x2={W}
              y1={y(g.ms)}
              y2={y(g.ms)}
              className={g.ms === FLOOR_MS ? "bench-guide bench-guide-floor" : "bench-guide"}
              vectorEffect="non-scaling-stroke"
            />
          ))}
          {cols.map((c, i) =>
            c && c.max > top ? (
              <polygon
                key={`over-${i}`}
                points={`${x(i) - 4},10 ${x(i) + 4},10 ${x(i)},0`}
                className="bench-over"
              />
            ) : null,
          )}
        </svg>
        <div className="bench-chart-axis">
          {ticks.map((ms) => (
            <span
              key={ms}
              style={{
                top: `${(y(ms) / H) * 100}%`,
                // The extreme ticks sit inside the plot's edges.
                transform: ms === 0 ? "translateY(-100%)" : ms === top ? "none" : undefined,
              }}
            >
              {ms} ms
            </span>
          ))}
          {guides.map((g) => (
            <span
              key={g.label}
              className="bench-axis-guide"
              style={{ top: `${(y(g.ms) / H) * 100}%` }}
            >
              {g.label}
            </span>
          ))}
        </div>
      </div>
      <div className="bench-chart-phases" aria-hidden>
        <div>
          {report.tour.phases.map((p) => (
            <span
              key={p.name}
              style={{ left: `${p.from * 100}%`, width: `${(p.to - p.from) * 100}%` }}
            >
              {p.name}
            </span>
          ))}
        </div>
      </div>
      <figcaption>
        <span>
          Frame time over the {report.durationMs / 1000} s run, in {COLUMNS} time slices:
        </span>
        <span className="bench-key bench-key-range" /> fastest to slowest frame in the slice
        <span className="bench-key bench-key-mean" /> mean frame
        <span className="bench-key bench-key-gpu" /> GPU frame total (mean over the last 240 frames,
        every 2 s)
        <span className="bench-key bench-key-guide" /> 120/60 FPS cadence
        {top > FLOOR_MS ? (
          <>
            <span className="bench-key bench-key-floor" /> 30 FPS floor
          </>
        ) : (
          <span>The 30 FPS floor (33.3 ms) is above this scale.</span>
        )}
        {over > 0 && (
          <span>
            ▲ marks {over} frame{over === 1 ? "" : "s"} slower than {top} ms (up to{" "}
            {report.frameMs?.max.toFixed(1)} ms).
          </span>
        )}
      </figcaption>
    </figure>
  );
}
