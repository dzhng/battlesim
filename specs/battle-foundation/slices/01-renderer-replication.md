# 01 — Pinned stack and 3D reproduction

**Status:** complete 2026-09-25. **Dependencies:** none. **Milestone:** Village checkpoint.

## Contract and question

Can the pinned reusable foundation produce an honest 3D frame on this machine before new gameplay is attached?

User requirements owned or exercised: S02, S03. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`packages/renderer-core` camera/device/depth/lifecycle; `packages/battle-renderer::createScene(device, mesh, instances) -> {render, resize, dispose}`. Caller owns device/canvas; scene owns its allocations.

## Runnable artifact

/lab/foundation in the source-only battle lab: an asymmetric tank proxy, facing-marked infantry, a raised ground patch and box prop. Pan/orbit/zoom, select a visible proxy and reset. First reproduce the pinned camera/depth behavior and record it; then adapt only the audited modules. Establish the root Bun aliases, web Vite/React/TypeGPU dependencies and Cargo contract/sim/WASM crates. No sibling runtime imports.



## Verification and verdict

Port camera screen/world round-trip and reverse-Z tests and resource cleanup behavior. Probe actual browser/adapter; capture installed clear/comparison depth states. Repeated resize/reset/dispose must return owned resource counts to baseline. Write the exact reuse manifest and dependency/license inventory. Run one current-host headful capability probe only if headless GPU differs; routine captures remain headless. Report unsupported GPU honestly.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Spatial framing and depth correctness**. Review crop/mask: **Full 1280×800 frame; tank nose/prop overlap crop**. Explicitly out of scope: Fog, game logic, texture/lighting polish and soldier animation.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Primitive topology/color; internal scene factoring. Pin source versions, coordinate and ownership contracts. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Illegible silhouettes or incorrect scale/projection changes the primitive scene before gameplay depends on it.

## Verdict — 2026-09-25

Accepted. `bun run --cwd web scene -- foundation` passes 15 checks on this host: headless Chromium 148 (`channel: "chromium"`) with a **hardware** Apple `metal-3` adapter, so no headful probe was needed. The checks cover:

- Installed depth state is depth32float with clear 0 and compare `greater`, read from the descriptors the scene installed.
- Occlusion is correct along one line of sight in both directions (crate hides tank; tank hides crate).
- Picking selects the tank and returns nothing for sky.
- Middle-drag orbits, WASD pans, wheel zooms and reset restores.
- Four resizes and three full rebuilds return live GPU buffers and textures to baseline.
- A browser without `navigator.gpu` shows an actionable message instead of loading forever.

Unit tests port camera3d verbatim, and adapt the uniform packing (160 B, sized against the WGSL struct), picking, the orbit rig and allocation counting. [Reuse manifest](../assets/reuse-manifest.json) lists every copied file.

Found while verifying: TypeGPU 0.12.5 `root.destroy()` does **not** free buffers the root created. Each rebuild leaked 11 buffers until the scene registered and destroyed its own allocations.

Visual gate: single-image diagnostics (frame entropy 2.4 bits, edge density 0.045) fit a flat-shaded primitive scene with a small subject. Two unprimed critiques ran.

- **Confirmed by both:** depth ordering is correct, with no missing or inverted faces.
- **Fixed after the first:** tank sinking into sloped ground (bump moved off the tank), unreadable facing cues (larger wedges), crushed dark tones, aliasing (4× MSAA).
- **Fixed after the second:** implausible proxy scale (crate 1.2 m, tank 2.2 m, slimmer soldiers) and the truck straddling a narrow bump.
- **Left as intended or out of scope:** the barrel over the crate is the deliberate overlap probe; missing shadows are lighting polish; the visible patch edge belongs to this render-only fixture, replaced by the bounded map in 02.

Preview set offered 01:36 with no response, so the reversible primitive choices stand.

Evidence captures live in gitignored `throwaway/evidence/foundation/` and are regenerated by the scene; they are not committed (see choices.md).
