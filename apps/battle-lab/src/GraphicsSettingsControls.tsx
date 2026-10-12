import { useState, useSyncExternalStore } from "react";
import { graphicsSettings, resolveEffectiveTier, type GraphicsMode } from "./graphicsSettings";

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
  onRunAutoDetect?: () => void;
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
      <p className="hud-menu-note">Target: 45 FPS · Active: {labels[effective]}</p>
      <p className="hud-menu-note">
        Auto-detect:{" "}
        {last ? `${labels[last.selectedMode]} · ${Math.round(last.averageFps)} FPS` : "Not run"}
      </p>
      <button
        type="button"
        className="hud-menu-item"
        disabled={applying}
        onClick={() => onRunAutoDetect?.()}
      >
        Run auto-detect again
      </button>
      {applying && <p role="status">Applying settings…</p>}
    </section>
  );
}
