# Slice 05 — migration, deletion and final verdict

**Status:** implementing. Production browser comparison is complete; native
profiling, final deletion coverage, startup/accounting and closeout remain.

Use the actual production entrypoints to compare TypeGPU browser, shared Rust
browser and shared Rust native with matched presentation, camera, viewport,
quality, time and authored asset generation. Freeze the old control's commit,
artifacts and raw evidence before deleting it.

Report frozen-frame and changing-reel CPU preparation/encoding, GPU timestamp
cost, cold startup/asset transport/decode, first visible frame, resize, surface
recovery and real device recreation separately. Queue completion is not GPU time.
Record submitted geometry/draws/passes, GPU-owned payloads, CPU retained payloads,
WASM linear-memory high-water marks and binary/compressed transfer sizes. State
what excludes allocator, browser, driver or canvas storage.

Inspect whole images and material/shadow/effect/player-overlay landmarks, with
unprimed critiques. Preserve approved visual tests. A favorable average cannot
hide a missing feature, a failed lifecycle path or a wrong frame.

The final complexity comparison includes shared core, both platform shells,
shader authoring/tooling, tests/harness and deleted owners. Delete the TypeGPU
browser runtime/dependency and standalone production-shaped Rust graph. Any
build-only TypeGPU authoring exception needs a separate measured comparison
against simpler authoring and an explicit decision. Retire obsolete ownership
controls only with the obsolete implementations; preserve their relevant
behavior/lifetime coverage through the actual runtime.

The decision favors substantially less code and fewer owners even with equal or
slightly worse performance. Do not invent a frame-time threshold or call an
incomplete renderer simpler because it omitted features.

When all gates pass, review the whole spec implementation, run repository
closeout once, consolidate the choices against the actual shipped architecture,
and archive with close-spec. No slice/global decision may remain silently open.
