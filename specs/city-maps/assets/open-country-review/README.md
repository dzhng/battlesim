# Open-country pickup review

The 2026-10-02 user references reaffirm M24/M25: fields stay useful and largely open, with small physical features interrupting some sight bearings and making the land between towns interesting. The [reference source record](../reference/broken-arrow/SOURCES.md#open-country-reference-target) owns the visual target; [C54](../../slices/C54-generation-gate.md) owns final opening-view acceptance.

## Current implementation and evidence

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

## What remains to judge

This evidence supports the physical sight rule for the sampled map and starts. It does not prove every position or seed, the silhouette of combined published fog, or the visual variety of the final landscape. Final C54 must show actual opening fog, a tactical field view and an overview against both new references. The ground lane already records rare hedgerows and fields that can read as a flat outlined carpet; these are relevant to that visual review. Do not increase density by assumption or treat low cover as a tall sight blocker.
