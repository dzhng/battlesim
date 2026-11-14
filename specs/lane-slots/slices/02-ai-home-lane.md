# 02 · AI home-lane pull

**Depends on:** 01. **Changes digests:** only commit 2c, the named re-baseline.

Run `tweak-mechanics` before starting: this changes how the AI behaves.

## Contract

Every AI on both sides scores its own lane (slice 01's `homes`) a little
better. The behaviour is the requirement, not a number
([D8](../README.md#decisions)):

- **(a)** At match start, every AI takes its home lane on admitted maps of every
  size, including when a human holds the centre road.
- **(b)** Later, a lane with clearly more enemy pressure or open objectives
  still pulls an AI off its home lane.

Slots 4–5 have no home and get no pull. There is no rule about human-held
lanes ([D2 withdrawn](../README.md#decisions)).

## Commits, in order

1. **2a · Wire at zero (no outcome change).** Add `home_lane_bonus` to
   `TeamTuning` (`team/mod.rs:40-61`, `deny_unknown_fields`) and to
   `fixtures/skirmish-ai.json` at `0.0`. The score (`team/mod.rs:307-326`)
   subtracts it when the candidate lane is the player's home entry; this covers the
   permutation and the subset-search paths. Test literals (about 18, mostly
   `team/tests/*`) take the field; a helper is allowed. Digests unchanged,
   because subtracting zero changes nothing.
   - Add behaviour (b) to `crates/sim/src/skirmish_ai/team/tests/planning.rs`
     with an explicit non-zero literal: a pressured or open objective pulls an
     AI off home; a slot-4 AI gets no pull.
2. **2b · Probe (scratch, no commit needed).** Sweep bonus values × sizes
   (S/M/L/XL) × human slot (none, 1, 2, 3) on admitted maps, reading the first
   plan from `team::plan`, plus a later-pressure case for (b). Put the probe in
   `throwaway/`, or make it a mapgen example if it earns a place. Pick the
   smallest value that passes (a) everywhere and still passes (b). The value is
   a pick, not a target ("numbers are not requirements").
3. **2c · Named re-baseline.** Set the value. Add a test with the shipped
   tuning, e.g. `every_ai_opens_on_its_home_lane_on_every_admitted_size`
   (`crates/sim/tests/skirmish_team.rs` or a mapgen test), covering the
   human-on-centre case. Move help requests' human lane (`help.rs:60`) onto
   `homes` (A8). Rerun the skirmish AI tests and the tournament corpus
   (`crates/mapgen/examples/skirmish_tournament`). The commit message names
   the outcome change and lists every pinned value that moved; say so plainly
   if none did (precedent: commit `24c40a9a1`; `BLESS_PARITY=1`, see
   `crates/sim/README.md`).

## What a human can see

The probe's table (bonus × size × human slot → AIs on home lane). Then a 3v3
watched from the start: each AI heads down its own numbered road.

## Verification

Behaviour tests above; the probe table; corpus outcome summary before and after
in the 2c commit message.

## Delegated

The probe's shape and seed set; whether the probe stays as an example; the test
literal helper.

## Must stay green

All digests through 2a; help-request, party and stance tests (their
outcomes may change only in 2c, and only through A8 or the new term).

## What would change this slice

If no single value passes (a) on every size and (b), **stop and report**. Do
not add per-size values or special cases, and do not loosen (a) or (b). The
user decides how to reshape it.
