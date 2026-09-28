# C60: map catalogue

**Depends on:** C09. **Kind:** slice.

## Question
Are all maps (playable, lab, benchmark, test) catalogued in one typed structure that every surface lists from (Q-G9)?

## Contract it unlocks
- Every map lives in `fixtures/maps/<id>/` with `map.json`, `SOURCES.json`, `meta.json` and `encounters/<name>.json`.
- `meta.json`:
  - `category`: playable | lab | benchmark | test;
  - `status`: draft | released | retired;
  - `label`, `character`, `biome`, `size_m`, `tags`;
  - `source`: imported | generated | authored;
  - `seed`, `encounters`, `benchmarks`.
- A schema test enforces it.
- The main menu lists `playable & released`, the lab app lists `lab`, the benchmark lists `benchmark` plus playable maps' benchmarks, and the scene runner lists everything grouped by category. **No hand-kept lists remain.**
- The 11 `fixtures/*-lab.json` files and the 25 `apps/battle-lab/src/fixtures.json` entries migrate in one cutover. Scene ids stay stable (L-G9).

## API seam
`fixtures/maps/`, a catalogue loader shared by `web/` and `apps/battle-lab`, `web/scene.mjs`, `MainMenu.tsx`.

## What the human can run or see
The menu, lab index and `bun run --cwd web scene -- --list` all grouped by category.

## Verification
- Schema test: every map has a valid `meta.json`; ids are unique; referenced encounters and benchmarks exist.
- Every lab scene still runs under its old id.
- Village and lab digests unchanged.

## Delegated to the implementer
Loader caching; tag vocabulary beyond the listed fields. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every existing scene and route; digests.

## Feedback that would change this slice
If hud-chrome's typed router lands first, the menu listing adapts to it; neither blocks.
