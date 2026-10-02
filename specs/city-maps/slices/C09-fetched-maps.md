# C09: common map acquisition and resolution

**Depends on:** C01; SG6's dry run. **Kind:** slice.

## Question
Can saved and eventual generated sources feed one compiled-map initialization contract without bundling maps in JavaScript?

## Contract it unlocks
`MapSource` and `ResolvedMap { definition, identity }` establish one resolution seam. Saved scenarios initially use the catalogue variant; C55 implements the generated-request variant through the same owner. Every saved map moves to `fixtures/maps/<id>/map.json` in this cutover: village, lab fixtures and endurance, with the actual inventory checked rather than a stale count. Rust and browser resolution load the same contract; the sim receives only the resolved definition.

Static consumers of village map data read the resolved public geometry instead of importing it from game.json. C60 adds saved metadata; runtime sources later keep equivalent identity in memory/replay and require no catalogue folder. No map is bundled in the JS entry point; no second loader or alias remains.

## API seam
`contract::scenario` map source/identity, browser map resolver and wasm preparation, native map resolution. C55 adds generation acquisition here without altering downstream battle/renderer consumers.

## Approved systems contract

The physical contract owns `MapSource::Catalogue { id }` and
`ResolvedMap { definition, identity }`. Native filesystem and browser HTTP
adapters obtain the same documents and call one pure resolver. Resolution
constructs no world; preparation can reuse the admitted definition without
requiring an additional main-thread `WorldView`. The existing main-thread
static-world construction is C33's separate cutover.

`SOURCES.json` records an authored content identity or the existing
`GenerationIdentity`, input receipts, and an explicit selection
from the shared physical template library. The selected catalogue is rebuilt
by the existing catalogue owner: endurance's one-template selection preserves
its original hash without a second template file. The resolver checks typed
map content, catalogue identity and descriptor materialization at each saved
frame, including category/family and complete physical geometry. Missing or
contradictory documents are refusals; source receipts do not certify art fit.

Acquisition identity stays beside `ScenarioDefinition`. Moving files and
adding metadata alone therefore preserves existing battle and replay hashes.
C55 will add the actual generated request contract; this pass introduces no
placeholder generator schema. Authored and generated are the current identity
tags; real-data import remains outside this scope.

Asset tools feed their existing file reads through the same WASM resolver,
initialized once per process. A built resolver ABI is an explicit prerequisite,
with an actionable refusal when absent. The scene-assets package accepts
resolved values and owns neither filesystem access nor WASM initialization.

Admission limits cover authored parts and emitted bay positions before further
materialization. They do not certify all input parsing, terrain, navigation,
trees, startup or complete-world memory.

Header validity is shared with the compiler; the compiler's playable architecture
envelope remains caller policy. Shape admission must use C72's shared contract
owner rather than a resolver copy of simulation polygon rules. Rules-specific
body capabilities and tree/raster allocation remain preparation obligations in
C33. Content/catalogue agreement alone is not complete physical/world admission.
Repository receipts require normalized paths, nonempty revisions and canonical
content hashes. Supplied preparation/CLI inputs record a nonempty human label
and the exact input-byte hash, without guessing Git history. Resolving current
documents does not authenticate an unavailable historical revision or generator
recipe. The compiler's saved `SOURCES.json` uses this same envelope; its stdout
outcome and existing generation identity are unchanged.

Catalogue directory IDs use lowercase ASCII letters/digits with hyphens or
underscores, starting with a letter/digit. They identify storage independently
of route IDs and physical hashes, and reject paths before source IO.

## Producer boundary

The source inventory is checked against actual route and scene registries.
Saved physical inputs are village, the lab files and endurance's authored base.
Endurance's existing seeded wreck augmentation retains its draw order and IDs;
fallen soldiers remain encounter composition. Authority and ballistics share
geometry, contacts shares sensors, and village-derived routes share village.
Their route identities are unchanged.

Foundation's Gaussian patch and the workbench's measured negative-coordinate
ground remain explicit C56 pending tool producers. The current physical map
schema cannot reproduce their frames exactly; this core must not claim complete
producer coverage or manufacture replacement maps. Panels and sound have no
physical map. Analytic unit/resource probes remain labelled API inputs.

## Current systems checkpoint

This section records the reviewed core as it stood before any caller moved. The
cutover has since landed: the [Outcome](#outcome) says what the seam is now.

The pure source/content/catalogue resolver and the compiler's common saved-source
envelope are implemented. Native contract refusals and actual compiler file
write-to-resolver behavior are proven; the original compiler stdout corpus remains
unchanged. This is the reviewed core checkpoint, not completed C09/C60 acquisition.

Next: implement thin native filesystem and browser HTTP/WASM adapters, TS catalogue
metadata and actual source/encounter cutover. Callers supply the narrow authored-part
and bay-position allowance explicitly. Consume C72 shared shape admission before
claiming complete physical validation. Preserve each original producer's prepared
f64 values and scripted encounter behavior, then prove actual factory/replay/scene
parity. Asset tools require the declared built resolver ABI; C33 still owns one
world preparation and its complete resource/rules admission.

No saved map moved, metadata was added to no production map, and no production
adapter or generated-request placeholder ships in this checkpoint. The original
registry inventory and source-byte freeze remain the cutover starting evidence.
The earlier publication80 corpus covers geometry-lab via Value-to-typed setup,
not every shipping Village ordinary-prop bit through factory serialization and
Battle decoding; keep that numeric boundary gap separate from C01's corrected
ScriptedBlue building-discovery omission.

The immutable [source inventory and core proof receipts](../assets/map-acquisition/README.md)
pin the stopping point and the consumer cutover still required.

## What the human can run or see
Original maps play unchanged during this location/schema-only cutover; build size before/after and a resolved map identity report. Surroundings are added later in C56.

## Verification
- Outcome digest/replay parity for this cutover; name config-identity changes.
- Every saved map resolves once through the common loader; scene IDs stay stable and JS gzip does not grow.
- Missing/invalid sources return useful diagnostics; no fallback seed/map.
- Native/browser consumers agree on the resolved definition/identity.

## Delegated to the implementer
Caching and internal resolver names. A second map format/source-specific battle path is not delegated.

## Must stay green
Original battle contracts and one compiled-map authority.

## Feedback that would change this slice
A source consumer still bypassing the resolver blocks the cutover until it is moved onto the resolver.

## Outcome

Every saved map is a folder of [`fixtures/maps/`](../../../fixtures/README.md#saved-maps), read by id through one resolver. The decisions the spec left open are in the [choices ledger](../choices.md#c09c60-saved-map-cutover).

**The seam.**

- **Resolver** (`contract::maps::resolve`, unchanged in what it admits). `MapAdmission::CATALOGUE` is the one allowance both catalogue adapters pass. (Since [C58](C58-offline-encounter.md#outcome): `map.json` is the compact saved form and the resolver materializes its buildings, `SOURCES.json` names the map's library, and the allowance is the generator's.) `ResolveCode` gains the adapters' refusals: `invalid_id`, `missing_document` and `invalid_encounter`. An identity mismatch now states the hash the resolver computed. `ResolveOutcome` is the answer across a JSON boundary: `{ status: "ok", result: { definition, identity } }` or `{ status: "error", error: { code, location, message } }`.
- **Native adapter** (`sim::maps`). `load(id) -> Result<ResolvedMap, ResolveError>` and `encounter(id, name) -> Result<EncounterDefinition, ResolveError>` read the shipped catalogue; `Catalogue { maps, libraries }` is the same over any directory, with `ids()` and `encounters(id)`. An address is checked before any read, and a refusal's location is prefixed with the map's folder (`geometry/SOURCES.json.identity.map_hash`).
- **Wasm** (`resolve_saved_map(map_json, sources_json, library_json) -> String`) answers a `ResolveOutcome` under the same allowance. `library_json` is the library the sources name.
- **JavaScript** (`web/src/maps/`). `resolve.ts` holds the types (`MapDefinition`, `MapIdentity`, `ResolvedMap`, `Encounter`), `MapResolveError { code, location }` and `resolveSavedMap`. `browser.ts` is the HTTP adapter: `loadMap(id)` and `loadEncounter(id, name)` fetch the documents, each its own served file found by a Vite glob. `node.ts` is the same pair over files and the built Wasm, for tests, scenes and tools.
- **Factories.** `sim::fixtures::village()` is the rules, the unit catalog and the village's resolved map under `map`: the village factory's input, natively and in the browser. `sim::endurance::scenario(field, fixture, seed, late)` takes its saved field; the Wasm `endurance_scenario(map_json, fixture_json, seed, late)` follows it. `village_scenario(fixture_json, variant)` is unchanged, and its fixture now carries the resolved map.

**What moved.** The twelve lab maps moved byte for byte. The village's map was lifted out of `village.json` text for text. The endurance field is the map its code used to build, saved as that code printed it; its late wrecks are still added in code, in the same draw order and with the same ids. `village.json` now holds rules only. Fourteen folders serve the 29 registered routes: `geometry` serves geometry, authority and ballistics, `sensors` serves sensors and contacts, and `village` serves the village, replay, watch, lean, benchmark, fog and fog-look routes and the village mode of ground.

**Consumers.** No script imports a map. Lab routes get theirs through `SavedMap` and `SavedEncounter` (`apps/battle-lab/src/savedMaps.tsx`), the village and endurance routes through `villageScenario` and `enduranceScenario` there. The workbench reads the village's placed boxes from the resolved map. Rust tests read `sim::maps::load(id)`; the test helper `common::saved_map(id)` is the resolved map as the JSON a scenario takes.

**Not saved maps.** The camera lab's plan and the generated lab's request are compiled at run time and stay where they were; they join this seam with C55 and C58. Foundation's and the workbench's render-only grounds are C56's, as before.

**Proof.** Captured before the cutover and compared after, with no difference:

- natively, the typed scenario (sha256 of its JSON), scenario digest, config digest and battle digest of both village variants (600 ticks) and of endurance at seed 1, seed 1 late and seed 7;
- through Wasm, the scenario digest, config digest and battle digest at tick 0 and after 300 ticks of all fifteen lab encounters, both village variants and the three endurance battles, and the lean firefight against its old composition.

The config digest is `9349bc07e09ad210` before and after: no config identity moved. No `fixtures/parity/` file changed. `scene -- --list` is unchanged. The entry script went from 238,249 to 231,509 bytes (75,033 to 73,751 gzipped), because the village's map left it.
