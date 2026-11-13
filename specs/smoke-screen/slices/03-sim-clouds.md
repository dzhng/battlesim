# 03: capability, order, cloud collection and the cutover (dormant)

## Contract

- **The capability:** a unit row can carry smoke, as a capacity of charges plus
  the cloud it makes.
- **Popping:** `Order::PopSmoke` spends one charge per selected unit that has
  one and puts a cloud in the world. The unit's current order is never
  cancelled.
- **Lifetime:** clouds expire on their own, and resupply refills charges.
- **The cutover:** the old transient-prop machinery is gone.
- **Dormant:** no roster row carries smoke yet. **Every pinned digest is
  unchanged.**

## Seam

- **Catalog** (`crates/contract/src/catalog/`, in a new file because
  `catalog.rs` is over budget):
  - `Capabilities.smoke: Option<SmokeCapability { capacity: u32, service_s: f64,
    stock_per_charge: f64, cloud: SmokeCloudSpec }>`.
  - Validation: positive capacity, a positive price (a price of 0 gives smoke
    away free), and every `SmokeCloudSpec` field finite and positive.
  - Unlike flares, smoke needs no hull, so infantry can carry it.
  - `Capabilities` has `deny_unknown_fields`.
  - Update the hand-kept TS mirror at `packages/scene-assets/src/units.ts:201`.
  - Regenerate the catalog with `BLESS_CATALOG=1`.
- **Order** (`crates/contract/src/command.rs`):
  - `Order::PopSmoke { units: Vec<UnitId> }`, checked by
    `validate_command_units`.
  - It is handled in `Battle::apply` (`battle.rs:3270`), because
    `apply_units` can't change the world. It is never in the `replacing` list
    (`battle.rs:3283`): see the `UpgradeMove` precedent.
  - A unit without a charge is skipped with no refusal reason.
  - Add it to the exhaustive matches the compiler forces (`battle.rs:1289`,
    `1425`, `1545`).
- **Popping** lives in a new module, `crates/sim/src/smoke.rs`:
  `pop(&mut Unit, &mut SmokeClouds, tick) -> bool`.
- **Clouds** live in a new module, `crates/sim/src/world/smoke.rs`:
  `SmokeClouds { pop, advance(tick, hz), live(tick) -> impl Iterator<(SmokeCloud,
  CloudState)>, near(centre, reach), key_near(centre, reach) -> u64, digest }`.
  - It is held by the world geometry.
  - Adding or removing a cloud bumps **no** obstacle or navigation revision.
- **Unit state:** `Unit.smoke: Option<SmokeCharges { charges }>`, filled at spawn
  (`battle/spawn.rs`, next to flares).
- **Resupply** (`supply.rs`):
  - `Need::Smoke { capacity, service_s, stock_per_charge }`, served after
    `Soldier` (D24).
  - `Progress.smoke_s` is folded into the digest only when `unit.smoke.is_some()`.
    Today `units/digest.rs:139-145` folds every clock unconditionally; keep that
    for the existing clocks.
- **Refund:** no change. Refund value ignores smoke (`withdrawal.rs:86`).
- **Digest:**
  - Clouds fold in the old expiry slot (`battle/state_digest.rs:55`): their
    count, then each cloud. With no clouds, this is bit-identical to today's
    empty `expiries`.
  - `Unit.smoke` folds only when it is `Some`.
- **Deleted (D19):**
  - `PropBody.lifetime_s`;
  - its validation (`catalog.rs:1515`);
  - the "smoke screen" sentence in the `PropBody` doc (`catalog/props.rs:27-32`);
  - `Battle.expiries`, `add_prop`'s expiry, `schedule_expiry` and
    `expire_props` (`battle.rs:323`, `:803`, `:1845-1880`, `:2053`);
  - the Q28 tests (`crates/sim/tests/bodies.rs:468-560`).
  
  The specs under `specs/done/` stay as history.
- **Test carriers:** new rows `test_smoke_tank` and `test_smoke_squad`, extending
  `test_tank` and the test squad. Existing `test_*` and `capture_*` rows get no
  smoke (choices).

## What you can run

`cargo test -p sim --test smoke`, a new system test file.

## Tests (write them first)

Each test reads its numbers from the carrier row; none pins a value.
- A moving `test_smoke_tank` pops smoke, keeps its move and its queue, and loses
  one charge.
- Two selected carriers and one non-carrier: each carrier spends one charge,
  and the non-carrier is silently ignored, with no rejected-order event.
- With no charges left, nothing happens.
- The cloud is gone after its `life_s`.
- A pop changes neither `obstacle_revision` nor any side's navigation revision.
- A truck refills one charge per delivery, and only after replacement soldiers.
- Replaying a recorded battle with pops reproduces the same digest.
- **Every pinned digest is unchanged:** `exact-numbers`, `skirmish-*` and the
  publication `combat`/`stream`/`contact-lifecycle` records. This proves the
  digest-slot reuse.

## Delegated

You may choose module-internal names, how `SmokeClouds` is stored (a `Vec` or a
`BTreeMap` by id, as long as iteration is deterministic), and the order of
validation messages.

## Must stay green

The `sim` and `contract` tests, the pinned digests, and the catalog check.

## Feedback that would change this slice

If the user wants smoke on more units, that's data in slice 09, not this slice.
