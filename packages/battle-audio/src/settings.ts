// One persisted owner for mute and the master, music and voices levels,
// shared by both menus.
// Where browser storage is unavailable, settings last for this page only.

export interface SoundSettings {
  muted: boolean;
  /** Master volume, 0 to 1, over the fixture's master level. */
  volume: number;
  /** Music bus level, 0 to 1. */
  music: number;
  /** Voices bus level, 0 to 1. */
  voices: number;
}

const KEY = "battle.sound";
const DEFAULTS: SoundSettings = { muted: false, volume: 0.8, music: 1, voices: 1 };

const level = (value: unknown, fallback: number) =>
  typeof value === "number" && Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : fallback;

/** Unknown or out-of-range values fall back or clamp, from storage and callers alike. */
function settle(s: Partial<SoundSettings>): SoundSettings {
  return {
    muted: typeof s.muted === "boolean" ? s.muted : DEFAULTS.muted,
    volume: level(s.volume, DEFAULTS.volume),
    music: level(s.music, DEFAULTS.music),
    voices: level(s.voices, DEFAULTS.voices),
  };
}

function load(): SoundSettings {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    if (raw) return settle(JSON.parse(raw) as Partial<SoundSettings>);
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
    current = settle({ ...current, ...next });
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
