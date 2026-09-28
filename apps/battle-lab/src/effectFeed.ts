// The observation, as combat effects read it: one side's decoded
// publication, under the battle's rules, becomes an `EffectPublication`
// (visible flight, blasts, the shot counters and hulls of every unit the
// side sees, and the wrecks it knows, which smoke), and an `EffectFrame`
// for a battle's tick rate, from the fixture's `presentation.effects`.
import village from "@fixtures/village.json";
import {
  EffectFrame,
  validateEffects,
  type EffectPresentation,
  type EffectPublication,
  type EffectShooter,
  type MuzzleSource,
} from "@packages/battle-renderer/src/effects/effectFrame";
import type { DrawnMuzzles } from "@packages/battle-renderer/src/models/drawnMuzzles";
import { fromSideKey, sideKey } from "@packages/battle-renderer/src/sideKey";
import {
  mountMuzzles,
  type MountMuzzle,
  type MountRow,
} from "@packages/scene-assets/src/mountMuzzle";
import type { SideName } from "@web/battle/sim/protocol";
import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";

export const villageEffects: EffectPresentation = validateEffects(
  village.presentation.effects as unknown as EffectPresentation,
);

/** The rule blocks the effects read (the scenario's or the fixture's). */
export interface EffectRules {
  /** The body table: each vehicle's `wreck` names the prop kind it leaves. */
  bodies: Record<string, { wreck?: string }>;
  mounts: Record<string, (MountRow & { weapons: string[] })[]>;
  physics: {
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

/** The prop kinds that are wrecks, per body table, read once. */
const wreckCache = new WeakMap<EffectRules["bodies"], ReadonlySet<string>>();
function wrecksOf(bodies: EffectRules["bodies"]): ReadonlySet<string> {
  let w = wreckCache.get(bodies);
  if (!w)
    wreckCache.set(
      bodies,
      (w = new Set(Object.values(bodies).flatMap((b) => (b.wreck ? [b.wreck] : [])))),
    );
  return w;
}

/** Each mount row list's muzzle models, read once. */
const muzzleCache = new WeakMap<readonly MountRow[], (MountMuzzle | null)[]>();
function muzzlesOf(rows: readonly MountRow[]): (MountMuzzle | null)[] {
  let m = muzzleCache.get(rows);
  if (!m) muzzleCache.set(rows, (m = mountMuzzles(rows)));
  return m;
}

/** An `EffectFrame` for a battle run under `presentation.effects`. */
export function createEffectFrame(tickHz: number): EffectFrame {
  return new EffectFrame({ tickHz, presentation: villageEffects });
}

function shooter(
  key: number,
  kind: string,
  position: EffectShooter["position"],
  yaw: number,
  members: readonly number[],
  poses: readonly WeaponPoseView[],
  rules: EffectRules,
): EffectShooter {
  const mounts = rules.mounts[kind] ?? [];
  const muzzles = muzzlesOf(mounts);
  const half = hullHalf(kind, rules);
  return {
    key,
    position,
    half,
    yaw,
    members,
    mounts: poses.map((p) => ({
      bearing: p.bearing,
      elevation: p.elevation,
      shots: p.shots,
      kind: mounts[p.mount]?.weapons[0] ?? "default",
      muzzle: muzzles[p.mount] ?? null,
    })),
  };
}

/** The flashes' muzzles, as `drawn` has the models posed, for a battle seen
 *  as `side`: a shooter's key is a `sideKey` with the side's own ids even,
 *  as `effectPublication` keys them. */
export function drawnMuzzleSource(drawn: DrawnMuzzles, side: SideName): MuzzleSource {
  return {
    muzzle(shooter, mount, soldier, at) {
      const { id, side: of } = fromSideKey(shooter, side);
      return soldier === null ? drawn.vehicle(of, id, mount, at) : drawn.soldier(of, soldier, at);
    },
  };
}

/** What `o`, seen as `side`, publishes for effects. Own units and identified
 *  enemies keep apart by `sideKey` (own ids even, enemy handles odd). */
export function effectPublication(
  o: ObservationView,
  side: SideName,
  rules: EffectRules,
): EffectPublication {
  const enemy: SideName = side === "blue" ? "red" : "blue";
  const wrecks = wrecksOf(rules.bodies);
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
        shooter(
          sideKey(u.id, side, side),
          u.kind,
          u.position,
          u.yaw,
          u.memberIds,
          u.weaponPoses,
          rules,
        ),
      ),
      ...o.identified.map((e) =>
        shooter(
          sideKey(e.id, enemy, side),
          e.kind,
          e.position,
          e.yaw,
          e.memberIds,
          e.weaponPoses,
          rules,
        ),
      ),
    ],
    // Every wreck the side knows smokes, where the side last saw it, with
    // the one `wreck` look (`presentation.effects.smoke.wreck`).
    smokes: o.knownProps
      .filter((p) => wrecks.has(p.kind))
      .map((p) => ({
        key: `${p.kind}:${p.center[0]},${p.center[1]}`,
        kind: "wreck",
        center: [p.center[0], p.center[1], p.baseZ],
        yaw: p.yaw,
        half: p.half,
      })),
  };
}
