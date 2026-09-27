// The player's sound settings, one owner for the whole app: muted or not,
// and the master volume. The main menu and the battle panel show the same
// controls over this store; it persists per browser (a convenience: it
// falls back to the defaults where storage is unavailable).

export interface SoundSettings {
  muted: boolean;
  /** Master volume, 0 to 1, over the fixture's master level. */
  volume: number;
}

const KEY = "battle.sound";
const DEFAULTS: SoundSettings = { muted: false, volume: 0.8 };

function load(): SoundSettings {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    if (raw) {
      const s = JSON.parse(raw) as Partial<SoundSettings>;
      return {
        muted: typeof s.muted === "boolean" ? s.muted : DEFAULTS.muted,
        volume: typeof s.volume === "number" ? Math.min(1, Math.max(0, s.volume)) : DEFAULTS.volume,
      };
    }
  } catch {
    // Storage blocked: the defaults.
  }
  return DEFAULTS;
}

let current: SoundSettings = load();
const listeners = new Set<() => void>();

export const soundSettings = {
  get: (): SoundSettings => current,
  set(next: Partial<SoundSettings>) {
    current = { ...current, ...next };
    try {
      globalThis.localStorage?.setItem(KEY, JSON.stringify(current));
    } catch {
      // Storage blocked: the setting lasts this page.
    }
    for (const l of listeners) l();
  },
  /** For `useSyncExternalStore`. */
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => void listeners.delete(listener);
  },
};
