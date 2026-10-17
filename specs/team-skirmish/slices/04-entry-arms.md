# Slice 04: mapgen entry arms and admission

**Decisions:** D8′, D18 (size floor), D36, D50, D52, D53. **Inputs:** slice 00's verdict and numbers.

## Contract it unlocks

A generation request with 1–3 entries per side yields legal, fair, spaced entries.

## Seam

- **Generation request** (`crates/contract/src/generation.rs`) gains the entry count, which is the same on both sides (D52). Bump `GENERATOR_VERSION`.
- **`crates/mapgen/src/skirmish.rs`** lays the left and right arms in a dedicated pass with its own `"entries"` stream (D36):
  - each arm starts inside a band of the edge;
  - each arm is joined to the network;
  - each arm avoids river mouths.
- **`reserve`** finds each arm by its known exit and classifies centre, left and right by position. It no longer takes the first stroke at `y == edge`.
- **`SkirmishSites.entries`** becomes an ordered list. `encounter.rs` validation checks the counts against `entry_count`.
- **`sim::map_analysis::admit_skirmish`.**
  - Every entry must stand, and must route to every objective, for each representative unit.
  - Fairness compares each side's best travel time over its entries, within 15%.
  - The size floor is enforced from the larger side's player count: 1 → S, 2 → M, 3–4 → L, 5 or more → XL.

## What the human can run or see

The existing map workbench and inspect SVG for a 2v3 and a 5v5 request, with the entries marked.

## Red tests first

- 1, 2 and 3 entries per side on every size and map type give finite, legal entries that are at least 150 m apart and clear of river mouths. These are hard admission tests (D53): the slice 00 rough pass failed both on L and XL.
- Admission of 2v3, 3v3 and 5v5 (equal entries, D52) is re-measured on the corpus with the real arm pass, and recorded in `choices.md` against slice 00's 1v1 rate.
- A 1-entry request reproduces today's map hash on the corpus.
- A 2v3 map is admitted or refused under the generalized fairness rule.
- A request below the size floor is refused.

## Delegated

Arm band widths and join strategy, within slice 00's evidence. Internal helper names.

## Stays green

1-entry map hashes. The 1v1 corpus admission rate. All simulation digests: entries are not consumed until slice 05.

## Feedback that would change it

The user judging entry spacing on the slice 08 picture.
