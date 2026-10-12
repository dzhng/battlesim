# Slice 03 — Observation-bounded skirmish artillery

## Contract unlocked

The skirmish AI can buy and use artillery through the ordinary order path. It may issue normal Attack/Attack Move orders and explicit Ground salvos based only on `ObservationFrame` evidence. It can extrapolate a last-seen/contact area or firing report and make bounded public-objective/route guesses, but it never reads hidden unit position, order, health, ammunition or exact emitter state.

## API seam and owner

- Keep `SkirmishPolicy::decide` and its route callback in `crates/sim/src/skirmish_ai/mod.rs`.
- Add pure evidence-to-guess helpers within the skirmish AI owner. They accept an `ObservationFrame`, public rules/catalog data and deterministic policy memory; they return `Option<TargetRef::Ground>` or no order.
- Read contacts and firing evidence from `crates/contract/src/observation.rs` and `crates/sim/src/knowledge.rs`; do not pass a `Battle` reference into the policy.
- Keep procurement role/category selection in the existing doctrine. The new artillery role is data, not an ID switch.

## Required behavior and tests

- An affordable admitted artillery unit can be selected for opening/reinforcement procurement without starving required logistics or violating caps.
- The AI issues ordinary attacks when an identified target is suitable; no artillery-only command or automatic hit is added.
- A contact/firing report can produce a ground salvo at an evidence-derived centre or bounded prediction. Hidden emitter position is never consulted.
- Observed movement may extrapolate only over a bounded horizon; stale/expired/out-of-range evidence produces no salvo.
- Public objective/route guesses are deterministic, range-checked and visibly fallible. They must not turn into unavoidable scripted hits.
- `ReturnFireOnly` and existing engagement gates remain effective.
- Fixed-seed decisions, commands and replay digests are identical on replay.

Add focused tests to `crates/sim/src/skirmish_ai/tests.rs` and a short real-artillery scenario to the skirmish integration tests. Use a synthetic observation frame to test each evidence source separately, then one fixed-seed battle to prove the composed path.

## Human-visible artifact

Produce an ignored fixed-seed AI probe report containing purchased artillery IDs, emitted Attack/Ground orders, evidence labels, observed versus hidden target positions (for the harness only), impacts and original/replay digests. The report must demonstrate at least one miss caused by a relocation or uncertainty.

## Verification

```bash
CARGO_TARGET_DIR=throwaway/target cargo test -p sim --test sim skirmish_ai skirmish
```

## Firewalls and delegated choices

Do not add omniscience, hidden-state mirrors, special-case unit IDs, new command types, guaranteed hit placement or a second contact schema. The exact scoring/order among equally valid evidence sources, salvo cadence and deterministic random seed salts are delegated; document them in the slice choices ledger and keep them observation-derived.

## Acceptance and feedback

The slice is accepted when the AI's command stream can be explained entirely from its observation frame and replay digests match. If a desired guess requires data absent from `ObservationFrame`, stop and name the missing public evidence instead of widening the observation with hidden fields.

## Pass evidence

The skirmish policy now procures the `artillery` role, emits ordinary identified
target attacks, and emits deterministic ground guesses from fresh contacts and
bounded observed movement. Eighteen focused policy tests and twenty-five
skirmish integration tests pass. The integration scenario also records an AI
artillery purchase and replays every tick to the same digest; the public policy
tests cover stale/expired evidence, bounded prediction, objective range checks,
and engagement gates.
