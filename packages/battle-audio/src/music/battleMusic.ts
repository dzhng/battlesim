// The score in a battle. Which mood plays (`tension`, `combat` from the
// side's own barks), which track comes next and when the music dips under a
// voice line is decided in Rust (`presentation::sound::music`, through the
// presentation module, which must be loaded first). This only carries its
// music commands out on a player: a start fades a track in, a stop fades
// every track out (a crossfade is the two over the same span), and a duck
// dips the bus.
import { Score } from "@wasm/renderer_browser";
import type { Duck } from "../audioMix";
import type { VoiceRequest } from "../voices/chatter";
import type { Mood } from "./musicCatalog";

/** `presentation.audio.music`: read by Rust, typed here for the workbench. */
export interface BattleMusicRules {
  /** How far the music dips under a voice line, and how fast it dips and recovers. */
  duck: Duck;
  /** A qualifying bark at most this old switches tension to combat. */
  combat_window_s: number;
  /** Combat returns to tension after this long without one. */
  calm_s: number;
  /** The crossfade between moods. */
  crossfade_s: number;
  /** The own barks that mean the side is in a fight (B5–B9). */
  combat_triggers: string[];
}

export type BattleMood = Exclude<Mood, "menu">;

/** What the music bus carries out (`presentation::sound::frame::Command`'s music ops). */
export type MusicCommand =
  | { op: "music"; at: number; track: string; fade: number }
  | { op: "music_stop"; at: number; fade: number }
  | { op: "duck"; at: number; bus: "music"; depth: number; glide: number };

/** What carries the commands out: the page's `MusicPlayer`, or a recorder. */
export interface MusicBus {
  play(track: string, options: { fade: number }): unknown;
  stop(fade: number): void;
  duck(depth: number, glide: number): void;
}

/** Carries out the score's commands on `player`. */
function playMusic(commands: readonly MusicCommand[], player: MusicBus) {
  for (const c of commands)
    if (c.op === "music") player.play(c.track, { fade: c.fade });
    else if (c.op === "music_stop") player.stop(c.fade);
    else player.duck(c.depth, c.glide);
}

export class BattleMusic {
  private readonly score: Score;

  /** By `rules` over the catalog's battle moods' tracks, shuffled by `seed`. */
  constructor(
    rules: BattleMusicRules,
    tracks: Record<BattleMood, readonly string[]>,
    private readonly player: MusicBus,
    seed: number,
  ) {
    this.score = new Score(JSON.stringify(rules), JSON.stringify(tracks), seed >>> 0);
  }

  /** At time `t` (seconds): the barks the voice lines heard since the last
   *  update, of which only the side's own move the mood, and whether a voice
   *  line sounds. Returns the commands it carried out. */
  update(t: number, barks: readonly VoiceRequest[], speaking: boolean): MusicCommand[] {
    const commands = JSON.parse(
      this.score.hear(t, barks.length ? JSON.stringify(barks) : "[]", speaking),
    ) as MusicCommand[];
    playMusic(commands, this.player);
    return commands;
  }

  dispose() {
    this.score.free();
  }
}
