# Unit concealment and spotting (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-10-28.

## Next Agent Prompt

The user wants every unit type to carry two sensing stats:

- **How hidden it is.** The more hidden a unit is, the closer an enemy has to get to spot it.
- **How well it spots hidden units.** 1 is normal. Below 1 it is worse and has to get closer. Above 1 it spots hidden units from farther away, but never past its own sight range.

The spotting stat already exists but cannot go below 1. The hiding stat does not exist. Concealment here belongs to the ground (forest, building), not to the unit. This page records what is there today and what the user has decided.

This changes battle outcomes. Run `tweak-mechanics` first, then `write-spec` to turn this page into a sliced plan, then `write-tests` (red first) for each slice. Update this section before you end each pass.

- [ ] Add a per-unit-type hiding stat and multiply it with the ground's concealment.
- [ ] Let the spotting stat go below 1, applied only against concealed targets.
- [ ] Settle the open questions below with the user before slicing.

## How sensing works today

Read `crates/sim/src/sensing.rs`. A target sample is identified when it is within the observer's directional sight range multiplied by a **concealment** factor (at most 1, lower is harder to spot), with clear sight and with foliage attenuation on top (`sees_point`).

- **Concealment comes only from where the target stands** (`target_concealment`). Forest ground gives one value for all infantry and another for all vehicles (`forest_concealment` in `crates/sim/src/world/forest.rs`, values `concealment_infantry` / `concealment_vehicle` in the forest rules, e.g. `fixtures/game.json`). A garrisoned squad gets its building's `sensors.building_range_multiplier`, scaled by shelter. The stronger of the two wins. In the open it is 1. Aircraft are always 1.
- **The spotting stat is `sensors.concealed_range_multiplier`** on each unit type (`Sensors` in `crates/contract/src/catalog.rs`). It defaults to 1, and roster units author 1.5 or 2. In `evaluate` it is applied as `(ground concealment × multiplier).min(1.0)`, so the cap at ordinary sight range is already there.
- **Validation forbids values below 1** (`catalog.rs`, "sensors.concealed_range_multiplier must be finite and at least 1"). The test `concealed_detection_bonus_cannot_reduce_ordinary_sight` in `crates/contract/tests/catalog.rs` pins that rule. The user's request reverses it, so that test changes on purpose.
- **The HIDDEN readout** (`concealed` in `sensing.rs`) is true when enough of a unit stands on concealing ground. It reads the same `target_concealment`.

## Decisions

- **The two concealments multiply** (decided by the user). Effective concealment = unit hiding stat × ground concealment. For example, a sniper team at 0.6 in forest (infantry concealment 0.25 in `fixtures/game.json`) is spotted at 0.15 of the spotter's sight range. A 1.5 spotter raises that to 0.225, and a 0.8 spotter lowers it to 0.12. It is always capped at 1 (ordinary sight range).
- **The spotting stat applies only against concealed targets** (my proposal, not yet confirmed). With the current formula, a 0.8 spotter would see an unconcealed tank in the open at 80% of its range. That makes it a general sight penalty, which the user did not ask for. Apply the spotting stat only when effective concealment is below 1.
- **The hiding stat reads like the existing concealment factors** (my proposal, not yet confirmed): a range multiplier where 1 is normal and lower is harder to spot. The user described it as "higher = more hidden". If they want that scale, it is a display/authoring choice over the same multiplier. Ask once while settling the open questions.
- **Where to build:** a battlegame worktree branched from `origin/staging`, not the main checkout.

## Open questions

- Does a unit's own hiding stat count in the open, with no forest or building? Under multiplication, yes. A 0.6 sniper team in the open is spotted at 0.6 of range. Confirm this is intended.
- Does the hiding stat apply to aircraft (for example the stealth fighters in the roster), which currently ignore concealment entirely?
- Should the HIDDEN readout reflect the unit's own stat, or stay about ground cover only? `tweak-mechanics` notes that HIDDEN publication is distinct from physical concealment.
- Should the field live in `Sensors` (for example `sensors.concealment`) next to the spotting stat? That is the obvious home. Confirm the name.
- Which unit types get non-default values, and what values? Recon and sniper infantry are the obvious candidates. Authoring roster numbers moves battle digests and parity fixtures (`fixtures/parity/`), so each value is a named decision.

## Where this came from

A 2026-10-28 conversation that started in a duet session. Nothing was changed or left on any machine, and there are no branches or scratch results.
