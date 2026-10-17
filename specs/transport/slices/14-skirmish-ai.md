# 14 · The skirmish AI rides

**Owns:** D16, D17, D18, L3 (AI). **Depends on:** 07, 13. **Outcome change:** AI skirmishes change (expected); re-bless `skirmish-purchases`.

Load `tweak-mechanics` first.

## Contract
- **Pairing is derived, not stored.** Each decision rebuilds pairs from `frame.own`: a squad and the carrier whose host it is, or a free carrier that will collect it. A new `crates/sim/src/skirmish_ai/transport.rs` owns this.
- **Carrier choice is data.** For each infantry or recon squad, the smallest carrier whose `carriage.seats` is at least the squad's size: 3 for 3-man teams, 8 for bigger squads. The AI never buys 16-seat helicopters for pairing, but may board any free carrier with room. It never checks roles or families.
- **Purchases** (D16, D17):
  - Separate cards, as today.
  - The purchase rotation (`skirmish_ai/mod.rs:480-541`) buys a carrier after each unpaired squad when credits and the unit cap allow.
  - A dead transport is rebought; a dead squad is replaced, and its transport takes the new one.
- **Movement.**
  - A squad heading for an objective (`mod.rs:672-716`) first boards its carrier.
  - The carrier drives toward the objective and unloads at `unload_point`.
  - Squads already near their objective walk.
- **`unload_point(frame, objective, catalog)`** (D18):
  - A pure function that stops short of the objective, outside the weapon reach of every enemy the side has **identified**.
  - It never uses hidden information (`specs/done/skirmish-ai/README.md:12-16`).
  - With no identified threat, it unloads a fixed share of the way short of the objective. That share is a new value in `fixtures/skirmish-ai.json`.
- **Passengers are not lost units.** Units aboard are skipped for orders, and `lost_unit` (`mod.rs:247`), assignment pruning (`mod.rs:341`) and per-kind counts (`mod.rs:393`) keep counting them.

## What a human can see
An AI-vs-AI skirmish where squads ride to the objectives and get out short of them.

## Verification
- Tests in `crates/sim/src/skirmish_ai/tests.rs`, or in `crates/sim/tests/transport.rs` if they need a full battle:
  - `the_ai_drives_its_squads_and_unloads_short_of_seen_weapons`;
  - `the_ai_ignores_unseen_enemies_when_choosing_where_to_unload`;
  - `a_lost_transport_is_rebought_and_a_lost_squad_replaced`.
- Re-bless `fixtures/parity/skirmish-purchases.json`.
- **One played probe**, with a pinned seed, AI versus AI, in the background with a visible progress line. It is the only expensive run, and it answers whether the AI really uses transport. Record what it shows in [choices.md](../choices.md).

## Delegated
- The share short of the objective.
- The order in which purchases interleave.
- When a squad near its objective walks instead of riding.

## Must stay green
Fog-bound AI tests, objective tests, the AI's air-counter purchases.

## Feedback that would change it
How the AI looks in play: carriers driving into fire, or squads walking too far.
