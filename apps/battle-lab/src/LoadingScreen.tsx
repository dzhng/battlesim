// The cover over a battle that is still being prepared: what is being made,
// how far along it is, and, if it cannot be made, why. In the HUD's look.
export interface LoadingStage {
  id: string;
  label: string;
}

export function LoadingScreen({
  title,
  subject,
  stages,
  current,
  failure,
}: {
  title: string;
  /** What is loading, e.g. the map's type, size and seed. */
  subject: string;
  stages: readonly LoadingStage[];
  /** The stage in progress (an id of `stages`). */
  current: string;
  /** Loading stopped: the reason, and the details a developer needs. */
  failure?: { message: string; details: string[] } | null;
}) {
  const at = stages.findIndex((s) => s.id === current);
  return (
    <main className="menu loading" data-testid="loading" aria-busy={!failure}>
      <div className="hud-panel menu-body">
        <h1>{title}</h1>
        <div className="loading-subject" data-testid="loading-subject">
          {subject}
        </div>
        {failure ? (
          <div className="hud-error" role="alert" data-testid="error">
            {failure.message}
            {failure.details.length > 0 && (
              <ul className="loading-details">
                {failure.details.map((line) => (
                  <li key={line}>{line}</li>
                ))}
              </ul>
            )}
            <a href="/">Main menu</a>
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
          </>
        )}
      </div>
    </main>
  );
}
