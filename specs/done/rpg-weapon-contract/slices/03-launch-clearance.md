# 03 — Opt-in launcher clearance

## Contract unlocked

Artillery and long-range SAM weapons with `requires_clear_launch_path: true` refuse to launch when terrain, foliage, buildings, or other blocking bodies obstruct the muzzle-side initial projectile path. This is not target visibility and does not forbid firing into a forest.

## Seam and ownership

- `WeaponDefinition` owns `requires_clear_launch_path`, default false.
- Add one simulation helper that checks the authored physical trajectory's launcher-side corridor (the muzzle and initial flight segment) for any terrain, foliage, building, or prop blocker. An indirect high arc may clear a low obstruction; a blocker immediately over/in front of the launcher must hold fire. Do not call target `guidance_clear`; do not change full-trajectory destructive fire-through for default direct-fire guns. Keep the boundary in this one helper and pin it with the scenarios below rather than scattering distance/name exceptions.
- Apply the flag to every artillery row and to the long-range SAM row (`aa_long`) only (user, 2026-10-15). Every other weapon, short- and medium-range AA and direct fire included, leaves it false; whether a weapon intercepts missiles (`can_intercept`) has no bearing on it.
- Return existing `ActionReason::BlockedTrajectory` before launch and ammunition decrement.

## Required scenarios

- Artillery under trees, beside/in front of a building, and behind a solid prop: holds.
- A long-range SAM beside a building with an aircraft behind it, or in dense foliage with an aircraft above it: holds. A short-range AA gun in the same spot still fires.
- Clear launcher path with forest/buildings at the target: fires.
- Ordinary rifle/tank/autocannon direct fire remains willing to destroy blockers.

## Player-facing proof

The existing weapon row remains present in full and compact info cards. `BlockedTrajectory` displays the existing warning-color blocked-shot icon. Add simulation, publication, and browser/panel assertions for row presence, icon, no round launch, and no ammo spend.

## Visual gates

Use `game-ui` judgment. If a screenshot changes, run `screenshot-critique` last; run `compare-screenshots` only when a baseline/reference is supplied.
