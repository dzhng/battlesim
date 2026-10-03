# Open-country pickup review

The 2026-10-02 user references and clarification reaffirm M24/M25 at every playable location on every generated map. Fields stay useful and largely open, with small physical features interrupting some circular sight bearings and making the land between towns interesting. The [reference source record](../reference/broken-arrow/SOURCES.md#open-country-reference-target) owns the visual target; [C54](../../slices/C54-generation-gate.md) owns integrated acceptance.

## Historical sampled evidence

`mapgen::open_country` is in the normal generator pipeline. It places rural homes from the same template catalogue as towns, short broken tree lines, copses, single trees and low field cover. It fills otherwise bare areas while protecting the main settlement's long approaches. [The generator README](../../../../crates/mapgen/README.md#open-country-open_country) explains its owners and physical contracts. C86 draws generated tree lines with a shrub layer; its Outcome keeps remaining visual evidence separate.

A focused native `sight_report` at pickup compared the same Mixed Small seed `55012999855851041` before and after the furnishing pass, sampling open ground every 300 m. Inputs were `layout-11` / `layout-presets-10`, physical catalogue `6b0a5e8b…`; the working change affected move admission only. The report uses simulation body occlusion and foliage queries, with 472 bearings at the infantry eye's 600 m range. It is a furnishing comparison, not the final integrated street-furniture/release matrix.

| Observation | Bare | Furnished |
| --- | ---: | ---: |
| Open-ground samples | 307 | 306 |
| Samples with an unbroken sight circle | 31 | 0 |
| Median bearings reaching full range | 76% | 72% |
| Open-ground samples with less than half the view open | 10.7% | 13.7% |
| Rural buildings added | 0 | 19 |
| Tree-line length added | 0 m | 701 m |
| Copses added | 0 | 24 |
| Single-tree/clump plots added | 0 | 47 |
| Low cover bodies added | 0 | 103 |

The furnished arm reports no unbroken circle for any unit in either planned starting column. Raw report and log are ignored scratch at `throwaway/open-country-pickup-sight.jsonl` and `.log`; the command and output schema live in `crates/mapgen/examples/sight_report.rs`.

## Actual opening-view checkpoint

Two paused production battles, Mixed Small seed `55012999855851041` and Open Small seed `1`, were captured at tick 2 on layout-12 / presets-11. Normal opening, tactical field, full start overview and oblique overview views retain the actual nine published own eyes and enabled fog. Matching per-case battle digests establish that changing the camera did not change the battle. Full screenshots, fog masks and native crops are retained in ignored `throwaway/c54-opening-candidate/` in the main checkout; its manifest and scripts own exact poses and capture inputs.

An unprimed review of all full frames and crops found genuine forest-aligned fog cuts in both full overviews. Close opening and field masks were entirely revealed within their viewport, so they cannot establish a full-range boundary. The oblique views show short separate tree lines and copses among mostly open fields. Sparse close fields, uncertain rural-house readability and weak overview labels remain open; this is a narrow fog/feature checkpoint, not landscape acceptance.

## Global coverage is still red

On saved layout-12 Market Town seed `1`, a jeep can stand at `[1150, 4450]` and see its entire 450 m circle: all 354 normal bearings and 3,540 finer bearings reach full range. The old 600 m rifle query at that same point has only 88.14% open bearings. Nearby physical bodies and tree crowns are outside the jeep's range, so this is a real coverage gap rather than a coarse angular sample missing a thin obstruction. The exact native probe and red regression are retained with the ongoing global coverage work.

The generator's current coverage owner marks cell centres near building anchors and authored wood edges; failed attempts may leave bare cells without a final refusal. Construction must account for physical blockers, actual circular observer ranges and the entire cell area, including unbuilt town space and edges. Finite samples remain useful counterexample searches, never the continuous/all-seed proof.

### Unintegrated coverage checkpoint

The working branch `codex/city-ground-fix` replaces proximity heuristics with a continuous whole-cell certificate using shared physical forest placement and the native fog sampling geometry. The exact jeep counterexample and between-cell/town-space probes have passed in earlier candidate checkpoints. Those results do not admit the final candidate.

Independent source review found that a witness near an edge could cast its isolated far fog cell outside the map, leaving all in-bounds fog clear. The candidate now reserves an in-bounds target over every position in a clipped cell, using the same predicate for lookup and new placement. A tiny edited tree-line width also exposed a potentially enormous allocation before charging; its forecast/refusal is added before allocation. These corrections retain their red controls. Fresh compilation after restart passes the actual Mixed Small seed-1 between-cell/edge/town-space regression, the four coverage proof guards and feature fairness. The earlier clipped-cell refusal is historical. Final independent source review, current main integration, full seed admission and refreshed geometry/access/visual identities are still required before acceptance.

The computer restart preserved source and ignored evidence in the existing worktree; old processes did not survive. The generator's README owns the proof's current assumptions and limits. No candidate geometry is integrated or accepted yet.

## What remains to judge

The historical report supports only its sampled map and standardized starts. The actual openings establish visible forest cuts for two candidate seeds. Neither admits M24/M25 globally or proves final visual variety. After the coverage fix, C54 must judge the new physical geometry and actual published fog against both references, including closer rural landmarks. The ground lane already records rare hedgerows and fields that can read as a flat outlined carpet. Do not increase density by assumption or treat low cover as a tall sight blocker.
