// The observation, as combat effects read it: one side's decoded
// publication, under the battle's rules, becomes an `EffectPublication`
// (visible flight, blasts, the shot counters and hulls of every unit the
// side sees, and the wrecks and destroyed buildings it knows, which smoke),
// and an `EffectFrame` for a battle's tick rate, from the fixture's
// `presentation.effects`. The unit catalog gives each type's hull, mounts
// and wreck, and what a destroyed building's parts become.
import game from "@fixtures/game.json";
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
import { inheritRows } from "@packages/renderer-core/src/kindTable";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import { mountMuzzles, type MountMuzzle } from "@packages/scene-assets/src/mountMuzzle";
import { airborne, type UnitCatalog, type UnitType } from "@packages/scene-assets/src/units";
import type { SideName } from "@web/battle/sim/protocol";
import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";

const effects = game.presentation.effects as unknown as EffectPresentation;
/** The game's effects, each weapon's round looking as its nearest styled
 *  ancestor's (`inheritRows`) where it has no row of its own. */
export const gameEffects: EffectPresentation = validateEffects({
  ...effects,
  tracers: inheritRows(effects.tracers, game.weapons),
  flashes: inheritRows(effects.flashes, game.weapons),
  impact_scale: inheritRows(effects.impact_scale, game.weapons),
  blast_scale: inheritRows(effects.blast_scale, game.weapons),
});

/** The prop kinds that smoke once a side knows of them, each with its look (a
 *  `presentation.effects.smoke` row), per catalog, read once: what a
 *  destroyed building's parts become, its remains (`ruin`) or its gutted
 *  shell (`gutted`), and every hull's wreck (`wreck`). */
const smokeLookCache = new WeakMap<UnitCatalog, ReadonlyMap<string, string>>();
function smokeLooksOf(units: UnitCatalog): ReadonlyMap<string, string> {
  let looks = smokeLookCache.get(units);
  if (!looks) {
    const byKind = new Map<string, string>();
    const { remains, shells } = buildingRemains(units);
    for (const kind of remains) byKind.set(kind, "ruin");
    for (const kind of shells) byKind.set(kind, "gutted");
    for (const id of units.ids) {
      const wreck = units.hull(id)?.wreck;
      if (wreck) byKind.set(wreck, "wreck");
    }
    smokeLookCache.set(units, (looks = byKind));
  }
  return looks;
}

/** Each type's mount muzzle models, read once. */
const muzzleCache = new WeakMap<UnitType, (MountMuzzle | null)[]>();
function muzzlesOf(t: UnitType): (MountMuzzle | null)[] {
  let m = muzzleCache.get(t);
  if (!m) muzzleCache.set(t, (m = mountMuzzles(t.mounts)));
  return m;
}

/** An `EffectFrame` for a battle run under `presentation.effects`. */
export function createEffectFrame(tickHz: number): EffectFrame {
  return new EffectFrame({ tickHz, presentation: gameEffects });
}

function shooter(
  key: number,
  kind: string,
  position: EffectShooter["position"],
  yaw: number,
  members: readonly number[],
  poses: readonly WeaponPoseView[],
  units: UnitCatalog,
): EffectShooter {
  const mounts = units.type(kind).mounts;
  const muzzles = muzzlesOf(units.type(kind));
  const half = units.hull(kind)?.half_extents_m ?? null;
  return {
    key,
    position,
    half,
    airborne: airborne(units.type(kind)),
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
      return soldier === null
        ? drawn.vehicle(of, id, mount, at)
        : drawn.soldier(of, soldier, mount, at);
    },
  };
}

/** What `o`, seen as `side`, publishes for effects. Own units and identified
 *  enemies keep apart by `sideKey` (own ids even, enemy handles odd). */
export function effectPublication(
  o: ObservationView,
  side: SideName,
  units: UnitCatalog,
): EffectPublication {
  const enemy: SideName = side === "blue" ? "red" : "blue";
  const smokeLooks = smokeLooksOf(units);
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
          units,
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
          units,
        ),
      ),
    ],
    // Every wreck the side knows smokes where the side last saw it, and so
    // does every part of a building it knows destroyed, each with its kind's
    // look (`presentation.effects.smoke`).
    smokes: o.knownProps.flatMap((p) => {
      const kind = p.destroyed ? undefined : smokeLooks.get(p.kind);
      return kind === undefined
        ? []
        : [
            {
              key: `${p.kind}:${p.center[0]},${p.center[1]}`,
              kind,
              center: [p.center[0], p.center[1], p.baseZ] as [number, number, number],
              yaw: p.yaw,
              half: p.half,
            },
          ];
    }),
  };
}
