# Explicit weapon targets, launch clearance, and the Eastern squad RPG

This contract makes weapon behavior data-owned and gives the Eastern base infantry squad a general-purpose RPG. Weapon rows state which target classes they engage on each altitude layer; explosive rows author their own armour chip threshold; and only weapons that waste ammunition when blocked opt into launcher-side clearance. The blocked-shot reason is published in the existing full and compact weapon rows.

## Why this shape

Target eligibility belongs to the weapon row, so penetration never doubles as an accidental target filter. A projectile keeps the semantic layer of the unit that fired it; live altitude does not move it between layers. Interception therefore uses the same per-layer target listing as ordinary fire, with `missile` as the explicit class. `air_defense`, `aa_tier`, and compatibility aliases were removed because they hid several unrelated mechanics behind role metadata.

Direct-fire weapons retain the game's destructive fire-through behavior. Artillery and long-range SAM rows opt into `requires_clear_launch_path`, which checks the muzzle-side initial corridor for trees, buildings, and other blocking props. This is separate from target visibility: a blocked launcher can show a warning while the destination remains a valid target.

The Eastern squad is a separate roster card so the US and Europe shared rifle squad keeps its grenade loadout. The specialist anti-armour teams fire `rpg_at_light` and `rpg_at_heavy`; the squad fires `rpg_general`, a high-explosive rocket that kills men by blast and chips armour. It reaches further and hits harder than a grenade, and less far and less hard than a tank's HE round. It was calibrated so three good hits kill a test tank through 100 side armour; its stats live in `fixtures/game.json`, and the starting values are in the [choices ledger](choices.md).

## Invariants

- `LayerFire` owns per-layer target classes (`infantry`, `vehicle`, `air_rotor`, `air_fixed_wing`, `missile`).
- `WeaponDefinition` owns `interceptable` and `requires_clear_launch_path`; no opaque air-defence role remains.
- `Round.layer` is copied at launch and never inferred from altitude.
- Explosive rows author `partial_from: 0.08` when they should chip armour.
- Only authored clearance weapons hold for launcher obstructions; ordinary direct fire still shoots through destructible cover.
- A blocked launch is represented by `BlockedTrajectory` and the existing warning row/icon contract.

## Code and tests

The main owners are `crates/contract/src/weapons.rs`, `crates/sim/src/weapons.rs`, the launch-clearance tests in `crates/sim/tests/launch_clearance.rs`, projectile/interception tests in `crates/sim/tests/`, and the fixture rows in `fixtures/game.json` and `fixtures/units/roster/`. The loadout is data and has no test of its own; the blast and armour-chip mechanics it relies on are tested in `crates/sim/tests/damage.rs` and `crates/sim/tests/weapons.rs`. The blocked icon is `assets/icons/hud/blocked_shot.svg`.

## Dead ends

A `penetration: 0` convention, inferred layer transitions, target-LOS reuse for launcher clearance, and compatibility aliases were rejected because they make unrelated mechanics implicit or preserve obsolete owners. No separate aircraft movement-mode field was added; `mobility.air.flight` remains the one flight-model owner.
