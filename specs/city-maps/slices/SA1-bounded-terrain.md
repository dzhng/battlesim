# SA1: exact bounded terrain and public exports

**Depends on:** S0 failed-owner evidence. **Kind:** focused representation proof and owning production correction.

## Question
Can the exact authoritative surface reach browser consumers without whole-map triangle/sample expansion?

## Contract it unlocks
A frozen representation/equivalence/resource verdict and one owning production correction after that verdict passes. [S0 baseline](../assets/scale-baseline/README.md) remains immutable. Complete G0/visual acceptance stays open; an individual systems unlock is recorded only for the proved contract.

## API seam
HeightField queries → compact public sampled surface → public picking and bounded terrain/fog/grass height consumers. Preserve closed bounds, SW→NE interpolation, normals, ray first-hit ordering, water/road/bridge precedence and existing relief fixtures. Flat ground is implicit; local nonflat samples belong to the same representation. No renderer-owned resampling or second simulation world.

## Candidate investigation
Compare implicit-flat/local-sample tiles with the frozen dense terrain oracle. Do not choose an approximation merely because it is cheap. Coalescing is permitted only on exactly planar sampled regions. Export once and migrate terrainGrid, terrainSurface, world bounds, fog height and grass height consumers; an unbounded consumer makes the proof fail.

## What the human can run or see
Matched original/candidate reports, focused neutral query/route/sampling overlays and fixed-extents resource rows. No new visual design/model is authored.

## Verification
Freeze analytic height/normal/ray/classification outputs and matched battle digests before changing source. Existing world_geometry, terrainSurface and picking suites own behavior. Native/wasm corners, edges, diagonals, tile joins, relief and separated river/bridge detail agree within existing precision tolerances. Exact full-size bounds and opposite-edge relocation are required.

Report source/live/peak/transfer/query/resident bytes. Flat storage must be independent of empty area; local detail follows occupied samples/patches. No full MeshBuilder JS-number staging or whole-map GPU ground buffer. Actual Metal consumer checks respect device limits and G0 budgets. SA1 unlocks its terrain/export owner, not complete frame/art acceptance.

Resource arms run serially and preflight permanent/temporary overlap against S0's ceiling. If a browser capability changes drawn output, compare production-route bytes/pixels against the frozen baseline and inspect the relevant masks via compare-screenshots; run unprimed screenshot-critique last; preview-shots is non-blocking. This does not certify deferred artwork.

## Delegated to the implementer
Internal representation candidates and instrumentation inside the stated parity/resource contracts. Record the selected design and rejected measured arms. A change to physical/route/knowledge rules or a user requirement is not delegated.

## Must stay green
Frozen authoritative outcomes, native/wasm parity, side knowledge and fixed extents. One owner per concept; remove dense/duplicate paths in the same consumer cutover.

## Feedback that would change this slice
A parity/resource failure reslices the failed owner before more implementation; user changes to the fixed requirements require new evidence.
