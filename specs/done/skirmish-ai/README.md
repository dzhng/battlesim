# Hierarchical skirmish doctrine AI

The skirmish opponent is an observation-bound `BalancedDoctrine` controller.
It consumes a side's published frame and side-scoped route knowledge, then
emits ordinary orders that the battle authority validates, records, and
replays. Its planner has a compact doctrine state, bounded seeded fallibility,
physical counter selection, recon standoff movement, logistics positioning, and
an external two-controller tournament host.

## Why this shape exists

The AI must react to what a player could discover. Enemy types and contacts come
from `ObservationFrame`; hidden health, ammunition, orders, and true geometry
never enter policy state. Counter admission therefore reads the same catalog
mounts and weapon target layers used by simulation. A discovered helicopter can
select an HMG or autocannon carrier, while a ground-only weapon is not treated
as an anti-air answer by name alone.

The policy's randomness is a separate deterministic stream derived from battle,
side, and controller identity. Each decision consumes one bounded draw. It
perturbs near-equal choices and delays reactions, while hard legality filters
remain deterministic. Transient doctrine state stays outside the battle digest;
the accepted command stream remains the replay authority.

Recon stages along the entry-to-objective approach using the unit's published
sight range and a facing toward the objective. `Battle::route_time` answers
travel time from a side's planning snapshot and rejects units the side does not
own. The simulation remains the authority for final route placement, visibility,
objective eligibility, and combat.

Doctrine transitions currently use low-air evidence, observed own-unit loss, and
maximum dwell. Force-shape states alter opening procurement; richer casualty,
objective, and task-group scoring is intentionally left as a future extension
of the same policy seam.

## Invariants and pointers

- Policy seam, doctrine state, procurement, and recon orders live in
  [`crates/sim/src/skirmish_ai/mod.rs`](../../../crates/sim/src/skirmish_ai/mod.rs);
  focused behavior tests live beside it in `skirmish_ai/tests.rs`.
- The live browser AI uses `Battle::skirmish_ai_turn`; external controllers use
  `Battle::observe`, `Battle::route_time`, `Battle::accept`, and `Battle::step`
  in [`crates/sim/src/battle.rs`](../../../crates/sim/src/battle.rs).
- Generated tournament maps use pinned `GenerationRequest`, `PreparedMap`,
  and `map_analysis::admit_skirmish` in
  [`crates/mapgen/examples/skirmish_tournament/`](../../../crates/mapgen/examples/skirmish_tournament/).
- Corpus rows live in [`fixtures/skirmish-corpus.json`](../../../fixtures/skirmish-corpus.json).
  Reports preserve complete, refused, timeout, and no-progress outcomes,
  admitted map identity/site count, objective ownership, contact/recon
  evidence, spend/living cost, doctrine pivots, refused orders, and battle and
  replay digests with replay parity.
- The tournament watchdog treats changes in published unit positions or goals
  as progress, so long approaches are allowed to reach their horizon instead
  of being classified as stalled solely because objectives have not changed.
- Existing simulation command admission, objective rules, sensing, replay, and
  catalog tests remain the authorities for legality and outcomes.

The initial corpus includes generated open/mixed maps at small, medium, and
large skirmish sizes, which exercise the generator's 3/5/7 admitted-site
contracts, plus saved-map rows for route pressure, contested objectives,
logistics/casualty pressure, and air contact. The corpus deliberately treats
winners as evidence rather than brittle fixture expectations; most rows may
end in timeout or no-progress while still proving deterministic control and
replay parity.

## Divergences and rejected paths

The planned authored transition graph collapsed into a compact enum because a
second intent graph would have duplicated order ownership. A separate sight
candidate query, full mission-task planner, and instruction-count report were
not added: the current policy has enough evidence to stage recon and select
legal counters, while those larger surfaces remain future work with clear seams.

The tournament host is external and drives two instances of the same policy.
This avoids changing the browser scenario contract until a second production
policy or runtime controller needs it. Saved maps remain in the corpus for
intentional edge cases; generated rows prove map identity and site admission.
The example's default mode now runs one fresh generated mixed small-map game
with a clock-derived seed; `--seed` makes that match reproducible and
`--max-seconds` controls its safety cap while `--corpus` retains the
deterministic regression sweep. A default run stops on the simulation's
terminal result when it arrives before the cap.

No visual surface was added, so there is no screenshot provenance for this
feature.
