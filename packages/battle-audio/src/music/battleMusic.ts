// The score in a battle: it opens on `tension`, the side's own barks move it
// to `combat` and back (`MusicDirector`), each change a crossfade, and it
// dips while a voice line sounds. Pure on an injected player and clock, so a
// test drives it without a media element.
import type { VoiceRequest } from "../voices/chatter";
import { MusicDirector, type BattleMood, type MusicDirection } from "./musicDirector";
import type { MusicPlayer } from "./musicPlayer";

/** `presentation.audio.music`. */
export interface BattleMusicRules extends MusicDirection {
  /** How far the music dips under a voice line, and how fast it dips and recovers. */
  duck: { depth: number; attack_s: number; release_s: number };
}

export class BattleMusic {
  private readonly director: MusicDirector;
  private mood: BattleMood | null = null;
  private ducked = false;

  constructor(
    private readonly rules: BattleMusicRules,
    tracks: Record<BattleMood, readonly string[]>,
    private readonly player: Pick<MusicPlayer, "play" | "crossfadeTo" | "duck">,
    random: () => number,
  ) {
    this.director = new MusicDirector(rules, tracks, random);
  }

  /** At time `t` (seconds): the barks heard since the last update, of which
   *  only the side's own move the mood, and whether a voice line sounds. */
  update(t: number, barks: readonly VoiceRequest[], speaking: boolean) {
    for (const bark of barks) if (bark.tier === "own_bark") this.director.note(bark.trigger, t);
    const mood = this.director.mood(t);
    if (mood !== this.mood) {
      const track = this.director.next(mood);
      // The first track fades in under the menu's fade-out; a change crossfades.
      if (track && this.mood === null) this.player.play(track, { fade: this.rules.crossfade_s });
      else if (track) this.player.crossfadeTo(track, this.rules.crossfade_s);
      this.mood = mood;
    }
    if (speaking !== this.ducked) {
      const { depth, attack_s, release_s } = this.rules.duck;
      this.player.duck(speaking ? depth : 0, attack_s, release_s);
      this.ducked = speaking;
    }
  }
}
