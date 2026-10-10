import { useEffect } from "react";
import { useLocation } from "react-router";
import { diagnostics } from "@web/diagnostics";
import { AppCursor } from "@web/battle/present/gameCursor";
import { AppAudioProvider, AppAudioScreen } from "./AppAudio";
import { AppResourceBoundary } from "./AppResourceBoundary";
import { LabRouter, screenForPath } from "./router";

/** Page owners stay above visits so route changes release only battle work. */
export function AppShell() {
  const screen = screenForPath(useLocation().pathname);
  useEffect(() => diagnostics.start(), []);
  return (
    <AppCursor>
      <AppAudioProvider>
        <AppAudioScreen screen={screen} />
        <AppResourceBoundary menu={screen === "menu"}>
          <LabRouter />
        </AppResourceBoundary>
      </AppAudioProvider>
    </AppCursor>
  );
}
