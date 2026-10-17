# 05 · Recon patrols become 3 soldiers

**Owns:** D2. **Depends on:** nothing; land it any time before 13. **Outcome change:** named. Battles that field a recon patrol change.

## Contract
Each roster recon patrol drops one scout. The 3-seat light vehicles then carry every recon team.

| Unit | Slots after |
|---|---|
| `us_army_scouts_light_patrol` (`fixtures/units/roster/us.json:404-411`) | marksman, scout, scout |
| `us_force_recon_recon_patrol` | grenadier, marksman, scout |
| `europe_recon_patrol_light_patrol` | marksman, scout, scout |
| `eastern_scouts_light_patrol` | marksman, scout, scout |

`test_recon` stays at 4 (it is a test fixture, and `menu_us_recon` extends it). Its `tests/garrison.rs:260-273` assertion still holds.

## Seam
Data only, in `fixtures/units/roster/*.json`. Fix any description that says four. Check whether the unit's price should move with its size, and record the call in [choices.md](../choices.md). The default is to keep the price.

## What a human can see
A recon card in the purchase picker and its squad on the field show three soldiers.

## Verification
- Re-bless `fixtures/catalog.json` (`BLESS_CATALOG=1`) and the parity records (`BLESS_PARITY=1`).
- Rung 1: `cargo test -p sim --test sim catalog` and the roster admission test.
- The menu reel is unchanged.

## Delegated
Whether price follows size (default: no change).

## Must stay green
The menu reel, catalog admission, and the appearance rules for infantry (`fixtures/units/roster/infantry-appearances.json`).
