/** Monotonic milliseconds and cancellable scheduling, supplied by the caller. */
export interface PresenceClock {
  now(): number;
  schedule(run: () => void, delayMs: number): () => void;
}

/** Ride through a stream cut before giving a lobby seat away. */
export const PRESENCE_GRACE_MS = 5_000;

type Entry = { streams: number; absentAt: number | null; cancel: () => void };

/** One count of open streams for each key, with one grace after the last
 *  closes. The lobby keeps one keyed by browser, for all its streams (its room
 *  list and battle among them); a room keeps its own, keyed by seat. */
export function createPresence(
  clock: PresenceClock,
  changed: (who: string) => void,
  gone: (who: string) => void,
) {
  const entries = new Map<string, Entry>();
  const becomeAbsent = (who: string, entry: Entry) => {
    entry.absentAt = clock.now();
    entry.cancel = clock.schedule(() => gone(who), PRESENCE_GRACE_MS);
    changed(who);
  };
  return {
    /** A successful hello or creation must age out even if no stream ever opens. */
    track(who: string) {
      if (entries.has(who)) return;
      const entry: Entry = { streams: 0, absentAt: null, cancel: () => {} };
      entries.set(who, entry);
      becomeAbsent(who, entry);
    },
    /** Admission starts the same grace while a deployed page opens its stream. */
    expectStream(who: string) {
      const entry = entries.get(who);
      if (!entry || entry.streams > 0) return;
      entry.cancel();
      becomeAbsent(who, entry);
    },
    open(who: string): () => void {
      let entry = entries.get(who);
      if (!entry) {
        entry = { streams: 0, absentAt: null, cancel: () => {} };
        entries.set(who, entry);
      }
      entry.cancel();
      entry.streams++;
      entry.absentAt = null;
      changed(who);
      let closed = false;
      return () => {
        if (closed) return;
        closed = true;
        entry.streams--;
        // A forgotten entry's streams close into nothing: whoever holds the
        // key now has an entry, and a grace, of their own.
        if (entry.streams === 0 && entries.get(who) === entry) becomeAbsent(who, entry);
      };
    },
    /** Null means connected; otherwise the duration since the last stream closed. */
    goneFor(who: string): number | null {
      const entry = entries.get(who);
      return entry?.absentAt == null ? null : clock.now() - entry.absentAt;
    },
    forget(who: string) {
      entries.get(who)?.cancel();
      entries.delete(who);
    },
    dispose() {
      for (const entry of entries.values()) entry.cancel();
      entries.clear();
    },
  };
}
