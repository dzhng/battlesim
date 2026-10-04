import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { AppAudio, type AudioScreen } from "@packages/battle-audio/src/appAudio";
import { gameAudio } from "./soundFeed";

const AppAudioContext = createContext<AppAudio | null>(null);

/** The mounted app retains audio across route changes. Creating the engine in
 * an effect keeps audio work after paint and gives React cleanup one owner. */
export function AppAudioProvider({ children }: { children: ReactNode }) {
  const [audio, setAudio] = useState<AppAudio | null>(null);
  useEffect(() => {
    const engine = new AppAudio({ presentation: gameAudio });
    setAudio(engine);
    return () => engine.dispose();
  }, []);
  return audio ? (
    <AppAudioContext.Provider value={audio}>{children}</AppAudioContext.Provider>
  ) : null;
}

export function useAppAudio(): AppAudio {
  const audio = useContext(AppAudioContext);
  if (!audio) throw new Error("Battle audio requires AppAudioProvider.");
  return audio;
}

/** Route policy, supplied by the router owner without an audio/router dependency. */
export function AppAudioScreen({ screen }: { screen: AudioScreen }) {
  const audio = useAppAudio();
  useEffect(() => audio.setScreen(screen), [audio, screen]);
  return null;
}
