# Explicit weapon targets, launch clearance, and the Eastern squad RPG

Status: planned, not implemented. Last updated 2026-10-15.

## Goal

Make weapon behavior data-owned and explicit while adding an Eastern-only multipurpose RPG for the base infantry squad. The change removes opaque air-defense role flags, keeps Hollywood direct-fire behavior, and makes launcher-side obstructions visible and actionable for the artillery and AA weapons that should hold fire rather than waste ammunition.

## Contracts

- `LayerFire` owns per-layer range, scatter, and target classes: `infantry`, `vehicle`, `air_hover`, `air_winged`. Air units need an explicit hover/winged movement mode; do not infer it from names or faction.
- `WeaponDefinition` owns `can_intercept`, `interceptable`, and opt-in `requires_clear_launch_path`.
- Projectile semantic layer is copied from the firing unit at launch and never inferred from live height.
- `radar` owns radar detection/emission. Delete unit `air_defense`, unit/weapon `aa_tier`, and their developer display.
- Existing direct-fire weapons keep firing through destructible trees/buildings for the Hollywood effect. Only authored weapons with `requires_clear_launch_path` hold when the launcher-side path is blocked.
- `BlockedTrajectory` is the player-facing reason. Existing full and compact weapon rows must show its warning mark.
- `partial_from: 0.08` is authored on every explosive weapon row/family. No engine inference from blast radius.

## Slice graph

1. **Contract hard cutover** — target classes, `can_intercept`, deletion of opaque AA fields, explosive stat authoring, and fixture validation.
2. **Immutable projectile layer and interception** — launch-layer storage, interception eligibility from weapon mounts, and regression tests.
3. **Opt-in launcher clearance** — physical muzzle/initial-trajectory obstruction for all artillery and AA/interceptor rows; preserve ordinary direct-fire behavior and expose `BlockedTrajectory`.
4. **Eastern RPG and roster data** — new three-round general-purpose RPG, dedicated AT row renames, Eastern squad loadout, and deterministic armour calibration.
5. **Presentation/catalog closeout** — regenerate derived catalogs, remove AA-tier developer surfaces, and verify full/compact warning rows/icons.

## Firewalls

- Do not make `penetration: 0` encode target eligibility.
- Do not derive projectile semantic layer from altitude or introduce layer transitions.
- Do not reuse target LOS (`guidance_clear`) for launcher clearance; target-area forest is allowed.
- Do not make all direct-fire guns hold for blockers; their existing destructive fire-through behavior is intentional.
- Do not call launcher clearance “clear sight”: it checks the muzzle-side initial flight corridor. Target-area forests/buildings remain valid fire destinations.
- Do not retain compatibility aliases for `anti_armor`, `interception`, `air_defense`, or `aa_tier`; this is a hard cutover.
- Do not add interception target classes; every `interceptable` projectile is eligible.

## Verification

Each slice runs its narrow Rust/web checks. Closeout runs the full repository checks once. Every changed simulation rule gets deterministic behavior tests and digest/replay coverage. The UI slice uses `game-ui`, keeps the existing holo-tactical panel surface, and runs `screenshot-critique` for any changed capture; use `compare-screenshots` only if a prior/reference shot is introduced.

## Refactor-clean audit

The plan gives one owner to each concept: `LayerFire` for per-layer target eligibility, `WeaponDefinition` for weapon mechanics, `Round`/`Projectile` for immutable launch layer, `RadarCapability` for radar behavior, the existing `BlockedTrajectory` publication path for player readout, and roster family labels only for grouping. It deletes the overloaded `air_defense`/`aa_tier` path rather than carrying an adapter or duplicate flag. The launch-clearance helper is deliberately separate from target LOS and from the ordinary destructive fire-through solver.

## Next Agent Prompt

Implement slice 1 in `slices/01-contract.md`. Start with red contract tests, then make the hard-cut schema and fixture changes. Do not implement simulation behavior or the RPG roster yet. Update this README's status, checklist, and next prompt before ending your pass.

## Checklist

- [ ] Slice 1: contract hard cutover (`slices/01-contract.md`)
- [ ] Slice 2: projectile layer/interception (`slices/02-projectile-layer.md`)
- [ ] Slice 3: launcher clearance and blocked readout (`slices/03-launch-clearance.md`)
- [ ] Slice 4: Eastern RPG and loadout (`slices/04-eastern-rpg.md`)
- [ ] Slice 5: catalog/UI closeout (`slices/05-closeout.md`)
