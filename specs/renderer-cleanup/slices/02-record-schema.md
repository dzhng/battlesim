# 02 · Record schema: nested records, i32, and the TypeScript export list

**Contract.** One macro can describe any GPU record, in any crate, including
nested records, signed fields and arrays of records. The TypeScript generator
publishes a chosen list, so `LAYOUTS` is free to grow (map unknown 3, item 8).

## Seam

- `records!` (`crates/presentation/src/records/layout.rs:52-95`, private today,
  `:99`) becomes usable from `renderer-core` (`#[macro_export]` with `$crate::`
  paths).
- `Field` gains an `i32` scalar (needed by `WaveRun.shift`) and a nested record
  field `x: Rec` alongside the existing `[Rec; N]`. `FieldLayout` carries the
  nested record so the ABI test can recurse.
- `pipeline_abi.rs` recurses into nested records and compares naga's constant
  array size against each field's count.
- `records/typescript.rs`: the record loop (`:93`, over `LAYOUTS`) and the
  constant loop (`:45-53`) both take explicit lists. The lists hold exactly what
  production code, tests or scenes import today (map item 8: 21 production, 11
  test or scene readers). The 82 unread exports go.
- Delete the hand-written `SCAR_KEY_MASK` export at `typescript.rs:86` if the
  constant list will carry it; never both.

**No new records yet**: slices 03 and 04 add them. No WGSL changes.

## Verify

- New failing-first test: a nested-record member reordered in WGSL fails
  `pipeline_abi` (prove it with a scratch edit, then revert).
- `cargo test -p presentation` (`records_are_current`), `-p renderer-core --test
  pipeline_abi`, typecheck, vitest.
- Assembled-shader dump unchanged, so no plates.

**Delegated:** list syntax; whether test-only readers are kept in the list or
the tests are changed to read Rust-backed values (prefer keeping them unless a
test only copies a constant).
**Stays green:** every surviving TypeScript import.
