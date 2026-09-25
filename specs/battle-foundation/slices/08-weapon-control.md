# 08 — Independent weapons and engagement policy

**Status:** planned, not implemented. **Dependencies:** 03, 04, 05, 06, 07. **Milestone:** Village checkpoint.

## Contract and question

Do target selection and simultaneous aim/reload obey every interruption and policy rule?

User requirements owned or exercised: V07, V10, V12, V13, W01, W02, W03, W04, W05, W06, W07, W08, W09, W10, W11, W12, W13, W14, W15, W16, W17, W18, P11, U01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::weapons::advance(own_state, observation, order, dt) -> LaunchRequest + WeaponReadiness`; unit-owned Engagement/OrderQueue.

## Runnable artifact

/lab/weapons: infantry rifle+grenade and tank cannon+HMG, moving/Stop/visibility events, switch policies, attack identified/contact/ground and attack-move. Timers are raw diagnostic bars until slice 14. Cannon ammo variants share one mount.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

All contracts.md transition rows; 45-tick grace advances without hidden tracking; stationary resets; target lock through shot; explicit override; cost priority; general-purpose uncertain fire; invulnerable default-gun fallback; per-unit policy switch; attacker-specific return permission; automatic no-pursuit; explicit last-known pursuit; vehicle-only prefire withholding; rifle bodies remain hittable. Speculative exchanges may persist.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Action reason correctness**. Review crop/mask: **Selected-unit diagnostic action panel**. Explicitly out of scope: Final circular timer styling and damage balance.

1. Freeze fixture seed, tick, camera, viewport and DPR. Save full frame and tight 2×–4× crops in this spec's `assets/evidence/08/` with machine/config metadata.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Also test visibility loss → approximate contact → reidentification with automatic targeting enabled: grace must not be destroyed by fallback. Test loaded AP/HE swap and stock conservation, reload-kind interruption, and no AP speculative shot.

## Decision budget

Delegated: Table organization/internal target cache, never alternative policy behavior. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Contradictory action reasons or repeated target churn triggers a transition-table revision, not ad hoc flags.
