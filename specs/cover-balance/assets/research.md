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

## Parameter effects and evidence

Existing exterior multipliers 1.4 / 1.8 / 2.4 produced equal-cell protection
32.2% / 49.0% / 63.8%. Reducing them to 1.15 / 1.4 / 1.8 produced fresh-seed
14.4% / 31.1% / 49.9%, with ordered cells throughout. Building spread stays 3.0;
its unchanged protection measures 71.9%. No fallback or second reduction owner
was needed. [Results](results.md) owns the complete interval tables and raw-data
links, including two small individual deviations from nominal five-point bands.

The initial eight-seed, fifteen-second middle rifle probe measured approximately
32 / 50 / 64 / 71% at baseline and 16 / 32 / 50 / 71% with the candidate.
The initial full candidate and fresh matrix exposed uncertain HMG cells.
Precision was expanded only where needed: middle/long HMG to 64 fresh pairs of
60 seconds, then medium short HMG to 32 fresh pairs of 30 seconds. Every final
cell interval half-width is below five points. Building grenade shelter needed
128 fresh pairs to reach the same precision; generic matched exterior splash
remains exactly unchanged. These are independent effects, not stacked promises.

The report clones one complete fixture/catalog snapshot across every arm,
including physical-map construction. Earlier baseline cohorts ran with unchanged
on-disk inputs throughout. Protocol revisions add explicit resolved factors and
maximum target displacement across exposure ticks; older retained artifacts do
not contain all later diagnostic fields. Raw samples retain their original
measurements. Stationary-target checks prevent movement from becoming an
unreported scatter treatment.

### Rejected attempts and validation

The first effects probe was invalid: decorative scripted bursts never invoked
soldier blast damage, and the eight-member garrison launched zero shots. Public
state showed occupied, identified façade members but `BlockedTrajectory` at the
shooter. Their averaged known position can lie inside the shell. The replacement
probe uses actual ground-directed grenades and a single occupied garrison seat;
exterior obstacle totals retain eight-member squads. Zero-shot comparisons are
rejected. Targeting is preserved and this limitation remains in the results.

Neutralizing the detector's protected arm made its regression fail on equal
harm. Flattening the three ordering inputs made the ordering regression fail
on exactly equal harm (5950 / 5950 / 5950). Restored tests pass, keep incoming
shot counts and successful-hit damage equal, and run in the default Cargo suite.
Independent review found and resolved the frozen-input gap. The configured Codex
CLI review could not run because its selected model was unsupported for the
signed-in account; an independent read-only review supplied the second opinion.

One initial full village run was stopped after about eight minutes without saved
rows because of a catalog-mutation concern. Inspection showed `village_report`
already snapshots its fixture before trials. Its restart uses the renamed
baseline so both arms share identifiers. The aborted run contributes cost, not
evidence.

### Experiment cost

Wall times describe this machine under concurrent load, not simulation speed.

| Study | Wall seconds |
| --- | ---: |
| Initial baseline / candidate probes | 7 / 9 |
| Baseline six-cell matrix | 165 |
| Expanded baseline HMG | 618 |
| Initial full candidate | 676 |
| Fresh confirmation matrix | 263 |
| Expanded fresh HMG | 1511 |
| Medium short HMG precision | 51 |
| Baseline position / fragment study | 719 |
| Final rifle / HMG position studies | 220 / 197 |
| Expanded fragment confirmation | 408 |

## Closeout evidence

Matched production frames passed the bounded trajectory-plausibility question;
[visual provenance](visual/README.md) records unchanged presentation debt.
The default Rust check passes, including both report tests. All 533 web tests
pass with one worker and unchanged timeout limits. One frozen decoder regression
was corrected to pair historical bytes with their own historical layout rather
than current catalog indices. Browser verification and full village comparison
are pending completion; the user-reported rifle-muzzle check is being examined
separately from protection tuning.

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
