// Which battle mood the score plays, from the side's own barks, and which
// track of that mood comes next. Pure: time arrives as an argument and chance
// from the injected `random`, so a test drives it on a fake clock.
import type { Trigger } from "../voices/voiceCatalog";

export type BattleMood = "tension" | "combat";

/** `presentation.audio.music`'s direction numbers. */
export interface MusicDirection {
  /** A qualifying bark at most this old switches tension to combat. */
  combat_window_s: number;
  /** Combat returns to tension after this long without one. */
  calm_s: number;
  /** The crossfade between moods. */
  crossfade_s: number;
  /** The own barks that mean the side is in a fight (B5–B9). */
  combat_triggers: Trigger[];
}

export class MusicDirector {
  private current: BattleMood = "tension";
  private lastBark = -Infinity;
  private readonly bags: Record<BattleMood, string[]> = { tension: [], combat: [] };
  private readonly last: Record<BattleMood, string | null> = { tension: null, combat: null };

  constructor(
    private readonly rules: MusicDirection,
    private readonly tracks: Record<BattleMood, readonly string[]>,
    private readonly random: () => number,
  ) {}

  /** An own bark at time `t` (seconds). */
  note(trigger: Trigger, t: number) {
    if (this.rules.combat_triggers.includes(trigger)) this.lastBark = Math.max(this.lastBark, t);
  }

  /** The mood at time `t`. Entering and leaving use different spans, so it never flickers. */
  mood(t: number): BattleMood {
    const since = t - this.lastBark;
    if (this.current === "tension" && since <= this.rules.combat_window_s) this.current = "combat";
    else if (this.current === "combat" && since > this.rules.calm_s) this.current = "tension";
    return this.current;
  }

  /** The mood's next track: each plays once per shuffle, never the same one twice in a row. */
  next(mood: BattleMood): string | null {
    const bag = this.bags[mood];
    if (!bag.length) {
      bag.push(...this.tracks[mood]);
      for (let i = bag.length - 1; i > 0; i--) {
        const j = Math.floor(this.random() * (i + 1));
        [bag[i], bag[j]] = [bag[j], bag[i]];
      }
      // The bag is drawn from its end; keep the track just played away from it.
      if (bag.length > 1 && bag[bag.length - 1] === this.last[mood])
        [bag[0], bag[bag.length - 1]] = [bag[bag.length - 1], bag[0]];
    }
    const track = bag.pop() ?? null;
    this.last[mood] = track;
    return track;
  }
}
