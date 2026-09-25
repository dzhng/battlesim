# 01 — Pinned stack and 3D reproduction

**Status:** planned, not implemented. **Dependencies:** none. **Milestone:** Village checkpoint.

## Contract and question

Can the pinned reusable foundation produce an honest 3D frame on this machine before new gameplay is attached?

User requirements owned or exercised: S02, S03. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`packages/renderer-core` camera/device/depth/lifecycle; `packages/battle-renderer::createScene(device, mesh, instances) -> {render, resize, dispose}`. Caller owns device/canvas; scene owns its allocations.

## Runnable artifact

/lab/foundation in the source-only battle lab: an asymmetric tank proxy, facing-marked infantry, a raised ground patch and box prop. Pan/orbit/zoom, select a visible proxy and reset. First reproduce the pinned camera/depth behavior and record it; then adapt only the audited modules. Establish the root Bun aliases, web Vite/React/TypeGPU dependencies and Cargo contract/sim/WASM crates. No sibling runtime imports.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Port camera screen/world round-trip and reverse-Z tests and resource cleanup behavior. Probe actual browser/adapter; capture installed clear/comparison depth states. Repeated resize/reset/dispose must return owned resource counts to baseline. Write the exact reuse manifest and dependency/license inventory. Run one current-host headful capability probe only if headless GPU differs; routine captures remain headless. Report unsupported GPU honestly.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Spatial framing and depth correctness**. Review crop/mask: **Full 1280×800 frame; tank nose/prop overlap crop**. Explicitly out of scope: Fog, game logic, texture/lighting polish and soldier animation.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/01/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Primitive topology/color; internal scene factoring. Pin source versions, coordinate and ownership contracts. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Illegible silhouettes or incorrect scale/projection changes the primitive scene before gameplay depends on it.
