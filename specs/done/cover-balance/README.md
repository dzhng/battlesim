# Cover balance and wreck clarity

Cover protects infantry primarily by spreading incoming direct fire. The tuning
makes light, medium and heavy protection distinct while keeping successful hits
fully damaging. Controlled scatter measures about 15%, 30% and 50% protection;
building shelter remains about 70%. [Results](assets/results.md) contains the
measured intervals, obstacle totals and whole-battle comparison.

## Why these effects stay separate

A tier describes extra launch spread, not a guarantee that every squad behind
any object loses a fixed percentage less health. A wall can also intercept
rounds; formation can turn a miss into a hit on another soldier. Calibration
therefore isolates one fully eligible stationary soldier, while separate rows
measure actual squad positions and catalog obstacles. Each weapon/distance case
has equal weight, with all individual results retained.

Building measurements use an actual occupied firing seat. Existing targeting
can withhold fire when a multi-façade squad's known center falls inside the
shell. Zero-shot cases are rejected rather than called perfect protection.
This work preserves targeting and does not certify that existing limitation.
Building fragment exposure is measured separately and applied once; exterior
cover does not acquire a generic fragment reduction.

The bounded scatter experiment meets the aggregate targets without a damage
fallback or extra miss roll. Weapons, flight, collision, armor and concealment
remain the baseline inputs. Trees provide nearby directional trunk cover;
forest concealment remains a separate effect. Ruins retain heavy cover.

## Object meaning and invariants

Live vehicle cover continues to come from weight. The first wreck is an ordinary
catalog prop, and generic prop cover remains explicit: an immovable wall and
immovable rubble need different protection. The catalog's live-to-initial-wreck
cover consistency check remains at load time.

`heavy_wreck`, `medium_wreck` and `light_wreck` name categories of remains.
Existing durability, replacement dimensions, footprint, blocking and removal
behavior are preserved. No special wreck state, inheritance, body field,
compatibility alias or new destruction artwork is added. Historical parity
bundles keep their own catalog; packed historical bytes retain their matching
layout. Appearance source names are provenance, separate from gameplay IDs.

## Code and evidence

- `fixtures/game.json` owns spread and building exposure tuning.
- `crates/sim/src/cover.rs` owns directional cover eligibility and body tiers;
  `crates/sim/src/weapons.rs` applies incoming spread.
- `crates/sim/src/damage.rs::shelter_spread` and `fragment_exposure` own the
  separate building effects.
- `crates/contract/src/catalog.rs` resolves authored catalogs and enforces
  live-to-initial-wreck consistency; `fixtures/catalog.json` is generated data.
- `crates/sim/examples/cover_report.rs` owns paired calibration and its default
  Cargo regressions. `web/tests/observation.test.ts` preserves the frozen
  decoder oracle's layout/byte contract.
- [Research](assets/research.md) records parameter effects, failed probes and
  their costs. Decorative burst events were rejected as a damage experiment;
  zero-shot garrisons were rejected as protection measurements.
- [Choices](choices.md) records decisions the plan left to implementation.

## Visual provenance

[Matched production captures](assets/visual/README.md) preserve the frozen village
rules and source checkpoint, baseline and tuned rifle/HMG frames, and local comparisons. These are
project-owned production-renderer screenshots judging whether reduced spread
introduces visibly implausible trajectories. They are reference evidence, never
runtime art. Independent critique found no conclusive new trajectory regression;
unchanged dust, occlusion and tracer presentation limits remain documented there.
Images support the bounded plausibility check, while the native paired report establishes
percentages.
