// Mute and the master, music and voices levels: the one control over
// `soundSettings`, shown in the main menu and the battle's pause menu alike.
import { useSyncExternalStore } from "react";
import { soundSettings } from "@packages/battle-audio/src/settings";

const LEVELS = [
  { key: "volume", label: "Master" },
  { key: "music", label: "Music" },
  { key: "voices", label: "Voices" },
] as const;

export function SoundControls() {
  const s = useSyncExternalStore(soundSettings.subscribe, soundSettings.get);
  return (
    <div className="sound-controls" data-testid="sound-controls">
      <label>
        <input
          type="checkbox"
          checked={!s.muted}
          onChange={(e) => soundSettings.set({ muted: !e.target.checked })}
        />
        Sound
      </label>
      <div className="sound-levels">
        {LEVELS.map(({ key, label }) => (
          <label key={key}>
            {label}
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={s[key]}
              disabled={s.muted}
              aria-label={`${label} volume`}
              onChange={(e) => soundSettings.set({ [key]: Number(e.target.value) })}
            />
          </label>
        ))}
      </div>
    </div>
  );
}
