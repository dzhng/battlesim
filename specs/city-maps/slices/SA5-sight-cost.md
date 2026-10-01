# SA5: sight and fog cost at full extent

**Depends on:** SA4, the grid-update pass in [SA2](SA2-counted-route-integration.md). **Kind:** slice, performance only.

## Question
Does working out what each side sees stay cheap on a 10 km map with a hundred units a side?

## Contract it unlocks
The cost of a side's sight and fog update is bounded by what its eyes can reach, not by the map or by every occluder on it. No rule changes: what is seen, when, and by whom is exactly as today.

## API seam
`crates/sim/src/visibility.rs`, the height field's page lookup in `world/terrain.rs`, and whatever index sight needs over occluding bodies. Nothing in the contract, the observation or the publication layout.

## What is known
- Each rebuilt sight map tests every occluder on every ray: about 1 ms with nine eyes on Metro Large, unmeasured with a hundred units a side ([S3](../spikes/S3.md)).
- About 40% of a village battle's CPU samples are hashing height-field pages in the fog sweep (grid-update pass, SA2 Outcome).

## Verification
- Digests identical on the village (`village_report -- --quick`), endurance and every `city_report` probe; replay parity.
- Cost in instructions retired, before and after, on `endurance_report` and on `city_report` for the 6 km and 10 km generated towns ([S1](../spikes/S1.md) names the maps and the tool) with at least a hundred units a side: extend `city_report` if it needs a force size.
- No tick over 33 ms from sight on those runs.

## Delegated to the implementer
The index structure and its cell size, recorded in `../choices.md` under this slice.

## Must stay green
Foliage and fog agreement; every existing sensing and fog test.

## Feedback that would change this slice
A rule that has to change to make sight affordable is a mechanic change: stop and run tweak-mechanics, and name the digest change.


## Outcome (SA5 implementation; integration gates pending)

Sight keeps the same rays, sampled terrain triangles, foliage spans, visibility
union and learning order. Repeated lookups now address lossless sparse page
directories; occluder tops rasterize only visited four-by-four fog-cell tiles
and carry the world's obstacle revision. World bodies and canopy-expanded
forest spans reuse their existing buckets. Remembered bodies have a separate
per-side index whose owner updates it when retaining, replacing or forgetting
a snapshot. Seeing one part still reveals the whole building: live parts and
historical snapshots are expanded by immutable building identity, including
parts beyond the eye's reach. These are internal seams; commands, observations,
publication and digest layout are unchanged.

The discovery corrected the initial premise above: physical line-of-sight
already used the world's body index. The expensive fog work was whole-grid
occluder refresh, repeated terrain/foliage lookups and repeated union work.
The all-cell ray preflight skips only rays that cannot add a visibility bit.
It checks the tail first, so blocked rays with unseen ends reject that proof
before walking their already-visible prefixes.

Measured on macOS aarch64 against e5cf680b with the force-size harness only,
using seed-4 generated river towns from S1: Mixed Small (6 km) and Metro Large
(10 km), 100 units per side. The 30-second crossing probes use width shares
0.03 and 0.018 respectively. The original at-most-six-unit probe is unchanged.

| Probe | Before instructions G | Winner instructions G | Digest | Quiet winner max wall / process CPU ms |
|---|---:|---:|---|---:|
| 6 km crossing | 354.4 | 153.9 | `63211f98d8877be6` | 22.8 / 21.89 |
| 10 km crossing | 463.7 | 142.0 | `487cd21bab9c7462` | 21.6 / 19.39 |
| Endurance, 5 battle minutes | 8581.9 | 1439.4 | `ce696ab655fff2d4` | 222 / 55.36 |

The quiet city runs have zero ticks over 33 ms. Final endurance retired 83.2%
fewer instructions, with the exact original digest, but retained 237 whole-step
wall ticks over 33 ms (p95 27.8 ms, p99 47.0 ms). Its maximum process CPU tick also
exceeded 33 ms, so a separate temporary measurement brackets the actual
sensing and due fog sweeps, including their learning work. The final tail-first
preflight passes all 9,000 endurance ticks: maximum combined CPU 22.477 ms,
wall 29.766 ms; sensing alone 1.699 ms and fog alone 21.921 ms. Zero sight
updates exceed 33 ms in either measure. A prior forward-preflight trial
failed (36.330 ms sight CPU, 13 slow sight ticks); this prompted the measured
ordering improvement rather than a relaxed gate. The temporary patch and
per-tick logs are saved in `throwaway/sa5/sight-tail-attribution*`; its
report helper reads macOS process counters. Instrumentation is removed from
production, and the final source's uninstrumented instruction counts are
confirmed separately.
Concurrent build load produced slower city results; the quiet repeats above
are kept alongside those failures, not substituted for them silently.

Final peak RSS was 216 MiB for 6 km and 501 MiB for 10 km; the
final endurance run was 259 MiB. Flat directories allocate
only their page pointers and populated sample pages. The follow-up adds two
32 m bucket directories (about 4.5 MiB at 10 km), within the measured final RSS. Earlier controlled corpse/wreck/tree churn
probes also retained
exact digests (`4237803389d2a187`, `b141761c35e11e4e`) with zero slow ticks.
Six final village quick battles retained all original digests (1149 G
instructions, serial trials). The final follow-up also passes both city digests.

Map JSON SHA-256 hashes: Mixed Small
`59c0971dade907af209c4b9c57185f104abc2427ad119dbd54b85b680445c606`,
Metro Large `f8079f26cf0b986f33b559a59063d0e58907a004d0e7e026f40a24762f9fff56`.
Paired report evidence, temporary attribution patches and tick logs are
preserved in the main checkout under `throwaway/city-maps-sa5/`. The report
files name their source arm; `*-final-winner` is the uninstrumented final
source through 21ec37fb. Earlier failed or loaded trials remain alongside it.

Focused regressions falsified cache invalidation, incomplete union preflight,
canopy bucket expansion, live aggregate identity lookup, stale remembered
index entries and far historical snapshot propagation. The standing snapshot
map is read-only outside its owner, so callers cannot bypass index updates.
Terrain, forest,
geometry, buildings, sensing and sight tests passed; the earlier whole sim
suite passed 447 tests. Independent read-only reviews by the sim coordinator
and body agent found no defect in the preserved winner; the coordinator
reviewed the remembered-history follow-up, requested its map encapsulation,
and reviewed the final exact tail-first proof. The requested local
Codex CLI review could not run because its configured model is unsupported
for this account; no model override was made. Shared `check` and `verify`
belong to the coordinator's integration closeout.
