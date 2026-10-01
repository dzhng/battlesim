# 02 — Tune protection through physical scatter

## Contract and seam

Existing fixture-owned cover factors feed the existing launch-scatter calculation. Target approximately 15% light, 30% medium as the proposed middle value, 50% heavy and 70% building/garrison protection in the representative measurements from slice 01. Trees remain medium; ruins remain heavy at 50%. No new forest-area protection or extra miss roll.

## Work

Invoke tweak-mechanics and write-tests before tuning. Narrate rifle/HMG fire into infantry behind each obstacle, infantry inside buildings, infantry beside a vehicle that drives away, direct fire at vehicles, and shell bursts near protected infantry. Preserve cover helping infantry rather than granting an armor buff to the vehicle itself.

Tune only the existing authored scatter factors first. Use a proposed tolerance of approximately five percentage points on the representative aggregate, reporting each distance/weapon separately. Require sensible tier ordering and disclose deviations; do not claim one factor guarantees identical reduction at every range. Preserve normal aim heights and the main pass's projectile fall/lifetime behavior.

Buildings use existing shelter strength and spread interpolation. Avoid also applying the generic exterior-body tier to a garrison shot. Review existing fragment exposure separately: a missed explosive round can still injure infantry, and there is already a building fragment multiplier. Do not silently multiply another full building reduction on top of it. Keep physical interception, direct-hit damage and blast effects identifiable in the report.

If one bounded scatter-tuning sweep cannot produce meaningful ordered protection without implausible flight/impact patterns or major weapon/distance failures, damage reduction is authorized as a fallback. Document the failed trials and use one existing owning damage/exposure path; do not add hidden miss rolls, per-unit special cases, or duplicate full-strength reductions. Record the combined result and which weapon classes it covers. The exact fallback composition is a delegated decision conditional on evidence, not a requirement to add it.

## Acceptance and review

Use the slice 01 paired measurements and narrow cover/damage/garrison tests. Establish behavior regressions that covered infantry benefit and tier ordering holds in controlled conditions, without merely asserting tuned constants. Preserve deterministic replay and existing vehicle cover geometry, movement/leaning and knowledge rules.

Run quick village reports for the rule change; run one full report at balance closeout. Compare with the saved completed-main-pass baseline and explain outcome changes rather than retuning unrelated weapons to hide them.

Produce one representative rifle/HMG firefight capture with the protected squad and ground impacts visible. Judge only cover benefit and projectile impact spread in this crop. Use compare-screenshots, preview-shots, then screenshot-critique as the last acceptance check. No new UI badge, model, foliage or destruction art is required. User feedback on protection strength may change the tuning; it does not authorize object architecture changes.

Finish with a compact measured table of target, achieved reduction, weapon/distance scope, mechanism and uncertainty. Update the README with the accepted middle value and any fallback decision.
