import { useLocation } from "react-router";
import { AppAudioProvider, AppAudioScreen } from "./AppAudio";
import { AppResourceBoundary } from "./AppResourceBoundary";
import { LabRouter, screenForPath } from "./router";

/** Page owners stay above visits so route changes release only battle work. */
export function AppShell() {
  const screen = screenForPath(useLocation().pathname);
  return (
    <AppAudioProvider>
      <AppAudioScreen screen={screen} />
      <AppResourceBoundary menu={screen === "menu"}>
        <LabRouter />
      </AppResourceBoundary>
    </AppAudioProvider>
  );
}
