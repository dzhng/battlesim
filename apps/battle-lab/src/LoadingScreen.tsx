import { Link } from "react-router";
// The cover over a battle that is still being prepared: what is being made,
// how far along it is, and, if it cannot be made, why. In the HUD's look.
import { useEffect, useState, type ReactNode } from "react";
import { downloads, type DownloadProgress } from "@web/downloads";

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

/** The page's downloads, sampled a few times a second (a byte count per
 *  chunk would redraw the screen hundreds of times a second). */
function useDownloads(): DownloadProgress {
  const [d, setD] = useState(downloads.get);
  useEffect(() => {
    const id = setInterval(() => setD(downloads.get()), 200);
    return () => clearInterval(id);
  }, []);
  return d;
}

const mb = (bytes: number) => (bytes / 1e6).toFixed(1);

/** What is downloading: megabytes received, of the size announced when it
 *  is (a compressed download can outrun its announced size). */
function Downloading() {
  const d = useDownloads();
  if (!d.active) return null;
  const of = d.total >= d.loaded ? ` of ${mb(d.total)}` : "";
  return (
    <div className="loading-bytes" data-testid="loading-bytes">
      Downloading {mb(d.loaded)}
      {of} MB
    </div>
  );
}

export function LoadingScreen({
  title,
  subject,
  stages,
  current,
  failure,
  recovery,
  back = "/",
}: {
  title: string;
  /** What is loading, e.g. the map's type, size and seed. */
  subject?: string;
  stages: readonly LoadingStage[];
  /** The stage in progress (an id of `stages`). */
  current: string;
  failure?: LoadingFailure | null;
  recovery?: ReactNode;
  /** Where cancelling, or leaving a failure, goes: the menu. Null when
   *  there is nowhere to go back to (the menu's own loading). */
  back?: string | null;
}) {
  const at = stages.findIndex((s) => s.id === current);
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
            {back !== null && (
              <Link className="hud-menu-item" to={back}>
                Back to the menu
              </Link>
            )}
            {recovery}
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
                  data-state={k < at ? "done" : k === at ? "active" : "pending"}
                />
              ))}
            </ol>
            <div className="loading-stage" role="status" data-testid="loading-stage">
              {stages[at]?.label}
            </div>
            <Downloading />
            {back !== null && (
              <Link className="hud-menu-item loading-cancel" to={back} data-testid="loading-cancel">
                Cancel
              </Link>
            )}
          </>
        )}
      </div>
    </main>
  );
}
