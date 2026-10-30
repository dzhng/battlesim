// The battle's sound observes the same side publications as its visuals.
// The page carries the mix, library, looks and battle weapon rows to Rust.
import { presentationSettings } from "@web/presentationSettings";
import { audioMix } from "@web/audioMix";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";

/** The game's recordings (`fixtures/sounds.json`). */
export const gameSoundCatalog: SoundCatalog = gameSounds;

/** A battle's sound, heard as its admitted mix and looks say; its score shuffles
 *  by the battle's `seed`. */
export function createBattleAudio(
  tickHz: number,
  audio: AppAudio,
  seed: number,
  weapons: Record<string, object>,
): BattleAudio {
  return audio.createBattle(
    {
      tickHz,
      presentation: audioMix,
      weapons,
      looks: presentationSettings,
      smokeTimes: presentationSettings.effects.smoke,
      cookOff: presentationSettings.effects.cook_off,
    },
    seed,
  );
}
