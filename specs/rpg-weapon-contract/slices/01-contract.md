# 01 — Weapon contract hard cutover

## Contract unlocked

Weapon rows explicitly state what they can target in each altitude layer. Dedicated anti-armour behavior is represented by `classes: ["vehicle"]`, not a boolean. Interception is `can_intercept: true`; incoming projectiles retain `interceptable: true`.

## Seam and ownership

- `crates/contract/src/weapons.rs`: add target-class enum/list to `LayerFire`; replace `anti_armor` and `interception` with `can_intercept`; delete `aa_tier`.
- Extend authored air mobility with an explicit `hover`/`winged` mode so `air_hover` and `air_winged` are physical classes rather than guesses from faction or unit names.
- `crates/contract/src/catalog.rs`: delete `Capabilities.air_defense` and `AirDefenseTier` unit metadata; retain `RadarCapability`.
- Contract validation owns class/layer shape and rejects empty/unknown class lists.
- Hard-cut every fixture, test, catalog, and publication schema reference; no aliases.

## Data rules

Ground weapons author `infantry`, `vehicle`, or both. Air rows author `air_hover` and/or `air_winged` in the layer they reach. Centralize class matching in the resolved weapon target helper; ground points/area orders bypass unit class filtering. Roster `family: air_defense` labels may remain if they are presentation grouping, but they cannot imply mechanics. Add `partial_from: 0.08` to every explosive row/family explicitly; non-explosive fallback remains unchanged.

## Tests and gates

- Red/green contract tests for valid mixed targets, vehicle-only rows, invalid classes, and removed fields.
- Catalog publication/codec tests compile with the new shape.
- Verify all explosive rows have explicit `partial_from: 0.08`.
- No simulation behavior changes in this slice.
