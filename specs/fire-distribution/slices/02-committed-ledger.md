# 02 Committed-fire ledger

## Contract

A pure owner that answers "is this target doomed for a shot arriving by t?"
from a list of counted rounds, plus the weapon-row flag. Nothing reads it yet,
no row sets the flag, digests are unchanged. Decisions D4–D10, A5, A7–A9.

## Seam

- `WeaponDefinition.avoids_overkill: bool` in `crates/contract/src/weapons.rs`,
  `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`, with a doc
  line. Inherited through `extends` like every field.
- `crates/sim/src/weapons/overkill.rs`:

  ```rust
  pub struct Committed { /* BTreeMap<(Side, UnitId), Vec<Hit>> */ }
  struct Hit { arrive_tick: f64, damage: f64, interceptable: bool, seq: u64 }
  impl Committed {
      pub fn add(&mut self, side: Side, target: UnitId, hit: Hit);
      /// Counted damage arriving by `arrive_tick` after the Trophy discount.
      fn landing_by(&self, side: Side, target: UnitId, arrive_tick: f64, charges: u32) -> f64;
  }
  pub fn remaining_hp(unit: &Unit) -> f64; // vehicle hp, or living soldiers' sum
  pub fn hit_damage(def: &WeaponDefinition, rules: &Rules, target: &Unit, from: V3) -> f64; // A7
  pub fn doomed(c: &Committed, side: Side, target: &Unit, arrive_tick: f64) -> bool;
  ```

  Hits are sorted by (arrival, insertion order). The Trophy discount absorbs
  the first N interceptable hits by arrival across the whole list, then
  `landing_by` sums the remaining hits with arrival ≤ `arrive_tick` (A5).
  The internal shape is the implementer's; the semantics are not.

## Verify

Unit tests inside `overkill.rs` (the ordering is the complex logic):

- Hits after the cutoff don't count; a tie counts.
- One Trophy charge absorbs the earliest interceptable hit, not a later one
  and not a non-interceptable one; charges beyond the interceptable hits
  absorb nothing more.
- A squad's remaining health is its living soldiers' sum; a dead soldier adds
  nothing.
- A flagged row that extends a parent inherits the flag; a row that omits it
  is off (contract test).

## Stays green

Everything; digests unchanged. `catalog.json` unchanged (no row sets the flag).

## Delegated

- Internal container shape and `seq` source.
- Whether `remaining_hp` lives in `overkill.rs` or beside `Unit::max_hp`.

## Feedback that would change it

None expected before slice 03 runs it in play.
