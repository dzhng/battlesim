# Hierarchical skirmish doctrine AI

**Status:** slices 00 through 03 complete; closeout review next. **Last updated:** 2026-10-11.

**Goal:** replace the fixed skirmish opponent with one shipped, observation-bound
`BalancedDoctrine` policy. It chooses a compact force-wide state, deploys recon
at useful sensor standoffs, buys physical counters for discovered threats, and
pivots on low-air evidence or a bounded dwell timer. Seeded bounded mistakes
make the opponent readable and fallible without violating legality, fog of war,
replay determinism, or economy limits.

The policy is a commander, not an omniscient optimizer. It may buy a mediocre
counter or react one decision window late, but it cannot invent a capability,
read hidden health/ammo/orders, or issue an order the authority would reject.

## Next Agent Prompt

Run the whole-spec review and closeout. The native runner now drives both sides
through public observation, route, accept, and step seams and reports JSONL
status, contact/recon evidence, objective ownership, refusals, pivots, and the
battle digest. Before archiving, run the full workspace checks and browser
smoke, consolidate `choices.md`, and use `close-spec` to move this rationale to
`specs/done/`.

## Contract

Today `BalancedDoctrine::decide` receives a side-scoped `ObservationFrame`, faction,
entry site, rules, and a route-time callback, then emits ordinary `Order`s. The
battle calls it before a tick and replays accepted commands instead of rerunning
the policy ([`battle.rs`](../../crates/sim/src/battle.rs:1695),
[`skirmish_ai.rs`](../../crates/sim/src/skirmish_ai.rs:19)). Preserve that
authority boundary.

The planned seam has three layers:

1. **Doctrine state** chooses the current force-wide state and records low-air
   evidence, dwell, and pivot history.
2. **Order emitter** turns that state into legal purchases and ordinary
   movement/attack/deployment orders.

One production policy, `BalancedDoctrine`, implements the seam. A future policy
can implement the same controller interface without adding a second economy,
observation contract, or command path. The first tournament host should drive
both sides externally with `Battle::observe`, side-scoped route/sight queries,
  `Battle::accept`, and `Battle::step`, leaving the public scenario setup's single
browser AI host unchanged unless the probe proves a two-controller setup is
necessary.

This plan supersedes the completed unit-roster spec's “basic AI” checkpoint for
skirmish behavior only. Encounter `Defender`, transport, publication, and
catalog mechanics remain separate owners.

## Settled map

- Doctrine pivots use evidence plus a maximum dwell time. A plan gets time to
  develop, but persistent failure can trigger an earlier pivot.
- Randomness is seeded per battle, side, and policy. It only perturbs stable
  near-equal choices or delays a response within bounded limits. Candidate order,
  draw count, and fallbacks are fixed.
- Recon uses sensor-range staging: a reachable post is selected along the
  entry-to-objective approach from the unit's published sight range and facing.
  The authority owns route legality; the current policy does not yet perform a
  separate line-of-sight candidate search.
- The doctrine graph is authored and believable, but it is an implementation
  detail. If it creates more machinery than the evidence justifies, collapse it
  to a smaller evidence-driven state machine without changing the observable
  contract.
- Personality is emergent through purchases, recon lines, and movement. No
  player doctrine selector or HUD is added in this feature.
- Hard cutover: no old AI replay/config migration layer.

## Doctrine graph

The initial graph is a proposal, not an irreversible abstraction:

```text
Balanced opening -> Recon screen -> Probe -> Assault -> Consolidate
       |               |            |         |             |
       +-> Tank swarm +-> Air denial +-> Emergency defence <-+
       +-> Infantry pressure                         -> Recover
```

Transitions are legal only when the low-air evidence delay or maximum dwell rule
passes. Casualty-driven loss pivots and a separate mission-plan layer remain
follow-up work.

## Unknowns and landmines

- `ObservationFrame` exposes typed sightings and expiring contacts, never hidden
  enemy state ([`observation.rs`](../../crates/contract/src/observation.rs:612)).
- The current route callback answers travel time, not sight quality. A separate
  side-scoped sight query is a future refinement; the current policy uses the
  published sight range and leaves LOS authority in the simulation.
- Dedicated Stinger/Igla/SHORAD cards remain disabled for deferred airborne
  targeting ([`europe.json`](../../fixtures/units/roster/europe.json:383)). The
  first policy can use generic `low_air` weapons; dedicated air defence is a
  separate content/mechanics slice.
- AI draws must not share combat/damage/observation RNG streams. No rejection
  sampling or unbounded route/candidate loops.
- Objective eligibility remains owned by `objectives::can_hold`; helicopters
  can screen or attack but do not count as objective holders.
- Tournament rows must distinguish complete, refused, timeout, and no-progress
  outcomes. Generated skirmish sites must remain valid 3/5/7-site setups.
- Transient doctrine state and trace are not publication or digest state. Policy
  identity/configuration belongs in scenario/tournament metadata; replay still
  proves the accepted command stream.

## Scenario corpus and proof

The initial corpus covers named saved-map rows for opening, town, woods,
route-pressure, contested, supply, and air cases. Generated open/mixed/metro
request identity remains a follow-up to the native runner.

Reports measure status/result, objective ownership time, first contact, recon
sightings, doctrine pivots, refused orders, and the battle digest. Replay digest,
credit/living-value accounting, and instruction counts remain future harness
extensions.
Qualitative invariants belong in tests; exact winners are tournament evidence,
not brittle fixture expectations.

## Slice graph

| Slice | Contract and human-checkable result |
|---|---|
| [00-policy-seam-and-probe](slices/00-policy-seam-and-probe.md) | A pure observation-bound policy seam, seeded bounded choice, and recon/pivot probe with current behavior preserved. **Complete.** |
| [01-balanced-doctrine](slices/01-balanced-doctrine.md) | One hierarchical balance policy with authored transitions, counter-aware procurement, and evidence-driven pivots. **Complete.** |
| [02-recon-and-operations](slices/02-recon-and-operations.md) | Sensor-envelope recon posts, role/task assignment, support/recovery, and integration battles showing visible adaptation. **Complete.** |
| [03-corpus-and-tournament](slices/03-corpus-and-tournament.md) | Fixture-owned scenario corpus and deterministic native AI-vs-AI report; full balance run at closeout. **Complete.** |

No UI or renderer work is planned. A browser battle remains the final smoke
check; native policy probes and reports own the primary proof.

## Global invariants

- Simulation remains the only outcome authority.
- Policies consume only side-scoped observations and side-scoped queries.
- Every accepted AI action is an ordinary command and is replayable.
- Stable sorting precedes every seeded choice.
- Randomness cannot violate faction, budget, cap, route, target-layer, or
  objective rules.
- `BalancedDoctrine` is the only shipped policy. Policy identity currently
  lives in the native tournament metadata and the browser setup still selects
  its single built-in controller; adding selectable policy ids requires an
  explicit admission field later.
- If the doctrine graph grows machinery without improving the corpus metrics,
  delete the graph and retain the simpler state machine.

## Handoff checklist

- [x] Slice 00 red/green tests and current replay digest proof.
- [x] Slice 01 doctrine transition and counter-purchase tests.
- [x] Slice 02 recon standoff and integration battle evidence.
- [x] Slice 03 corpus repeatability, refusal/timeout rows, and quick tournament.
- [ ] Full `check`, full browser verification, and closeout tournament once.
- [ ] Run `review` before declaring the feature complete, then archive with
  `close-spec`.

