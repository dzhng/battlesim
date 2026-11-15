import { Link } from "react-router";
// The cover over a battle that is still being prepared: what is being made,
// how far along it is, and, if it cannot be made, why. In the HUD's look.
import { useEffect, useState, type ReactNode } from "react";
import { diagnosticsFailure } from "@web/diagnostics";
import { CopyDiagnostics } from "./DiagnosticsControls";
import {
  downloadRate,
  downloads,
  type DownloadProgress,
  type DownloadSample,
} from "@web/downloads";

export interface LoadingStage {
  id: string;
  label: string;
}

/** Loading stopped: what the player is told, what they can do about it, and
 *  the diagnostics a developer needs, shown only when asked for. */
export interface LoadingFailure {
  message: string;
  /** What was asked for stands as asked: the seed, and what to do next. */
  advice?: string;
  details: string[];
}

/** How far back the speed looks: long enough to smooth chunky arrivals,
 *  short enough to show a stall within a few seconds. */
const RATE_WINDOW_MS = 3000;

/** The page's downloads, sampled a few times a second (a byte count per
 *  chunk would redraw the screen hundreds of times a second), and how fast
 *  their bytes are arriving. */
function useDownloads(): { d: DownloadProgress; rate: number | null } {
  const [state, setState] = useState(() => ({ d: downloads.get(), rate: null as number | null }));
  useEffect(() => {
    let samples: DownloadSample[] = [];
    const id = setInterval(() => {
      const d = downloads.get();
      const at = performance.now();
      // A new burst counts from zero, so its samples start afresh.
      const last = samples[samples.length - 1];
      if (!d.active || (last && d.loaded < last.loaded)) samples = [];
      if (d.active) samples.push({ at, loaded: d.loaded });
      samples = samples.filter((s) => at - s.at <= RATE_WINDOW_MS);
      setState({ d, rate: downloadRate(samples) });
    }, 200);
    return () => clearInterval(id);
  }, []);
  return state;
}

const mb = (bytes: number) => (bytes / 1e6).toFixed(1);
const speed = (perSecond: number) =>
  perSecond >= 1e6 ? `${mb(perSecond)} MB/s` : `${Math.round(perSecond / 1e3)} KB/s`;

/** What is downloading: megabytes received, of the size announced when it
 *  is (a compressed download can outrun its announced size), and how fast. */
function Downloading() {
  const { d, rate } = useDownloads();
  if (!d.active) return null;
  const of = d.total >= d.loaded ? ` of ${mb(d.total)}` : "";
  return (
    <div className="loading-bytes" data-testid="loading-bytes">
      Downloading {mb(d.loaded)}
      {of} MB{rate !== null && ` · ${speed(rate)}`}
    </div>
  );
}

export function LoadingScreen({
  title,
  subject,
  stages,
  current,
  stageDetail,
  failure,
  recovery,
  back = "/",
  onCancel,
}: {
  title: ReactNode;
  /** What is loading, e.g. the map's type, size and seed. */
  subject?: string;
  stages: readonly LoadingStage[];
  /** The stage in progress (an id of `stages`). */
  current: string;
  /** Short live detail for a stage that has measurable sub-progress. */
  stageDetail?: ReactNode;
  failure?: LoadingFailure | null;
  recovery?: ReactNode;
  /** Where cancelling, or leaving a failure, goes: the menu. Null when
   *  there is nowhere to go back to (the menu's own loading). */
  back?: string | null;
  onCancel?: () => void;
}) {
  const at = stages.findIndex((s) => s.id === current);
  const failed = failure && JSON.stringify([failure.message, failure.details]);
  useEffect(() => {
    if (failed) diagnosticsFailure(...(JSON.parse(failed) as [string, string[]]));
  }, [failed]);
  const [details, setDetails] = useState(false);
  return (
    <main className="menu loading" data-testid="loading" aria-busy={!failure}>
      <div className="hud-panel menu-body">
        <h1>{failure ? "Aborted" : title}</h1>
        {subject && (
          <div className="loading-subject" data-testid="loading-subject">
            {subject}
          </div>
        )}
        {failure ? (
          <div className="loading-failure" role="alert" data-testid="error">
            <p className="hud-error">{failure.message}</p>
            {failure.advice && <p className="loading-advice">{failure.advice}</p>}
            <div className="loading-actions">
              {back !== null && (
                <Link className="hud-menu-item" to={back} onClick={onCancel}>
                  Back to the menu
                </Link>
              )}
              {recovery}
            </div>
            <div className="loading-dev">
              {failure.details.length > 0 && (
                <button
                  type="button"
                  className="menu-dev"
                  aria-expanded={details}
                  onClick={() => setDetails(!details)}
                >
                  Details
                </button>
              )}
              <CopyDiagnostics className="menu-dev" />
            </div>
            {details && (
              <ul className="loading-details" data-testid="error-details">
                {[...new Set(failure.details)].map((line) => (
                  <li key={line}>{line}</li>
                ))}
              </ul>
            )}
          </div>
        ) : (
          <>
            <ol className="loading-stages" aria-label="Loading stages">
              {stages.map((stage, k) => (
                <li
                  key={stage.id}
                  aria-label={stage.label}
                  aria-current={k === at ? "step" : undefined}
                  data-state={k < at ? "done" : k === at ? "active" : "pending"}
                >
                  <span className="loading-stage-bar" aria-hidden="true" />
                  <span className="loading-stage-label">{stage.label}</span>
                </li>
              ))}
            </ol>
            <div className="loading-stage" role="status" data-testid="loading-stage">
              {stages[at]?.label}
            </div>
            {stageDetail && (
              <div className="loading-stage-detail" data-testid="loading-stage-detail">
                {stageDetail}
              </div>
            )}
            <Downloading />
            {back !== null && (
              <Link
                className="hud-menu-item loading-cancel"
                to={back}
                onClick={onCancel}
                data-testid="loading-cancel"
              >
                Cancel
              </Link>
            )}
          </>
        )}
      </div>
    </main>
  );
}
