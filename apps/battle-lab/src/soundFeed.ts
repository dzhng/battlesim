// The battle's sound, fed as the visuals are: the presentation's publication
// (what the effects take, and the launches) plus the observation's hearing
// cues per publication, and the drawn vehicles and soldiers per frame
// (`presentedFrame.ts`). Sound tables and levels come from the fixture's
// `presentation.audio`; fires burn as long as the effects' smoke does.
import game from "@fixtures/game.json";
import {
  validateAudio,
  type AudioPresentation,
} from "@packages/battle-audio/src/audioPresentation";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";
import { inheritRows } from "@packages/renderer-core/src/kindTable";

/** `audio` with each weapon of `weapons` (rows as `game.json` holds them)
 *  heard as its nearest styled ancestor (`inheritRows`) where it has no row
 *  of its own: its shot, blast and, for a guided missile, its motor. */
export function inheritWeaponAudio(
  audio: AudioPresentation,
  weapons: Record<string, object>,
): AudioPresentation {
  return {
    ...audio,
    shots: inheritRows(audio.shots, weapons),
    impact_scale: inheritRows(audio.impact_scale, weapons),
    blasts: inheritRows(audio.blasts, weapons),
    motors: inheritRows(audio.motors, weapons),
  };
}

/** `catalog` with each weapon of `weapons` firing, striking and glancing as
 *  its nearest ancestor's chosen recordings (`defaults`, and each contact's
 *  row of `impacts`) where it has none of its own. */
export function inheritWeaponChoices(
  catalog: SoundCatalog,
  weapons: Record<string, object>,
): SoundCatalog {
  return {
    ...catalog,
    defaults: inheritRows(catalog.defaults, weapons),
    impacts: Object.fromEntries(
      Object.entries(catalog.impacts).map(([contact, row]) => [contact, inheritRows(row, weapons)]),
    ),
  };
}

/** The game's sound (`presentation.audio`), weapons inherited. */
export const gameAudio: AudioPresentation = validateAudio(
  inheritWeaponAudio(game.presentation.audio as unknown as AudioPresentation, game.weapons),
);

/** The game's recordings (`fixtures/sounds.json`), weapons inherited. */
export const gameSoundCatalog: SoundCatalog = inheritWeaponChoices(gameSounds, game.weapons);

/** A battle's sound, heard as `presentation.audio` says. */
export function createBattleAudio(tickHz: number, audio: AppAudio): BattleAudio {
  return audio.createBattle({
    tickHz,
    presentation: gameAudio,
    smokeTimes: game.presentation.effects.smoke,
    cookOff: game.presentation.effects.cook_off,
  });
}
