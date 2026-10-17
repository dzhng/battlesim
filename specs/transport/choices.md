# Transport: choices ledger

The calls the implementing agent made where the plan was silent, one entry per call. Each entry says:
- the slice it came from;
- the choice, and the alternative it beat;
- why;
- what it reaches;
- how to reverse it.

Decisions the user made live in [map.md](map.md); change them there.


## Slice 05
- **Recon prices stay the same after dropping a scout.**
  - Alternative: cut the price in proportion to the squad's size.
  - Why: the slice's default; nothing asked for retuning prices.
  - Reach: recon cards cost the same for three soldiers.
  - Reverse: edit the cost on the four rows in `fixtures/units/roster/*.json`.

## Slice 02
- **`Unit::garrisoned()` stays the name of the building-only query**, rather than being renamed.
  - Why: it still means "inside a building".
  - Reverse: rename it if carriers make the name ambiguous.
- **Building seat data lives in `Place::Building(Garrison)`, while `Hold` owns the host and phase.**
  - Alternative: flatten the seat fields into `Hold`.
  - Why: keeps building specifics in `garrison`.
- **`Hold` has a temporary `Deref`/`DerefMut` to `Garrison`** so building code reads unchanged. It panics on a non-building host.
  - This is short-lived scaffolding: slice 06 removes it when `Host::Carrier` arrives, and building code then reaches its `Place` explicitly.
- **The digest proof was replay-based.** The garrison and buildings modules ran before and after (54 and 32 tests), the 1800 per-tick replay digests matched, and `cargo test -p sim` passed. A probe listing every battle's final digest wasn't practical through the private test helpers.
