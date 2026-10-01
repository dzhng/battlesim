# C09: common map acquisition and resolution

**Depends on:** C01; SG6's dry run. **Kind:** slice.

## Question
Can saved and eventual generated sources feed one compiled-map initialization contract without bundling maps in JavaScript?

## Contract it unlocks
`MapSource` and `ResolvedMap { definition, identity }` establish one resolution seam. Saved scenarios initially use the catalogue variant; C55 implements the generated-request variant through the same owner. Every saved map moves to `fixtures/maps/<id>/map.json` in this cutover: village, lab fixtures and endurance, with the actual inventory checked rather than a stale count. Rust and browser resolution load the same contract; the sim receives only the resolved definition.

Static consumers of village map data read the resolved public geometry instead of importing it from village.json. C60 adds saved metadata; runtime sources later keep equivalent identity in memory/replay and require no catalogue folder. No map is bundled in the JS entry point; no second loader or alias remains.

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
