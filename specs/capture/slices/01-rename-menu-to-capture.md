# 01 — Rename `menu` to `capture`

## Contract unlocked

The filmable map category is `capture`, its unit catalog set is `capture`, and its unit ids start with `capture_`. A map in any other category isn't filmable. Both menu maps (`market-town`, `paris-corner`) move together, since `paris-corner` is the menu's second scene.

## Seam

Hard rename, no alias. Known owners:
- `fixtures/maps/{market-town,paris-corner}/meta.json` (`"category"`);
- `web/src/maps/catalogue.ts`, `crates/contract/src/maps.rs` (the category enum);
- `web/src/battle/catalog/compose.ts` and `sets.ts` (the set);
- `crates/sim/src/fixtures.rs` (fixture readers).

Grep for the rest.

- `crates/presentation/src/static_world/files.rs:88-92` falls back silently on an unknown category string. Make that an error naming the string, or the rename can half-land unnoticed.
- Unit ids sit in a sorted type list, so renaming them reorders it. Re-record the parity fixtures under `fixtures/parity/` with `BLESS_PARITY=1` and the `native-parity-capture` scene.
- Re-record `crates/sim/tests/fixtures/menu-reel/*.log` with `BLESS_MENU_REEL=1`. That also fixes the known red event 1256. Before blessing, play the menu in the browser and watch both scenes for a regression: units that no longer fight, a shot that frames nothing. The agent has approval authority, but only after looking.

## Human can see

The menu plays both scenes as before. The map catalogue lists 2 capture maps and 18 test maps.

## Verification

- `web/tests/mapCatalogue.test.ts`, `web/tests/sessionCatalog.test.ts` and the Rust map/catalog tests, updated for the new names, plus a red/green test that an unknown category string is refused.
- The parity tests that `BLESS_PARITY` touched pass unblessed afterwards.
- `cargo test -p sim --test sim menu_reel` passes unblessed afterwards.
- Visual: if you take shots of the menu while checking, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on them as the last check. The rename shouldn't change a pixel.

## Delegated

Rename order, commit split, and how the error is worded.

## Stays green

Everything except the tests this slice re-records.

## Firewalls

Leave alone:
- the `menu` audio screen/mood and the `.menu-backdrop` CSS;
- `specs/done/`;
- the capture-points game mechanic and `fixtures/parity/capture/` (a separate collision; see the README).
