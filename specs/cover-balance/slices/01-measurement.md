# 01 — Establish the cover measurement

## Contract and seam

Measure effective protection at the simulation's shot/impact/damage seam before changing balance. Reuse deterministic catalog and battle fixtures; no production test hooks or second cover implementation. The first useful deliverable is a repeatable paired-fire report showing what each tier actually does.

## Work

1. Inspect the completed main-pass cover, building shelter, projectile flight and catalog code. Save that baseline before editing rule numbers.
2. Invoke write-tests. Retain useful directional-cover and replay tests. Inspect the mixed wall/crate regression rather than treating its 15% floor as a heavy-cover target.
3. Build tier-isolated comparisons with fully eligible soldiers, durable test targets, stationary shooters, matched geometry and paired seeds. Avoid early casualties changing the number of exposed targets or ending fire. Use rifle and HMG separately at short, middle and long distances within each weapon's completed main-pass effective range.
4. For the scatter contribution, retain identical bodies and flight collision in both arms and change only the applicable cover/shelter spread to neutral in the control. A body stopping every round is not a useful scatter calibration fixture. Separately compare the real protected position with an exposed position to describe total protection including interception.
5. Report launched shots, hits and damage where existing instrumentation permits, exposure duration, cover eligibility, distances, seeds, and uncertainty. Primary reduction is `1 - protected_damage / control_damage`; reject zero-control samples rather than inventing a ratio. Match seed inputs without assuming paired RNG consumes remain aligned after outcomes diverge.

## Acceptance

Baseline distinguishes scatter from interception, light/medium/heavy from mixed coverage, garrison from exterior cover, rifles from HMG, and direct fire from explosive splash. Use enough samples that statistical uncertainty is smaller than the proposed five-percentage-point tuning tolerance. Reuse the smallest existing report/test surface; internal helper names and sample count are delegated, not the meaning of the comparison.

Run focused sim tests for cover and damage, including replay. Preserve all existing production behavior in this measurement slice. Pin classification/direction and relative protection as meaningful contracts; do not make tests mirror a list of fixture constants or depend on one lucky seed. Calibration bands belong to reproducible balance evidence, not fragile unit tests.

If visual captures are useful, crop the target squad, obstacle and impact area. Compare-screenshots against the completed main-pass baseline; show with preview-shots; run screenshot-critique last before accepting a shot. No visual art change is required by this slice.

The slice is complete when the human can inspect one paired report and understand the current protection and its limits. Weapon range, projectile gravity, target aim height and firing cycles are frozen inputs.
