import { useState, useSyncExternalStore } from "react";
import {
  GRAPHICS_TARGET_FPS,
  graphicsSettings,
  resolveEffectiveTier,
  type GraphicsMode,
} from "./graphicsSettings";

const choices: readonly GraphicsMode[] = ["auto", "low", "medium", "high"];
const labels: Record<GraphicsMode, string> = {
  auto: "Auto-detect",
  low: "Low",
  medium: "Medium",
  high: "High",
};

export function GraphicsSettingsControls({
  onApply,
  onRunAutoDetect,
}: {
  /** Rebuild hook owned by the paused viewport. The controls stay locked until it settles. */
  onApply?: () => void | Promise<void>;
  /** Measures again; only where the menu reel it measures is mounted. */
  onRunAutoDetect?: () => Promise<unknown>;
}) {
  const settings = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  const [applying, setApplying] = useState(false);
  const effective = resolveEffectiveTier(settings.mode, settings.detectedMode);
  const select = async (mode: GraphicsMode) => {
    if (applying || mode === settings.mode) return;
    setApplying(true);
    graphicsSettings.set({ mode });
    try {
      await onApply?.();
    } finally {
      setApplying(false);
    }
  };
  const last = settings.lastResult;
  return (
    <section className="graphics-settings" aria-label="Graphics" data-testid="graphics-settings">
      <div className="menu-field" id="graphics-quality-label">
        Quality
      </div>
      <div role="radiogroup" aria-labelledby="graphics-quality-label" className="menu-options">
        {choices.map((mode) => (
          <button
            key={mode}
            type="button"
            role="radio"
            aria-label={labels[mode]}
            aria-checked={settings.mode === mode}
            disabled={applying}
            className="hud-menu-choice"
            onClick={() => void select(mode)}
          >
            {labels[mode]}
          </button>
        ))}
      </div>
      <p className="hud-menu-note">
        Target: {GRAPHICS_TARGET_FPS} FPS average · Active: {labels[effective]}
      </p>
      <p className="hud-menu-note">
        Auto-detect:{" "}
        {last ? `${labels[last.selectedMode]} · ${Math.round(last.averageFps)} FPS` : "Not run"}
      </p>
      {onRunAutoDetect && (
        <button
          type="button"
          className="hud-menu-item"
          disabled={applying}
          // Detection always settles on a tier; only a cancelled run rejects.
          onClick={() => void onRunAutoDetect().catch(() => {})}
        >
          Run auto-detect again
        </button>
      )}
      {applying && <p role="status">Applying settings…</p>}
    </section>
  );
}
