# 00 — Policy seam and decision probe

This slice proves the policy boundary before adding doctrine behavior.

## Contract

Split the current monolithic `SkirmishAi` responsibilities into a small
observation-bound policy seam and an adapter that owns cadence, route queries,
catalog admission, and command emission. The policy receives a typed context
containing side, faction, entry, `ObservationFrame`, `Rules`, and side-scoped
route/sight queries. It returns ordinary order intents or `Order`s; it never
reads `Battle`, `WorldGeometry`, or private enemy state.

Keep one production implementation, `BalancedDoctrine`, and preserve the current
basic behavior until later slices change it. Keep transient policy memory outside
the digest. Give the policy a dedicated seeded `Rng` stream derived from battle
seed, side, and policy id. Stable sorting and fixed draw counts are mandatory.

## Probe

Add pure decision fixtures for no contacts, typed ground contact, airborne
contact, contested objective, casualty/suppression pressure, insufficient
credits, and blocked route. Resolve the smallest side-scoped query needed to
score a recon post; reuse the existing sight-analysis owner rather than writing
another ray caster.

## Verification

- Existing focused skirmish tests remain green.
- Same seed and observations produce the same intents and command stream.
- Different seeds vary only bounded near-equal choices or response delay.
- Randomness never changes faction, cap, budget, route legality, target layer,
  or objective eligibility.
- A live run and `Battle::from_replay` retain identical digests.
- Hidden enemy changes with the same observation do not change policy output.

No browser scene or screenshot is required for this slice.

## Delegated decisions

Internal module names and whether the adapter uses an enum or trait are open.
Do not add a second policy config owner, compatibility wrapper, or public
controller matrix unless the probe demonstrates a concrete need.

