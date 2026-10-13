// The battle's sound, fed as the visuals are: the presentation's last
// publication and last frame, which the presentation module hears
// (`presentation::sound`). Sound tables and levels come from the fixture's
// `presentation.audio`; fires burn as long as the effects' smoke does, and a
// derived weapon sounds as its nearest styled ancestor.
import game from "@fixtures/game.json";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";

/** The game's sound (`presentation.audio`); the presentation crate checks
 *  it when a battle's sound starts. */
export const gameAudio = game.presentation.audio as unknown as AudioPresentation;

/** The game's recordings (`fixtures/sounds.json`). */
export const gameSoundCatalog: SoundCatalog = gameSounds;

/** A battle's sound, heard as `presentation.audio` says. */
export function createBattleAudio(tickHz: number, audio: AppAudio): BattleAudio {
  return audio.createBattle({
    tickHz,
    presentation: gameAudio,
    weapons: game.weapons,
    smokeTimes: game.presentation.effects.smoke,
    cookOff: game.presentation.effects.cook_off,
  });
}
