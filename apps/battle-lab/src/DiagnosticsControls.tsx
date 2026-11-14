// The diagnostics setting and the report a player copies to report a
// problem: shown in the main menu's settings and the battle's pause menu alike,
// and the copy on every failure's plate.
import { useState, useSyncExternalStore } from "react";
import { diagnostics, diagnosticsReport } from "@web/diagnostics";
import { APP_COMMIT, simFingerprint } from "./buildIdentity";

/** Copies the diagnostics report, for the player to paste into a problem
 *  report; says whether the copy took. */
export function CopyDiagnostics({ className }: { className: string }) {
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
    <button type="button" className={className} onClick={() => void copy()}>
      {copied === "copied" ? "Copied" : copied === "failed" ? "Copy failed" : "Copy diagnostics"}
    </button>
  );
}

export function DiagnosticsControls() {
  const enabled = useSyncExternalStore(diagnostics.subscribe, diagnostics.enabled);
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
      <CopyDiagnostics className="hud-menu-item" />
    </div>
  );
}
