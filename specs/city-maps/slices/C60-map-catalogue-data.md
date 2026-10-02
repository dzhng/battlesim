# C60: map catalogue data

**Depends on:** C09, SG6. **Kind:** slice.

## Question
Does every map live in one catalogue folder, validated and loaded by id (Q-G9)?

## Contract it unlocks
- C09 already put every map at `fixtures/maps/<id>/map.json`. This slice adds `SOURCES.json`, `meta.json` and `encounters/<name>.json` to every saved map found by the cutover inventory (endurance as `category: benchmark`), moving each lab's scenario into an encounter.
- `meta.json` holds Q-G9's saved-map fields; generated entries derive size/character/seed from canonical provenance and cross-check the compiled map hash/bounds. C55 transient maps use equivalent runtime/replay identity through the same resolver and need no persistent folder.
- **Owner: a TypeScript module** (`web/src/maps/catalogue.ts`: `MapMeta`, a validator, `listMaps(filter)`). Only JavaScript reads `meta.json`; the sim reads `map.json` by id through C09's loader, and Rust tests get `maps::load(id)`.
- The browser lists maps with a Vite glob, and `scene.mjs` reads the directory. **No committed generated index** (it would bring back the merge conflicts).
- No second map location or loader is introduced.

## API seam
`fixtures/maps/`, `web/src/maps/catalogue.ts`, the C09 loader, Rust `maps::load`.

## Current systems checkpoint

This section records the slice before the cutover. It has since landed: the
[Outcome](#outcome) says what the catalogue is now.

[C09's core checkpoint](C09-fetched-maps.md#current-systems-checkpoint) provides
the shared source envelope and pure resolution contract. Catalogue metadata,
encounter extraction and saved folders remain unimplemented. This slice's
contract above describes the intended cutover, not an accepted production
catalogue. The original producer inventory is pinned in C09's proof evidence.

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

## Outcome

Every saved map's folder holds `map.json`, `SOURCES.json`, `meta.json` and its encounters, and two tests hold every folder to the schema. `map.json` is the compact saved form since [C58](C58-offline-encounter.md#outcome): a building is its template id, frame and ids, and `SOURCES.json` names the library it is materialized from. The folder layout and how to add a map are in the [fixtures guide](../../../fixtures/README.md#saved-maps); the loaders are in the [C09 outcome](C09-fetched-maps.md#outcome); the decisions the spec left open are in the [choices ledger](../choices.md#c09c60-saved-map-cutover).

**`meta.json`** (`web/src/maps/catalogue.ts`: `MapMeta`, `validateMapMeta`, `checkMapFolder`, `listMaps(filter)`). The folder's name is the id; the file has exactly these fields:

| Field | Value |
|---|---|
| `category` | `playable`, `lab`, `benchmark` or `test` |
| `status` | `draft`, `released` or `retired` |
| `label` | The name a listing shows |
| `character` | `open`, `mixed` or `metro` (a generated map's type), `village`, or `arena` (a lab's or benchmark's ground) |
| `biome` | A `fixtures/biomes/<biome>.json` |
| `size_m` | `[width, height]`: the map's own `size` |
| `tags` | The physical features the map has, from `relief`, `river`, `road`, `bridge`, `forest`, `prop`, `building`, in that order |
| `source` | `imported`, `generated` or `authored` |
| `seed` | A generated map's seed as decimal text; `null` otherwise |
| `encounters` | The names under `encounters/`, in order |
| `benchmarks` | The benchmark routes that run on the map |

A field that is missing, unknown or out of its vocabulary is refused, naming the file and the field. `checkMapFolder` then compares what the listing says with what the folder holds: `size_m` with the map, `tags` with the map's features, `encounters` with the directory, and for a generated map `source` and `seed` with the generation identity in `SOURCES.json`. `listMaps(filter)` reads the folders with a Vite glob and filters by category, status and tag. There is no index file.

**Encounters** (`contract::scenario::EncounterDefinition`). An encounter is the half of a scenario that is not the map or the rules: `units`, and optionally `events`, `scripts`, `opponent` and `encounter` (the completion rule). An unknown top-level field is refused. Each lab's inline scenario became one file, named after the route that plays it; the ambush lab's three variants are three files, and the village's lean firefight is the village map's `lean`. A lab's battle seed, and the rule values a lab pins for its experiment, stay with its route.

**The tests.** `cargo test -p sim --test sim maps::` resolves every folder to the identity its sources pin, builds a battle from every encounter under the shipped rules, and checks the refusals against a scratch catalogue. `web/tests/mapCatalogue.test.ts` validates every `meta.json` against its map, resolves every folder through the Wasm resolver to the same pinned identity, checks that the definition handed on is the resolver's own text and that an authored map survives JavaScript's printing, and checks the validator's and the adapter's refusals. Both read the directory, so a new folder is covered without touching either.

**Nothing moved in play.** The proof is in the [C09 outcome](C09-fetched-maps.md#outcome): every digest is identical and no config identity changed. This slice's Verification expected config identity to move; it did not, because the rules' digest covers the typed rules, which never held the map.
