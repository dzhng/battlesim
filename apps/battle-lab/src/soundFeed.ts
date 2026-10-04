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
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";
import type { SoundMotion, SoundVehicle } from "@packages/battle-audio/src/soundFrame";
import type { PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";
import { gameEffects } from "./effectFeed";
import { sideKey } from "@packages/battle-renderer/src/sideKey";
import type { Side } from "@packages/scene-assets/src/schema";

export const gameAudio: AudioPresentation = validateAudio(
  game.presentation.audio as unknown as AudioPresentation,
);

/** A battle's sound, heard as `presentation.audio` says. */
export function createBattleAudio(tickHz: number, audio: AppAudio): BattleAudio {
  return audio.createBattle({
    tickHz,
    presentation: gameAudio,
    smokeTimes: gameEffects.smoke,
  });
}

const EMPTY: SoundMotion = { vehicles: [], soldiers: [] };
const NONE: ReadonlySet<number> = new Set();

/** What the pose driver drew moving this frame. Vehicles are keyed apart by
 *  side (`sideKey`, own even, as the effects key them). `reversing` and
 *  `enemyReversing` hold the own and seen enemy vehicles driving backwards
 *  (the observation's `reversing`). */
export function soundMotion(
  poses: PoseFrame | null,
  own: Side,
  reversing: ReadonlySet<number> = NONE,
  enemyReversing: ReadonlySet<number> = NONE,
): SoundMotion {
  if (!poses) return EMPTY;
  const vehicles: SoundVehicle[] = poses.vehicles.map((v) => ({
    key: sideKey(v.unit, v.side, own),
    kind: v.kind,
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
