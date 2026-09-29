// Mute, master volume and subtitles: the one control over `soundSettings`,
// shown in the main menu and the battle's pause menu alike.
import { useSyncExternalStore } from "react";
import { soundSettings } from "@packages/battle-audio/src/settings";

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
      <label>
        Volume{" "}
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={s.volume}
          disabled={s.muted}
          aria-label="Master volume"
          onChange={(e) => soundSettings.set({ volume: Number(e.target.value) })}
        />
      </label>
      <label>
        <input
          type="checkbox"
          checked={s.subtitles}
          onChange={(e) => soundSettings.set({ subtitles: e.target.checked })}
        />
        Subtitles
      </label>
    </div>
  );
}
