# S1: sim scale

**Depends on:** none (start here). **Kind:** slice.

## Question
At 500 / 1,500 / 3,000 buildings, which sim and publication costs break, and which fog cell (2, 4 or 8 m) is right on 18 m side streets?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write the verdict to `specs/city-maps/spikes/S1.md` (numbers table + one verdict row per question + kill check).

## API seam
Build a crude box city in **today's** schema with a script: one minimum-area box per building, plus a ≤4-box variant, and a synthetic city at the three counts. Also hand-convert one real crop from Q2's candidates; it is the shared input for S3–S5. Save it to `throwaway/city-spike/crop.json`.

## What the human can run or see
The verdict tables. `movement_shots` GIFs of a squad crossing a block and of sight down a side street at each cell. A parts-per-footprint histogram.

## Verification
- Measure at each count × fog cell 2/4/8 m:
  - `WorldGeometry::new` ms;
  - `NavGrid::build` ms and instructions per side (`navigation.rs:181`), and rebuilds per minute in a scripted fight;
  - `OcclusionGrid::refresh` cost (`visibility.rs:38-60`), including a "shove storm" of 20 cars pushed;
  - sweep instructions per eye;
  - step p50/p95/p99 and instructions per 5 min (endurance style);
  - publication bytes per tick, **split into fog words, known props and the rest** (`battle.rs:1850-1890`, `publication.rs:51, :730`), including known-props bytes after 200 collapses;
  - `map.json` bytes and parse ms, native and wasm; RSS.
- Measure the share of 18 m street cells wrongly blocked by cells straddling the building line at each fog cell size, against exact `sight_clear`.
- **Kill:** a cost that no known local fix (incremental grid, index, deltas) brings to step p95 ≤16 ms. Then shrink the map (1.0 km) or its density, and reslice at G0.
- **Proposes:** the sim step target and the fog cell for G0.

## Delegated to the implementer
The stress-generator shape; which candidate crop to hand-convert; the fight script. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.

## Feedback that would change this slice
If the user wants a different district, re-run on it.
