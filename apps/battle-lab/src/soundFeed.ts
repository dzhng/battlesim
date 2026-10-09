// The battle's sound, fed as the visuals are: the same `EffectPublication`
// the effects take plus the observation's hearing cues per publication, and
// the pose driver's drawn vehicles and soldiers per frame. Sound tables and
// levels come from the fixture's `presentation.audio`; fires burn as long
// as the effects' smoke does.
import game from "@fixtures/game.json";
import {
  validateAudio,
  type AudioPresentation,
} from "@packages/battle-audio/src/audioPresentation";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";
import type { SoundMotion, SoundVehicle } from "@packages/battle-audio/src/soundFrame";
import type { PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";
import { gameEffects } from "./effectFeed";
import { sideKey } from "@packages/battle-renderer/src/sideKey";
import { inheritRows } from "@packages/renderer-core/src/kindTable";
import type { Side } from "@packages/scene-assets/src/schema";
import { vehicleClass, type UnitCatalog } from "@packages/scene-assets/src/units";

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
    smokeTimes: gameEffects.smoke,
    cookOff: gameEffects.cook_off,
  });
}

const EMPTY: SoundMotion = { vehicles: [], soldiers: [] };
const NONE: ReadonlySet<number> = new Set();

/** What the pose driver drew moving this frame. Vehicles are keyed apart by
 *  side (`sideKey`, own even, as the effects key them) and heard as their
 *  class (`vehicleClass`) in `units`. `reversing` and
 *  `enemyReversing` hold the own and seen enemy vehicles driving backwards
 *  (the observation's `reversing`). */
export function soundMotion(
  poses: PoseFrame | null,
  units: UnitCatalog,
  own: Side,
  reversing: ReadonlySet<number> = NONE,
  enemyReversing: ReadonlySet<number> = NONE,
): SoundMotion {
  if (!poses) return EMPTY;
  const vehicles: SoundVehicle[] = poses.vehicles.map((v) => ({
    key: sideKey(v.unit, v.side, own),
    vehicleClass: vehicleClass(units.type(v.kind)) ?? "default",
    position: v.position,
    travelL: v.articulation.travel_l,
    travelR: v.articulation.travel_r,
    turret: v.articulation.turret_yaw,
    reverse: v.side === own ? reversing.has(v.unit) : enemyReversing.has(v.unit),
  }));
  return {
    vehicles,
    soldiers: poses.soldiers.map((s) => ({ id: s.soldier, position: s.position })),
  };
}
