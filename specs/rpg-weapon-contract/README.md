# Explicit weapon targets, launch clearance, and the Eastern squad RPG

Status: slices 1–3 landed (2026-10-15, branch `awf/weapon-contract`, as part of the air-war-flight closeout, air-war-flight D53); interception became the `missile` target class per layer and guided rows may intercept (air-war-flight D54, branch `awf/intercept-layers`); slices 4–5 open. Reconciled with air-war-flight: the AA-flag deletion has landed; air classes use the existing flight model; timings follow the game-wide caps.

## Goal

Make weapon behavior data-owned and explicit while adding an Eastern-only multipurpose RPG for the base infantry squad. The change removes opaque air-defense role flags, keeps Hollywood direct-fire behavior, and makes launcher-side obstructions visible and actionable for the artillery and long-range SAM weapons that should hold fire rather than waste ammunition.

## Contracts

- `LayerFire` owns per-layer range, scatter, and target classes: `infantry`, `vehicle`, `air_rotor`, `air_fixed_wing`, and `missile`, an interceptable round fired from that layer (air-war-flight D54). The air classes read the aircraft's existing flight model, `mobility.air.flight` (`rotor` | `fixed_wing`, air-war-flight), never a second movement-mode field, a name or a faction.
- `WeaponDefinition` owns `interceptable` and opt-in `requires_clear_launch_path`; whether it intercepts is its `targets` listing `missile` (D54, `can_intercept` is gone). The corridor it checks is the rules' `launch.clearance_m` (30 m) of the round's arc from the muzzle.
- `WeaponDefinition::engages(layer, class)` is the one class match; a ground point or an area passes `None` and no class bars it; a round passes `missile` at its launch layer. A unit's class is `UnitType::target_class` (body and `mobility.air.flight`). Area fire stays general-purpose ammunition only: a row whose ground layer lists `infantry` and is not armour-piercing (W10), which is what `anti_armor: true` used to forbid.
- Weapon rows refuse any field they do not have, by name: `anti_armor`, `interception`, `can_intercept`, `aa_tier` and the old `fov_deg` alias fail at load.
- Projectile semantic layer is copied from the firing unit at launch (`Round.layer`) and never inferred from live height. It is not folded into the digest: it is a function of the round's digested shooter.
- `radar` owns radar detection/emission. Unit `air_defense`, unit/weapon `aa_tier` and their developer display are already deleted (air-war-flight D52, 2026-10-15): interceptors arm from their weapon, SEAD finds any ground unit whose radar emits, and any unit firing at an air target needs its own view of it.
- Existing direct-fire weapons keep firing through destructible trees/buildings for the Hollywood effect. Only authored weapons with `requires_clear_launch_path` hold when the launcher-side path is blocked.
- `BlockedTrajectory` is the player-facing reason. Existing full and compact weapon rows must show its warning mark.
- `partial_from: 0.08` is authored on every explosive weapon row/family. No engine inference from blast radius.

## Slice graph

1. **Contract hard cutover** — target classes (replacing `anti_armor`), explosive stat authoring, and fixture validation. The AA-field deletion and the `interception` → `can_intercept` rename already landed (D52).
2. **Immutable projectile layer** — launch-layer storage and interceptor range selection from it. Interception eligibility from weapon mounts already landed (D52).
3. **Opt-in launcher clearance** — physical muzzle/initial-trajectory obstruction for artillery and the long-range SAM only; preserve ordinary direct-fire behavior and expose `BlockedTrajectory`.
4. **Eastern RPG and roster data** — new three-round general-purpose RPG, dedicated AT row renames, Eastern squad loadout, and deterministic armour calibration.
5. **Presentation/catalog closeout** — regenerate derived catalogs, remove AA-tier developer surfaces, and verify full/compact warning rows/icons.

## Firewalls

- Do not make `penetration: 0` encode target eligibility.
- Do not derive projectile semantic layer from altitude or introduce layer transitions.
- Do not reuse target LOS (`guidance_clear`) for launcher clearance; target-area forest is allowed.
- Do not make all direct-fire guns hold for blockers; their existing destructive fire-through behavior is intentional.
- Do not call launcher clearance “clear sight”: it checks the muzzle-side initial flight corridor. Target-area forests/buildings remain valid fire destinations.
- Do not retain compatibility aliases for `anti_armor` or `interception`; this is a hard cutover (`air_defense` and `aa_tier` are already gone).
- Do not add a hover/winged mobility field: `mobility.air.flight` is the one owner of how an aircraft flies.
- Weapon timings follow the game-wide caps: reload about 5 s at most (artillery and the long-range SAM up to 15 s), aim 3 s at most (air-war-flight D50).
- Tests use `test_*` units and numbers written in the test (write-tests: a test sets the bounds it depends on). Roster content (the Eastern squad's loadout, the RPG's numbers) is validated where it loads or judged in sample runs, never pinned by a test.
- Interception is per layer (superseded firewall, user 2026-10-15, air-war-flight D54): a layer listing `missile` shoots down interceptable rounds fired from that layer, and no other path matches a round. No row lists it on the ground today, so ATGMs and RPGs are never intercepted.

## Verification

Each slice runs its narrow Rust/web checks. Closeout runs the full repository checks once. Every changed simulation rule gets deterministic behavior tests and digest/replay coverage. The UI slice uses `game-ui`, keeps the existing holo-tactical panel surface, and runs `screenshot-critique` for any changed capture; use `compare-screenshots` only if a prior/reference shot is introduced.

## Refactor-clean audit

The plan gives one owner to each concept: `LayerFire` for per-layer target eligibility, `WeaponDefinition` for weapon mechanics, `Round`/`Projectile` for immutable launch layer, `RadarCapability` for radar behavior, the existing `BlockedTrajectory` publication path for player readout, and roster family labels only for grouping. It deletes the overloaded `air_defense`/`aa_tier` path rather than carrying an adapter or duplicate flag. The launch-clearance helper is deliberately separate from target LOS and from the ordinary destructive fire-through solver.

## What slices 1–3 left open

- ~~Guided interceptors never launch at a round.~~ Resolved by D54: a guided row listing `missile` launches at a round and homes on it; `aa_long` lists no `missile`, so it no longer wastes its lock on one.
- The skirmish AI moves a launcher its cover keeps holding (D54, `launch.relocate_after_s` / `relocate_m`); a player's own units are theirs to move.
- **`partial_from: 0.08` on every explosive row is a balance change** (user, through the coordinator, 2026-10-15): grenades, RPGs, ATGMs, SAMs, autocannon rounds, bombs and artillery now chip armour they cannot pierce. Retune in slice 4's sample run if it reads wrong.
- The kamikaze drones' unit ids still end `_anti_armor`; they name models, not the removed field.

## Next Agent Prompt

Implement slice 4 in `slices/04-eastern-rpg.md` on top of the landed contract (target classes, explicit `partial_from` on every explosive row, `requires_clear_launch_path`). The grenade's explicit `partial_from: 0.08` has already landed. Start with red tests on `test_*` units, then the roster data; judge the three-hit calibration in a sample run. Then slice 5. Update this README's status, checklist and next prompt before ending your pass.

## Checklist

- [x] Slice 1: contract hard cutover (`slices/01-contract.md`)
- [x] Slice 2: projectile layer/interception (`slices/02-projectile-layer.md`)
- [x] Slice 3: launcher clearance and blocked readout (`slices/03-launch-clearance.md`)
- [ ] Slice 4: Eastern RPG and loadout (`slices/04-eastern-rpg.md`)
- [ ] Slice 5: catalog/UI closeout (`slices/05-closeout.md`)
