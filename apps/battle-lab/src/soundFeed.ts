// The battle's sound, fed as the visuals are: the same `EffectPublication`
// the effects take plus the observation's hearing cues per publication, and
// the pose driver's drawn vehicles and soldiers per frame. Sound tables and
// levels come from the fixture's `presentation.audio`; fires burn as long
// as the effects' smoke does.
import village from "@fixtures/village.json";
import {
  validateAudio,
  type AudioPresentation,
} from "@packages/battle-audio/src/audioPresentation";
import { BattleAudio } from "@packages/battle-audio/src/battleAudio";
import type { SoundMotion, SoundVehicle } from "@packages/battle-audio/src/soundFrame";
import type { PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";
import { villageEffects, type EffectRules } from "./effectFeed";

export const villageAudio: AudioPresentation = validateAudio(
  village.presentation.audio as unknown as AudioPresentation,
);

/** A battle's sound for `rules`, heard as `presentation.audio` says. */
export function createBattleAudio(rules: EffectRules, tickHz: number): BattleAudio {
  return new BattleAudio({
    tickHz,
    presentation: villageAudio,
    vehicleMuzzle: rules.physics.tank_muzzle_local_m,
    smokeTimes: villageEffects.smoke,
  });
}

const EMPTY: SoundMotion = { vehicles: [], soldiers: [] };

/** What the pose driver drew moving this frame. Vehicles are keyed apart by
 *  side (own even, enemy odd, as the effects key them). */
export function soundMotion(poses: PoseFrame | null, own: string): SoundMotion {
  if (!poses) return EMPTY;
  const vehicles: SoundVehicle[] = poses.vehicles.map((v) => ({
    key: v.unit * 2 + (v.side === own ? 0 : 1),
    kind: v.kind,
    position: v.position,
    travelL: v.articulation.travel_l,
    travelR: v.articulation.travel_r,
    turret: v.articulation.turret_yaw,
    // Slice 39's drive sets this when a vehicle backs up; off until then.
    reverse: false,
  }));
  return {
    vehicles,
    soldiers: poses.soldiers.map((s) => ({ id: s.soldier, position: s.position })),
  };
}
