// The observation, as combat effects read it: one side's decoded
// publication becomes an `EffectPublication` (visible flight, blasts, and the
// shot counters of every unit the side sees shoot), and an `EffectFrame` for
// a battle's rules, from the fixture's `presentation.effects`.
import village from "@fixtures/village.json";
import {
  EffectFrame,
  validateEffects,
  type EffectPresentation,
  type EffectPublication,
  type EffectShooter,
} from "@packages/battle-renderer/src/effects/effectFrame";
import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";

export const villageEffects: EffectPresentation = validateEffects(
  village.presentation.effects as unknown as EffectPresentation,
);

/** The rule blocks the effects read (the scenario's or the fixture's). */
export interface EffectRules {
  mounts: Record<string, { weapons: string[] }[]>;
  physics: { tank_muzzle_local_m: number[] };
}

const VEHICLES = new Set(["tank", "supply"]);

/** An `EffectFrame` for a battle run under `rules`. */
export function createEffectFrame(rules: EffectRules, tickHz: number): EffectFrame {
  return new EffectFrame({
    tickHz,
    presentation: villageEffects,
    vehicleMuzzle: rules.physics.tank_muzzle_local_m,
  });
}

function shooter(
  key: number,
  kind: string,
  position: EffectShooter["position"],
  members: readonly number[],
  poses: readonly WeaponPoseView[],
  rules: EffectRules,
): EffectShooter {
  const mounts = rules.mounts[kind] ?? [];
  return {
    key,
    vehicle: VEHICLES.has(kind),
    position,
    members,
    mounts: poses.map((p) => ({
      bearing: p.bearing,
      elevation: p.elevation,
      shots: p.shots,
      kind: mounts[p.mount]?.weapons[0] ?? "default",
    })),
  };
}

/** What `o` publishes for effects. Own units and identified enemies keep
 *  apart by key (own ids even, enemy handles odd). */
export function effectPublication(o: ObservationView, rules: EffectRules): EffectPublication {
  return {
    tick: o.tick,
    segments: o.projectiles.map((p) => ({
      path: p.path,
      ricochets: p.ricochets,
      kind: p.kind,
      shooter: p.shooterMember,
      hit: p.hit,
      normal: p.impactNormal,
    })),
    blasts: o.blasts,
    shooters: [
      ...o.own.map((u) =>
        shooter(u.id * 2, u.kind, u.position, u.memberIds, u.weaponPoses, rules),
      ),
      ...o.identified.map((e) =>
        shooter(e.id * 2 + 1, e.kind, e.position, e.memberIds, e.weaponPoses, rules),
      ),
    ],
  };
}
