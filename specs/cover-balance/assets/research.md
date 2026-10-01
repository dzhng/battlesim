# Protection measurement

## Fixed contract

The baseline is `4dad9b0`: completed main `3e512a1` plus the property-preserving wreck identifier rename. Weapon ranges, launch speed,
nominal aim height, projectile fall, hit damage and firing cycles stay fixed.

The incoming-scatter calibration uses a durable unarmed soldier, a stationary
rifle squad or jeep HMG, and paired seeds. Both arms contain identical cover
geometry; the control neutralizes only its incoming spread multiplier. A ground
body keeps eligibility complete without intercepting the nominal aim. A garrison
uses its existing building shelter instead of exterior cover. Calibration is
per-soldier hit-rate protection; real obstacle/squad rows separately describe
interception and total harm. Explosive splash is measured separately.

Each cell reports `1 − sum(protected harm) / sum(control harm)`, incoming rounds,
physical target hits, prop impacts and actual range. Zero-control samples invalidate
a comparison. Seeds are paired blocks, not a promise of identical subsequent RNG
consumption. A deterministic paired bootstrap supplies the 95% interval; increase
samples only for cells whose interval half-width exceeds five percentage points.
Tier aggregation gives equal weight to rifle/HMG at short/middle/long distances.
Targets are approximately 15%, 30%, 50% and 70%, with a proposed five-point band.
Cell results remain visible; an aggregate cannot hide a failed weapon/distance.

## Experiment order

1. Verify that the report detects neutralized cover and keeps damage per hit fixed.
2. Record the existing scatter baseline on a cheap middle-distance rifle case.
3. Sweep existing scatter factors, keeping only meaningful ordered candidates.
4. Expand to both weapons and three distances; confirm on fresh seeds.
5. Check real interception, garrison fragment exposure and whole-village outcomes.

A bounded scatter sweep precedes any damage/exposure fallback. A fallback is
allowed only if the sweep cannot meet the contract without implausible shots or
major weapon/distance failures. No hidden extra miss roll or duplicate reduction.

## Attempts and parameter effects

The initial middle-distance rifle comparison (eight seeds, fifteen seconds) measured
32% / 50% / 64% / 71% protection with the existing factors. The first hypothesis
uses 1.15 / 1.4 / 1.8 / 3.0: weaker exterior spread, existing building spread.
It measured 16% / 32% / 50% / 71% in that same comparison. Full baseline means
across both weapons and three distances are 31.6% / 47.2% / 61.7% / 69.9%.
These are development results, not final accepted tuning. Rifle intervals meet
the precision criterion; middle/long HMG intervals require more exposure.

The report detector passed, then failed as intended when its protected comparison
was deliberately neutralized. The restored detector compares eight seeds and
checks fewer hits, equal incoming shots, equal damage per hit and exact equality
when all cover factors are neutral. Additional real-position and fragment rows
use the same battle/report owner.

The village baseline now uses the name-cleanup checkpoint so before/after share
catalog identifiers. The earlier in-progress full run was stopped after about
eight minutes without saving rows; those research costs are included as an
aborted attempt, not a result. The report snapshots its fixture before running
trials; future report runs preserve that frozen-input contract.

The first effects study is **invalid**. Its scripted decorative bursts never
called soldier blast damage, and the eight-member garrison arm launched zero
shots. A public-state probe showed `Inside` garrison state, identified façade
members, and `BlockedTrajectory` at the shooter. The visible members' averaged
position can lie inside the shell. No targeting rule is changed by this pass.
The report instead measures a single occupied building firing position and
uses eight-member squads for exterior obstacles. Explosive comparison uses
actual ground-directed grenades, with direct hits excluded and detonations
counted. Generic exterior tiers are expected to add no fragment protection;
existing building shelter is the separate fragment owner.

Expanded HMG baseline: 64 paired seeds, 60 seconds at middle/long ranges,
618 seconds wall time under concurrent machine load. All eight intervals now
have half-width below five percentage points. Light measured 32.7/34.0%, medium
48.8/49.5%, heavy 63.7/63.6%, building 72.0/72.7% at 275/500 m.
Eligibility is recorded before exposure; these stationary, durable targets do
not die or relocate during the scatter comparison. Raw per-seed data is retained
alongside this human-readable summary.

The precision-expanded baseline's equal-cell means are 32.2%, 49.0%, 63.8%
and 72.4%; all individual 95% interval half-widths are below five points.
Reports keep raw samples in compact JSON; this document owns interpretation,
so data serialization does not add thousands of unhelpful diff lines.
The report now clones one frozen fixture/catalog snapshot across every arm,
including physical-map construction. Baseline artifacts produced before that
change ran with unchanged on-disk inputs throughout their cohort.

## Mechanics review

A rifle squad firing at infantry behind a crate should see some rounds scatter
into the surrounding dirt; behind sandbags or a wall, the surviving direct hits
still hurt normally. Wall collision adds its own stronger protection. A trunk
helps only nearby infantry with the trunk toward the threat; the rest of a forest
provides concealment through its existing reader. A garrison uses shelter spread,
never the exterior tier again. Infantry beside a vehicle lose its directional
cover when it drives away, while a later wreck is an ordinary cover prop. Fire
at the vehicle itself still meets armor rather than infantry cover. Nearby shell
bursts retain ordinary exterior fragment damage; garrison fragment exposure is
handled once by shelter. These outcomes require fixture tuning only, with no
new aim, collision, movement, armor, knowledge or damage rules.

Report regression checks pass after restoration. Neutralizing the detector's
protected arm made it fail on equal harm; flattening all three ordering inputs
to the same multiplier made the ordering test fail on exactly equal harm
(5950 / 5950 / 5950). Restored tests pass and the focused cover and damage suites
pass. Independent review found and resolved the frozen-input gap; no further
measurement code findings remain. The configured Codex CLI review could not run
because its selected model was unsupported for the signed-in account, so an
independent read-only review supplied the second opinion instead.
