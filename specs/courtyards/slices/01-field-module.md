# 01 The legality check gets its own module

**Unlocks:** one legality check that court and garden placement share with the street pass.

**Seam:** `crates/mapgen/src/street_props.rs` becomes `street_props/mod.rs`; `Field`, `Candidate` and `legal` move unchanged into `street_props/field.rs`. The fence loop and gate reservation inside `site()` become one routine (`fence_round(rect, kind, gate, open_sides)` or similar) that `site()` calls.

**Verify:** `cargo test -p mapgen --test street_props`; the `map-layout` parity record and a `layout_sweep` of every type and size at two seeds give byte-identical plans before and after.

**Stays green:** plan bytes, parity. **Delegated:** module and function names.
