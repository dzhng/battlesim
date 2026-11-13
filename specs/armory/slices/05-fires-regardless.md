# 05 · Fires regardless and reach, published; W09 amended

Map rows: U7, U8, W09, plus P4. No dependencies.

## Contract

K5. Two facts become contract methods: whether a weapon fires regardless, and
its reach per band. The simulation calls the method. The catalog view publishes
both under `derived`, apart from the authored fields. Digests don't move.

## Seam

In `crates/contract/src/weapons.rs`, inside `impl WeaponDefinition`. That impl
is already the one place that reads `targets`, through `against` and `engages`.

```rust
/// A default gun with unlimited rounds fires at an identified enemy it cannot hurt (W09).
pub fn fires_regardless(&self) -> bool;
/// Per layer it lists: range, and the unit classes it is limited to only when they
/// narrow what `TargetClass::found_in` that layer holds; `Missile` is interception, shown apart.
pub fn reach(&self) -> Vec<Reach>;
#[derive(Serialize)] pub struct Reach { pub layer: AltitudeLayer, pub range_m: f64,
    pub only: Option<Vec<TargetClass>>, pub intercepts: bool }
```

- **The sim's copy goes.** The private `fires_regardless` in
  `crates/sim/src/weapons.rs:286-291` is deleted. Its callers (around `:392`
  and `:1113`) call the method.
- **The catalog view.** `sim::fixtures` (`fixtures.rs:211`) writes
  `row["derived"] = { fires_regardless, reach }` after the serde row.
  `WeaponDefinition`'s serde is untouched, so `config_digest` and the rules
  identity don't move.
- **Then:**
  - re-bless `fixtures/catalog.json` with `BLESS_CATALOG=1` (on a merge
    conflict, re-bless; never hand-merge);
  - the TypeScript `WeaponRow` (`packages/scene-assets/src/units.ts:296`) gains
    `derived`.
- **The mechanics editor** shows `derived` read-only and never drafts it.
  `WeaponDefinition` has no `deny_unknown_fields`, so a drafted field would
  vanish silently.
- **W09:**
  - amend `specs/done/battle-foundation/requirements.md:257-263` to read: "a
    default gun with unlimited ammo fires regardless; some units have none";
  - fix the doc comment at `weapons.rs:42`.

## The human runs or sees

The `derived` key in `fixtures/catalog.json`, and the mechanics editor's weapon
panel. The sheet draws it in slice 08.

## Verification

Write these first.

`crates/contract/tests/weapons.rs`, on made-up rows:
- `only_an_unlimited_default_gun_fires_regardless`, across all four
  combinations of default and unlimited;
- `a_class_limit_shows_only_where_it_narrows_its_layer`: a ground row listing
  only `vehicle` narrows; `infantry` plus `vehicle` doesn't.

`crates/sim/tests/catalog.rs`:
- `every_published_weapon_row_carries_its_contracts_derived_facts`, generative
  and compared with the methods;
- then `the_browsers_catalog_view_is_current`.

`web/tests/mechanicsEditorDraft.test.ts`:
- `derived weapon facts are shown and never drafted`.

## Delegated to the implementer

- The key name `derived`, if a better existing convention turns up.
- The field names inside `Reach`.

## Must stay green, with digests unchanged and nothing re-blessed

- `crates/sim/tests/air_weapons.rs` `a_rifle_squad_keeps_plinking_a_helicopter_it_cannot_hurt`.
- The plinking test in `crates/sim/tests/weapons.rs`.
- `supply.rs`.
- Every replay and digest test.
- The `ConfigMismatch` tests.

## Feedback that would change it

If the user rejects publishing `reach`, drop it. The sheet would then need
`found_in` some other way, and that is the user's design call.
