# Battle foundation

This record preserves the original combined-arms foundation plan and the
village checkpoint it delivered. The plan is closed because its umbrella
continuation no longer owns the remaining work: transport, helicopters,
air-war, skirmish AI, and team skirmish each have their own current owner.
The closed [team-skirmish](../team-skirmish/README.md) record holds the 5v5
pacing evidence the old full-battle continuation was going to collect (AI
matches that finish on their own). The stress and frame-time work at the unit
cap moved to a future performance spec (user, 2026-10-19).

## Why the plan had this shape

The first playable loop needed one simulation authority, physical projectiles,
side-scoped knowledge, persistent battlefield remains, and a small recovery
economy before the roster or presentation grew. The village checkpoint made
those seams real without pretending that a short encounter was the eventual
full match.

The plan deliberately kept rules in physical and authored properties. Geometry
and combat remain Rust-owned; browser and native clients observe publications;
commands, seeded randomness, digests, and replay stay explicit. That separation
is why later transport, aircraft, and team-play work could replace the old
continuation slices without a compatibility layer.

## What shipped and what changed

- The village checkpoint established authoritative terrain and movement,
  observation and fog, physical flight and damage, garrisons and ruins,
  deployment, finite supply, weapon readouts, and the replayable encounter.
  Current owners are `crates/sim/src/battle.rs`, `movement/`, `knowledge.rs`,
  `flight/`, `weapons.rs`, `damage.rs`, `garrison.rs`, `deployment.rs`,
  `supply.rs`, `objectives.rs`, and `publication.rs`, with browser surfaces in
  `apps/battle-lab/` and `web/src/battle/`.
- Transport lifecycle is recorded in [transport](../transport/README.md), and
  helicopter movement and crash behavior in [helicopters](../helicopters/README.md).
- Persistent fixed-wing aircraft, radar emission and SEAD, layered AA,
  interception, fuel, dogfights, and withdrawal are recorded in [air-war](../air-war/README.md)
  and [air-war-flight](../air-war-flight/README.md). They supersede the old
  sortie and radar/AA slices.
- Objective capture, equal periodic income, edge reinforcement, victory
  scoring, and match reports are current simulation behavior in
  `crates/sim/src/objectives.rs`, `skirmish.rs`, and `settlement.rs`. The old
  slice that described these as future work was stale.
- The late-remains workload and generic publication delivery are implemented,
  but the broader optimization options proposed after the old endurance
  verdict were not completed here. Their measured failure remains useful
  evidence; the decision about the workload worth supporting belongs to the
  future performance spec.
- The old `/battle/combined-arms` continuation fixture was never created. A
  5v5 all-AI battle is the acceptance surface instead of a second umbrella
  scenario; its frame-time measure belongs to the future performance spec.

## Invariants that survive the archive

- The simulation is the only authority for commands, visibility, combat,
  objectives, and replay digests.
- Each side receives only its own observations; shared identification never
  reveals hidden geometry or state.
- Rounds are physical and collide with the first physical body they strike;
  wrecks and ruins remain world geometry, while corpses remain persistent
  visual records and do not block movement.
- Weapon engagement, guidance, deployment, carriage, supply, and objective
  eligibility come from authored capabilities and data, not role-name special
  cases.
- New authoritative state enters the digest and new observation state enters
  the publication layout. Presentation and animation never drive a rule.

Focused Rust tests and the registered browser checkpoints under
`crates/sim/tests/`, `web/tests/`, and `web/scenes/` are the executable proof
for the shipped foundation. Lab-only scenes remain diagnostic surfaces. The
[team-skirmish](../team-skirmish/README.md) record holds the multi-player
closeout evidence; stress at the unit cap is the future performance spec's.

## Rejected or superseded paths

The plan did not grow a second sortie engine, a separate radar/AA rules path,
or a giant full-match controller inside the village scenario. Those paths either
became dedicated shipped specs or would have duplicated current owners. The
separate radar-vehicle network remains a deferred idea in
[air-defence-radar](../air-defence-radar/README.md), where its distinction from
the shipped air-war radar contract is recorded.

## Planning provenance

[visualizations/roadmap.html](visualizations/roadmap.html),
[assets/interview-map.html](assets/interview-map.html), and
[assets/original-brief.txt](assets/original-brief.txt) preserve the original
scope walk and user decisions. They are historical planning provenance, not
current implementation instructions or visual acceptance baselines. Current
visual evidence lives beside the browser scenes and the dedicated feature
records named above.

The remaining planning ledgers and contracts in this directory are retained as
historical source material. They are not active handoffs; code and the current
feature records own behavior.
