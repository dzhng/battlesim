# C04: importer

**Depends on:** C01, C03 (contract); the reader can start after G0. **Kind:** slice.

## Question
Does real NYC data become a byte-stable `MapDefinition` through a source-agnostic intermediate?

## Contract it unlocks
- `crates/city-import`, a Rust bin plus lib depending only on `contract` (S-import).
- Pipeline: `read_nyc(source_dir) → CityPlan` (Q1's intermediate: footprints with height and year, roadbed and sidewalk polygons, origin), then `decompose(ring, tol) → parts + exposed edges` (S5's data), then `lower(plan, rect) → MapDefinition + SOURCES.json`.
- EPSG:2263 → local metres (US-survey-foot scale plus an offset; no proj dependency).
- **Floors come from height only** (S-floors): `floors = max(1, 1 + round((h − 4.2)/3.2))`, ground floor 4.2 m, upper floors `(h − 4.2)/(floors − 1)`.
- Archetype is `nyc` for every building in this spec. Seed = hash(BIN).
- Output `fixtures/maps/<id>/{map.json, SOURCES.json}`, loaded through C09's fetch-by-id path.

## API seam
`crates/city-import`, `fixtures/maps/`.

## What the human can run or see
`cargo run -p city-import -- <src> <rect> --id <id>`; a top-down PNG of footprints over parts with exposed edges highlighted; the parts histogram.

## Verification
- A golden byte-stable test on the **committed clipped excerpt** (S-excerpt), attributed in its SOURCES.json.
- Property tests: area error ≤ tolerance, parts ≤ N, no overlap beyond ε, deterministic ordering.
- SOURCES.json allow-list test (Q6).
- Village digests unchanged.

## Delegated to the implementer
Decomposition algorithm, tolerance and N (bounded by S1's histogram); the final crop among Q2's districts; excerpt size. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village digests.
