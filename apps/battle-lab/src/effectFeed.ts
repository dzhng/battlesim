// The observation, as combat effects read it: one side's decoded
// publication becomes an `EffectPublication` (visible flight, blasts, the
// shot counters and hulls of every unit the side sees, and the wrecks it
// knows, which smoke), and an `EffectFrame` for a battle's rules, from the
// fixture's `presentation.effects`.
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

/** The prop kinds that are wrecks: each vehicle's wreck row, from the body table. */
const WRECKS: ReadonlySet<string> = new Set(
  Object.values(village.bodies as Record<string, { wreck?: string }>).flatMap((b) =>
    b.wreck ? [b.wreck] : [],
  ),
);

/** The rule blocks the effects read (the scenario's or the fixture's). */
export interface EffectRules {
  mounts: Record<string, { weapons: string[] }[]>;
  physics: {
    tank_muzzle_local_m: number[];
    jeep_muzzle_local_m: number[];
    tank_half_extents_m: number[];
    supply_half_extents_m: number[];
    jeep_half_extents_m: number[];
  };
}

/** A hull's half extents by unit kind; infantry has none. */
function hullHalf(kind: string, rules: EffectRules): EffectShooter["half"] {
  if (kind === "tank") return rules.physics.tank_half_extents_m;
  if (kind === "supply") return rules.physics.supply_half_extents_m;
  if (kind === "jeep") return rules.physics.jeep_half_extents_m;
  return null;
}

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
  const half = hullHalf(kind, rules);
  return {
    key,
    position,
    half,
    ...(kind === "jeep" && { muzzle: rules.physics.jeep_muzzle_local_m }),
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
      ...o.own.map((u) => shooter(u.id * 2, u.kind, u.position, u.memberIds, u.weaponPoses, rules)),
      ...o.identified.map((e) =>
        shooter(e.id * 2 + 1, e.kind, e.position, e.memberIds, e.weaponPoses, rules),
      ),
    ],
    // Every wreck the side knows smokes, where the side last saw it, with
    // the one `wreck` look (`presentation.effects.smoke.wreck`).
    smokes: o.knownProps
      .filter((p) => WRECKS.has(p.kind))
      .map((p) => ({
        key: `${p.kind}:${p.center[0]},${p.center[1]}`,
        kind: "wreck",
        center: [p.center[0], p.center[1], p.baseZ],
        yaw: p.yaw,
        half: p.half,
      })),
  };
}
