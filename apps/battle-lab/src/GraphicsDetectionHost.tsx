import { useEffect, useSyncExternalStore, type ReactNode } from "react";
import {
  graphicsDetectionState,
  registerGraphicsDetectionRunner,
  runAutoDetect,
  subscribeGraphicsDetection,
  type GraphicsDetectionRunner,
} from "./graphicsDetection";
import { graphicsSettings } from "./graphicsSettings";

/**
 * Owns first-launch gating around the menu while the backdrop supplies the
 * canonical menu-reel runner. Settings can use the same runner and call
 * `runAutoDetect` directly; concurrent calls share this run.
 */
export function GraphicsDetectionHost({
  runner,
  children,
  enabled = true,
}: {
  runner: GraphicsDetectionRunner | null;
  children: ReactNode;
  enabled?: boolean;
}) {
  const state = useSyncExternalStore(subscribeGraphicsDetection, graphicsDetectionState);
  const settings = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  useEffect(() => (runner ? registerGraphicsDetectionRunner(runner) : undefined), [runner]);
  useEffect(() => {
    if (!enabled || import.meta.env.MODE === "test") return;
    const settings = graphicsSettings.get();
    if (settings.mode !== "auto" || settings.detectedMode) return;
    const start = setTimeout(() => void runAutoDetect().catch(() => {}), 0);
    return () => clearTimeout(start);
  }, [enabled]);

  if (state.status === "failed")
    return (
      <main className="lab-loading" role="alert" data-testid="graphics-detection-error">
        <h1>Graphics detection failed</h1>
        <p>{state.error}</p>
        <button type="button" className="hud-menu-item" onClick={() => window.location.reload()}>
          Reload
        </button>
      </main>
    );
  const waitingForFirstDetection =
    enabled &&
    import.meta.env.MODE !== "test" &&
    state.status === "idle" &&
    settings.mode === "auto" &&
    !settings.detectedMode;
  if (state.status === "running" || waitingForFirstDetection)
    return (
      <>
        {children}
        <main
          className="graphics-detection-cover"
          role="status"
          aria-live="polite"
          data-testid="graphics-detection-progress"
        >
          <h1 className="graphics-detection-title">Detecting Graphics</h1>
        </main>
      </>
    );
  return <>{children}</>;
}
