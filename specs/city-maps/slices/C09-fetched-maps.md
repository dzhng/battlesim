# C09: common map acquisition and resolution

**Depends on:** C01; SG6's dry run. **Kind:** slice.

## Question
Can saved and eventual generated sources feed one compiled-map initialization contract without bundling maps in JavaScript?

## Contract it unlocks
`MapSource` and `ResolvedMap { definition, identity }` establish one resolution seam. Saved scenarios initially use the catalogue variant; C55 implements the generated-request variant through the same owner. Every saved map moves to `fixtures/maps/<id>/map.json` in this cutover: village, lab fixtures and endurance, with the actual inventory checked rather than a stale count. Rust and browser resolution load the same contract; the sim receives only the resolved definition.

Static consumers of village map data read the resolved public geometry instead of importing it from village.json. C60 adds saved metadata; runtime sources later keep equivalent identity in memory/replay and require no catalogue folder. No map is bundled in the JS entry point; no second loader or alias remains.

## API seam
`contract::scenario` map source/identity, browser map resolver and wasm preparation, native map resolution. C55 adds generation acquisition here without altering downstream battle/renderer consumers.

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
A source consumer still bypassing the resolver blocks the cutover until it is migrated.
