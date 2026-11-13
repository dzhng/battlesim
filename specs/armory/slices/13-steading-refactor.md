# 13 · Steading refactor: no output change

Status: done 2026-11-13.

Map row: L4, plus P3. No dependencies. It can start now: it is a code change
to map generation with byte-identical output, so L1 doesn't hold it.

## Contract

One owner for "a group of pieces in a yard at a road station, spaced, with the
halves even". Then depots (slice 17) never copy `stead`. The preset JSON doesn't
change, and every generated map is byte-identical.

## Seam

In `crates/mapgen/src/open_country/`, the signature becomes
`stead(&mut self, half, kind: &Steading, fits, rng)` (`mod.rs:459`).
`Steading` carries:
- **the station rules:** `apart_m`, lane chance and lane length;
- **its own `Yard`** (P3: depots will size theirs from their pieces);
- **`Pieces`**, one of two kinds:
  - `Homes(groups)`: today's building lots plus `furnish_yard`;
  - `Bodies(groups)`: prop bodies, with extents read from the prop catalog,
    never repeated in presets. This kind is new and unused until slice 17.

The `Homesteads` type (`rules.rs:76`) becomes `Steads`, and the JSON for
`homesteads` stays the same. `spread` (`mod.rs:432`) and `even` (`:179`) are
unchanged.

## The human runs or sees

Nothing visible.

## Verification

Byte identity, with **no** `BLESS_PARITY`:
- `crates/mapgen/tests/layout_cli.rs`
  `native_cli_replays_the_frozen_generation_records`;
- `web/tests/mapLayout.test.ts`;
- `crates/mapgen/tests/open_country.rs`, all of it;
- `country_fairness.rs`.

## Delegated to the implementer

Type and enum names.

## Must stay green

The whole mapgen crate.

## Feedback that would change it

None.
