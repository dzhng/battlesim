import { Link } from "react-router";
// The cover over a battle that is still being prepared: what is being made,
// how far along it is, and, if it cannot be made, why. In the HUD's look.
import { useState, type ReactNode } from "react";

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
                {failure.details.map((line) => (
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
