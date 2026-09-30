# C60: map catalogue data

**Depends on:** C09, SG6. **Kind:** slice.

## Question
Does every map live in one catalogue folder, validated and loaded by id (Q-G9)?

## Contract it unlocks
- C09 already put every map at `fixtures/maps/<id>/map.json`. This slice adds `SOURCES.json`, `meta.json` and `encounters/<name>.json` to every saved map found by the migration inventory (endurance as `category: benchmark`), moving each lab's scenario into an encounter.
- `meta.json` holds Q-G9's saved-map fields; generated entries derive size/character/seed from canonical provenance and cross-check the compiled map hash/bounds. C55 transient maps use equivalent runtime/replay identity through the same resolver and need no persistent folder.
- **Owner: a TypeScript module** (`web/src/maps/catalogue.ts`: `MapMeta`, a validator, `listMaps(filter)`). Only JavaScript reads `meta.json`; the sim reads `map.json` by id through C09's loader, and Rust tests get `maps::load(id)`.
- The browser lists maps with a Vite glob, and `scene.mjs` reads the directory. **No committed generated index** (it would bring back the merge conflicts).
- No second map location or loader is introduced.

## API seam
`fixtures/maps/`, `web/src/maps/catalogue.ts`, the C09 loader, Rust `maps::load`.

## What the human can run or see
`ls fixtures/maps`; every lab and the village play unchanged.

## Verification
- A schema test: every folder has a valid `meta.json`, `SOURCES.json` is present, every encounter parses, ids are unique.
- **Every digest and replay is identical for this metadata-only cutover** (0 battles); only config identity moves, named. C56's later geometry addition explicitly updates affected map/digest/cost baselines.
- `vite build` JS gzip does not grow.

## Delegated to the implementer
Each lab's id; the `character` and `tags` vocabulary. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every scene id and sim test; village digests.

## Feedback that would change this slice
A route/map identity that cannot be represented without a duplicate list reopens catalogue ownership/schema.
