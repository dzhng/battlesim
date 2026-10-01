# Measured cover protection

The accepted setting changes exterior launch spread only. Building spread and
fragment exposure remain unchanged; direct hits keep their full damage. No
extra miss roll or damage fallback was added.

## Scatter contribution

Each cell compares one fully eligible durable soldier against matched bodies
with neutral spread, using paired seeds. The tier summary gives equal weight to
the six weapon/distance cells. It is not a guarantee for a whole squad behind
every obstacle. Interception and formation/exposure differences are reported
separately below.

| Tier | Target | Baseline | Confirmed | Largest cell 95% interval half-width |
| --- | ---: | ---: | ---: | ---: |
| light | ~15% | 32.2% | 14.4% | 2.7 points |
| medium | ~30% | 49.0% | 31.1% | 3.6 points |
| heavy | ~50% | 63.8% | 49.9% | 4.7 points |
| building | ~70% | 72.4% | 71.9% | 3.4 points |

All cell uncertainty is below five percentage points; ordering holds throughout.
Medium rifle at 200 m (35.2%) and building HMG at 80 m (75.2%) slightly exceed
their nominal five-point bands. Their means remain within the agreed aggregate
tolerance; these small deviations are disclosed rather than hidden in a mean.

Values below are reduction with its 95% paired-bootstrap interval. Distances
are nominal; each raw sample also records its actual source-to-receiver range.

| Weapon / nominal range | Light | Medium | Heavy | Building |
| --- | --- | --- | --- | --- |
| rifle / 60 m | 13.4% (12.5–14.5) | 31.1% (29.1–32.9) | 49.4% (47.8–50.9) | 72.7% (71.2–74.4) |
| rifle / 200 m | 15.1% (12.1–17.5) | 35.2% (32.9–37.5) | 52.6% (51.0–54.4) | 71.6% (70.0–73.1) |
| rifle / 350 m | 12.3% (10.6–14.0) | 27.7% (25.1–30.4) | 46.0% (43.6–48.6) | 69.0% (66.2–72.8) |
| hmg / 80 m | 13.9% (11.7–15.6) | 29.3% (27.3–31.6) | 52.9% (47.7–57.1) | 75.2% (72.1–78.7) |
| hmg / 275 m | 16.9% (14.6–19.3) | 33.1% (30.1–36.3) | 50.7% (47.3–54.0) | 71.8% (69.4–74.4) |
| hmg / 500 m | 15.0% (12.3–17.7) | 30.4% (27.0–34.2) | 48.1% (44.6–51.8) | 70.9% (67.4–74.2) |

Fresh confirmation uses seeds 128–135 for the original six-cell matrix,
64–127 for the precision-expanded middle/long HMG cells, and 144–175 for the
medium short HMG cell. Exposures are respectively 30, 60 and 30 seconds.
[Fresh matrix](scatter-confirm.json), [expanded HMG](scatter-confirm-hmg.json),
and [medium short HMG](scatter-confirm-short-hmg.json) retain every seed pair.
The baseline is completed main plus the property-preserving wreck rename;
[baseline matrix](scatter-baseline.json) and [expanded baseline](scatter-baseline-hmg.json)
retain unchanged weapon, aiming and flight inputs.

## Total position protection

These comparisons use catalog obstacles held intact, with eight-member squads
behind a wide low barrier versus an exposed squad. They include geometry,
formation/exposure and scatter; they are not pure interception percentages.
Buildings use one real occupied garrison firing position. Eight soldiers around
several façades can give the shooter an averaged known center inside the shell
and cause it to hold fire. Such zero-shot cases are rejected, not scored as
protection. The targeting rule is outside this tuning pass.

| Position at 200 m | Rifle | HMG |
| --- | --- | --- |
| light | 45.5% (42.0–48.9) | 53.8% (49.1–57.9) |
| medium | 57.0% (54.0–59.6) | 64.0% (60.2–67.1) |
| heavy | 60.2% (57.3–62.9) | 66.5% (61.5–70.9) |
| building | 73.3% (71.9–74.7) | 71.2% (66.4–75.6) |

Sixteen paired seeds, 60 seconds per arm. The light/medium/heavy bodies use
crate/sandbag/wall catalog properties with the same low barrier dimensions.
The footprint is a controlled frontage, not a claim about every ordinary prop.
[Baseline position and splash](effects-baseline.json),
[tuned rifle positions](effects-final-rifle.json) and
[tuned HMG positions](effects-final-hmg.json) retain shots, physical hits,
prop impacts, harm, initial eligibility and actual range.

## Explosive fragments

Grenades are aimed at ground five metres from the receiving soldier;
samples with direct body hits are rejected. Generic exterior tiers add exactly
0% fragment reduction in matched geometry. Building shelter measured
65.8% (62.9–68.8%) reduction over 128 fresh paired seeds, 30 seconds, 6144
launched rounds per arm. This is the existing shelter fragment probability,
applied once; it is separate from the roughly 70% direct-fire target.
[Expanded fragment evidence](splash-confirm.json).

## Whole battles and rendered evidence

The quick village check preserves 2/3 flank captures and 0/3 ambush captures
from the completed-main quick baseline; neither run loses a tank. Flank blue
cost lost totals 1197.5 → 1147.5 across three seeds.

The full comparison pairs the same ten seeds per plan for up to 900 seconds.
Only the three exterior scatter factors change between these cohorts.

| Plan | Captures, baseline → tuned | Blue cost lost | Tanks lost | Rejoined |
| --- | --- | --- | --- | --- |
| frontal push | 5/10 → 4/10 | 800 → 600 | 4 → 3 | 0 → 0 |
| flank | 8/10 → 8/10 | 3692.5 → 3455 | 0 → 0 | 9 → 8 |
| ambush, 0.75 s | 0/10 → 0/10 | 200 → 200 | 1 → 1 | 0 → 0 |
| ambush, 3 s | 0/10 → 0/10 | 600 → 600 | 3 → 3 | 0 → 0 |
| crossfire | 0/10 → 0/10 | 200 → 200 | 1 → 1 | 0 → 0 |

Neither cohort rejects commands. Twenty-nine paired battles retain identical
digests; twenty-one change, as expected from changed launch spread. Flank remains
the strongest tested plan. The frontal plan loses one capture despite lower
aggregate casualties; tuning does not promise equal outcomes in every seed.
[Full baseline](village-baseline.json) and [full tuned results](village-final.json)
retain all fifty rows, including capture times and final digests.

[Visual evidence](visual/README.md) compares production-renderer frames and
records the final unprimed critique, including unchanged dust/occlusion debt.
Images validate trajectory plausibility, not protection percentages.
