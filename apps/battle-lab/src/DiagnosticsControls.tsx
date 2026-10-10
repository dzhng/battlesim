// The diagnostics setting and the report a player copies to report a
// problem: shown in the main menu's settings and the battle's pause menu alike.
import { useState, useSyncExternalStore } from "react";
import { diagnostics, diagnosticsReport } from "@web/diagnostics";
import { APP_COMMIT, simFingerprint } from "./buildIdentity";

export function DiagnosticsControls() {
  const enabled = useSyncExternalStore(diagnostics.subscribe, diagnostics.enabled);
  const [copied, setCopied] = useState<"copied" | "failed" | null>(null);
  const copy = async () => {
    try {
      const report = diagnosticsReport({ commit: APP_COMMIT, sim: await simFingerprint() });
      await navigator.clipboard.writeText(report);
      setCopied("copied");
    } catch {
      setCopied("failed");
    }
  };
  return (
    <div className="sound-controls diagnostics-controls" data-testid="diagnostics-controls">
      <label>
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => diagnostics.set(e.target.checked)}
        />
        Diagnostics
      </label>
      <button type="button" className="hud-menu-item" onClick={() => void copy()}>
        {copied === "copied" ? "Copied" : copied === "failed" ? "Copy failed" : "Copy diagnostics"}
      </button>
    </div>
  );
}
