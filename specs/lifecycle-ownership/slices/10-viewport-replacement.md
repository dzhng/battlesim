# 10 — Viewport scene admission and replacement

**Status:** complete. **Audit families:** 27. **Dependencies:** 04, 06.

## Contract and question

Can one viewport-local owner manage complete scene builds and replacements while existing render owners stay intact?

Current owners/consumers: [LabViewport.tsx](../../../apps/battle-lab/src/LabViewport.tsx), [scene.ts](../../../web/src/battle/render/scene.ts), [sceneAppearances.ts](../../../web/src/battle/render/sceneAppearances.ts).

## API seam and target shape

Extract a local SceneAdmission owner from the viewport effect. It owns the installed BrowserScene, current build/rebuild operation and captured identity of observer, world, required art and quality. Its request/current/dispose surface receives current input suppliers and the existing BrowserScene factory. Keep AppResources as page GPU owner and SceneAppearances as asset-only replacement owner. No second frame scheduler or common GPU resource manager.

## Transitions and invariants

Wait for current canonical topology; build against captured complete inputs; obsolete success frees its scene and yields to the current request, obsolete rejection yields without refusing the page, current failure terminates readiness. Rebuild remains bounded to existing work; late pose inputs install before readiness. Departure withdraws telemetry/diagnostic borrows before freeing scene. Preserve when old scene is drawable versus intentionally skipped for topology mismatch.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete scattered scene/rebuilding/admission recursion ownership from LabViewport once the local owner handles it. Do not extract shaders, camera math, runtime passes or the already coherent SceneAppearances admission.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [viewport.test.tsx](../../../web/tests/viewport.test.tsx), [appResources.test.ts](../../../web/tests/appResources.test.ts), [sceneAppearances.test.ts](../../../web/tests/sceneAppearances.test.ts), [battleFielding.test.tsx](../../../web/tests/battleFielding.test.tsx).

Existing viewport tests cover departure while admitting, latest observer/quality/poses, stale rejection, current terminal failure and complete topology bypassing asset-only admission. Keep them at the mounted viewport boundary. Run the smallest actual replacement browser scene after the seam is green; no full renderer benchmark absent a new concern.

Replace session, world and graphics quality in existing battle/workbench paths; depart while loading and return with the same page GPU.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Complete battlefield and its loading/error cover across replacement; art/quality inputs held constant for preservation shots.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private class/module name and input supplier grouping. Device lifetime, canonical framing, work bounds and old-scene draw policy are fixed.

Feedback that would change scope: Automatic GPU recovery, cross-generation sharing or changed rendering quality belongs to separate specs.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

`SceneAdmission` in `apps/battle-lab/src/viewportSceneAdmission.ts` now owns the
viewport's complete observer/world/art/quality identity, serialized candidate
builds, stale candidate disposal, scene replacement, and departure cleanup.
`LabViewport` retains frame scheduling and camera/input ownership; `AppResources`
and `SceneAppearances` remain the page GPU and asset-only owners.

Focused checks: `viewport.test.tsx`, `viewportAdmission.test.tsx`, `appResources.test.ts`,
`sceneAppearances.test.ts`, and `battleFielding.test.tsx` — 42 tests passed.
The viewport suite adds coverage for initial-build refusal and a
consumer-requested rebuild failure ending readiness. Web typecheck, changed-file Oxlint, and changed-file Oxfmt checks
passed; Oxlint retains the existing exhaustive-deps warning in `LabViewport`.

The generated replacement browser scene was attempted but could not reach
readiness because this checkout's runtime art is still Git LFS pointer content;
the run also reported the existing menu audio decode limitation. No screenshot
was produced and successful scene builds preserve the existing pixels, so the
compare-screenshots, screenshot-critique, and preview gates were not applicable.

Claude Opus 5.5 high-effort review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Ab3b7bb17-b597-4fa3-9ae5-cc4a9e8396d4%3Adelegate-task%3A3262d6eb-2c8a-4d7f-a231-9d780bb3b58e>.
Findings were verified and fixed before commit: rebuild failures report through
the viewport boundary, first-build failures retain page refusal behavior, and
diagnostic/telemetry borrows withdraw before scene disposal.
