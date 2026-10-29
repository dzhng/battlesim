// The battle's sound, fed as the visuals are: the presentation's last
// publication and last frame, which the presentation module hears
// (`presentation::sound`). Sound tables and levels come from the fixture's
// `presentation.audio`; fires burn as long as the effects' smoke does, and a
// derived weapon sounds as its nearest styled ancestor.
import { presentationSettings } from "@web/presentationSettings";
import { audioMix } from "@web/audioMix";
import { shippedRules } from "@web/shippedRules";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";

/** The game's sound (`presentation.audio`); the presentation crate checks
 *  it when a battle's sound starts. */
export const gameAudio = audioMix;

/** The game's recordings (`fixtures/sounds.json`). */
export const gameSoundCatalog: SoundCatalog = gameSounds;

/** A battle's sound, heard as `presentation.audio` says; its score shuffles
 *  by the battle's `seed`. */
export function createBattleAudio(tickHz: number, audio: AppAudio, seed: number): BattleAudio {
  return audio.createBattle(
    {
      tickHz,
      presentation: gameAudio,
      weapons: shippedRules.weapons,
      smokeTimes: presentationSettings.effects.smoke,
      cookOff: presentationSettings.effects.cook_off,
    },
    seed,
  );
}
