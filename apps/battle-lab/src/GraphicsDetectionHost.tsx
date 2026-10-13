import { useEffect, type ReactNode } from "react";
import {
  registerGraphicsDetectionRunner,
  runAutoDetect,
  type GraphicsDetectionRunner,
} from "./graphicsDetection";
import { graphicsSettings } from "./graphicsSettings";

/**
 * Starts first-launch detection once the menu backdrop supplies the canonical
 * menu-reel runner. The menu owns the loading card; settings can use the same
 * runner and call `runAutoDetect` directly, with concurrent calls sharing one
 * run.
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
  useEffect(() => (runner ? registerGraphicsDetectionRunner(runner) : undefined), [runner]);
  useEffect(() => {
    if (!enabled || import.meta.env.MODE === "test") return;
    const settings = graphicsSettings.get();
    if (settings.mode !== "auto" || settings.detectedMode) return;
    const start = setTimeout(() => void runAutoDetect().catch(() => {}), 0);
    return () => clearTimeout(start);
  }, [enabled]);

  return <>{children}</>;
}
