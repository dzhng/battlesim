# Implementation audit cleanup

This pass repairs disagreements between simulation knowledge, player commands and presentation, and removes test layers that duplicated ownership without adding proof. The simulation remains authoritative; presentation acts on observations and requested user intent.

## Why these boundaries matter

A selected casualty must not invalidate orders to surviving units. A distant squad must not learn that a building collapsed through command rejection. A remembered prop must retain its whole pose. Replacement soldiers restore authored squad capacity, without corpse history enlarging their cover area. The owners are the [input hook](../../../web/src/battle/input/useUnitControl.ts), [side geometry](../../../crates/sim/src/movement/mod.rs), [garrison rules](../../../crates/sim/src/garrison.rs) and [cover policy](../../../crates/sim/src/cover.rs).

Asynchronous work must have one lifetime. Requested pause is independent of a worker waiting for its consumer. Posted terminal replies survive authority shutdown. A viewport owns a frame while its appearances load, so unmount can dispose it immediately. Appearance upload and distant-view baking share buffers and form one queued transaction. These contracts live in the [simulation client](../../../web/src/battle/sim/client.ts), [viewport](../../../apps/battle-lab/src/LabViewport.tsx) and [frame owner](../../../packages/battle-renderer/src/frame/battleFrame.ts).

History belongs where it is consumed. Ordinary sessions retain the latest digest and bounded captions; the authority diagnostic owns its full recording. Tests own independent oracles, not abandoned production APIs. Scenario registries execute their own cases; rule matrices belong in Rust, while browser scenes retain input, publication, DOM and GPU proof. See [test suites](../../../web/tests/) and [simulation tests](../../../crates/sim/tests/).

## Readout intent and provenance

The user's [reported screenshot](assets/readout-order-reference.png), captured from this project on 2026-09-29, establishes the spatial problem: the tank is below the AT team but its panel is above it. Its [provenance](assets/README.md) records why it is retained as diagnostic documentation. Labels should follow their subjects' vertical order where space permits, retain all eligible panels under crowding, and place selected panels in front. Placement and leader routing remain best effort. [Soft dark backing](../readout-backgrounds/README.md) supports text contrast with breathing room around the text and no enclosing frames.

The [readout layer](../../../web/src/battle/present/readouts.tsx) and [browser scene](../../../web/scenes/readouts.mjs) own placement and its consumer proof. A lower pixel-difference score is not the goal: the workbench's restored scale-figure shadow is correct even though it differs substantially from the earlier capture, which had lost that world update.

## Decisions and validation

[Decisions](choices.md) records the queue latency, test-filtering and crowded-layout tradeoffs. [Verification](progress.md) records scope, coverage preservation and performance evidence. Detailed local logs and comparisons live in `throwaway/implementation-review/closeout.md`; they are evidence from this run, not runtime dependencies.
