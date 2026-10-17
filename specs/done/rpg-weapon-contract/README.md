# Explicit weapon targets, launch clearance, and the Eastern squad RPG

This contract makes weapon behavior data-owned and gives the Eastern base infantry squad a general-purpose RPG. Weapon rows state which target classes they engage on each altitude layer; explosive rows author their own armour chip threshold; and only weapons that waste ammunition when blocked opt into launcher-side clearance. The blocked-shot reason is published in the existing full and compact weapon rows.

## Why this shape

Target eligibility belongs to the weapon row, so penetration never doubles as an accidental target filter. A projectile keeps the semantic layer of the unit that fired it; live altitude does not move it between layers. Interception therefore uses the same per-layer target listing as ordinary fire, with `missile` as the explicit class. `air_defense`, `aa_tier`, and compatibility aliases were removed because they hid several unrelated mechanics behind role metadata.

Direct-fire weapons retain the game's destructive fire-through behavior. Artillery and long-range SAM rows opt into `requires_clear_launch_path`, which checks the muzzle-side initial corridor for trees, buildings, and other blocking props. This is separate from target visibility: a blocked launcher can show a warning while the destination remains a valid target.

The Eastern squad is a separate roster card so the US and Europe shared rifle squad keeps its grenade loadout. Dedicated anti-armour rows are named `rpg_at_light` and `rpg_at_heavy`; `rpg_general` is the three-round Eastern squad weapon. Its initial tuning is 280 m range, 80 penetration, 50 damage, 35 structural damage, and 4 s reload; a deterministic sample against 100 side armour gives roughly 39 damage per good hit, so three hits can destroy that test tank.

## Invariants

- `LayerFire` owns per-layer target classes (`infantry`, `vehicle`, `air_rotor`, `air_fixed_wing`, `missile`).
- `WeaponDefinition` owns `interceptable` and `requires_clear_launch_path`; no opaque air-defence role remains.
- `Round.layer` is copied at launch and never inferred from altitude.
- Explosive rows author `partial_from: 0.08` when they should chip armour.
- Only authored clearance weapons hold for launcher obstructions; ordinary direct fire still shoots through destructible cover.
- A blocked launch is represented by `BlockedTrajectory` and the existing warning row/icon contract.

## Code and tests

The main owners are `crates/contract/src/weapons.rs`, `crates/sim/src/weapons.rs`, the launch-clearance tests in `crates/sim/tests/launch_clearance.rs`, projectile/interception tests in `crates/sim/tests/`, and the fixture rows in `fixtures/game.json` and `fixtures/units/roster/`. The catalog test `eastern_rifle_squad_fields_general_purpose_rpg_and_dedicated_rows_are_renamed` pins the roster and naming contract. The blocked icon is `assets/icons/hud/blocked_shot.svg`.

The managed browser could not provide a WebGPU adapter for a full panel capture, so the SVG was inspected in a standalone render; the focused panel/readout test passed. Full Rust verification passed; the full web run remains environment-sensitive where Git LFS assets and browser audio/WASM fixtures are unavailable.

## Dead ends

A `penetration: 0` convention, inferred layer transitions, target-LOS reuse for launcher clearance, and compatibility aliases were rejected because they make unrelated mechanics implicit or preserve obsolete owners. No separate aircraft movement-mode field was added; `mobility.air.flight` remains the one flight-model owner.
