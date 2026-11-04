// The battle's sound observes the same side publications as its visuals.
// The page carries the mix, library, looks, weapon rows and catalog mounts to Rust.
import type { SessionCatalog } from "@web/battle/catalog/compose";
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
  audio: AppAudio,
  seed: number,
  catalog: SessionCatalog,
): BattleAudio {
  return audio.createBattle(
    {
      tickHz: catalog.rules.tick_hz,
      presentation: audioMix,
      weapons: catalog.weapons,
      unitCatalog: catalog.units.view,
      looks: presentationSettings,
    },
    seed,
  );
}
