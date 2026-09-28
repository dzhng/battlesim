# C60: map catalogue data

**Depends on:** C09, SG6. **Kind:** slice.

## Question
Does every map live in one catalogue folder, validated and loaded by id (Q-G9)?

## Contract it unlocks
- C09 already put every map at `fixtures/maps/<id>/map.json`. This slice adds `SOURCES.json`, `meta.json` and `encounters/<name>.json` to each of the 12 (endurance as `category: benchmark`), moving each lab's scenario into an encounter.
- `meta.json` holds exactly Q-G9's fields.
- **Owner: a TypeScript module** (`web/src/maps/catalogue.ts`: `MapMeta`, a validator, `listMaps(filter)`). Only JavaScript reads `meta.json`; the sim reads `map.json` by id through C09's loader, and Rust tests get `maps::load(id)`.
- The browser lists maps with a Vite glob, and `scene.mjs` reads the directory. **No committed generated index** (it would bring back the merge conflicts).
- No second map location or loader is introduced.

## API seam
`fixtures/maps/`, `web/src/maps/catalogue.ts`, the C09 loader, Rust `maps::load`.

## What the human can run or see
`ls fixtures/maps`; every lab and the village play unchanged.

## Verification
- A schema test: every folder has a valid `meta.json`, `SOURCES.json` is present, every encounter parses, ids are unique.
- **Every digest and replay is identical** (0 battles); only the `config_digest` headers move, named.
- `vite build` JS gzip does not grow.

## Delegated to the implementer
Each lab's id; the `character` and `tags` vocabulary. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every scene id and sim test; village digests.
