# Lifecycle ownership

**Status:** implementation in progress. **Updated:** 2026-11-06.

The simulation remains the authority, public formats remain unchanged, and each
resource or operation gets one transition owner where that removes scattered
coordination. This is a behavior-preserving cleanup of browser and developer-tool
lifecycles. It introduces no state-machine dependency, compatibility layer, data
migration, gameplay change, or Rust simulation/presentation rewrite.

## Next Agent Prompt

You are implementing the beneficial lifecycle cleanups selected from the app's
state-machine audit. Read the repository README and development guide, then this
plan's [contracts](contracts.md), [scope decisions](scope.md),
[verification policy](verification.md), and [decisions](decisions.md).

Slices 01–10, 12–16 and 18–19 are complete: client settlement, shared preparation-worker
handoff, authority resource ownership, session startup, pointer/purchase/order/released-action
ownership, viewport scene admission, page/battle audio, mechanics editor,
map draft/manual/artifact ownership, and audition session ownership all passed focused checks
and Claude review. Continue with the integration closeout.

There is no blocking product decision. The [source-derived behavior
questions](decisions.md#behavior-questions-outside-this-refactor) are deliberately
outside the preservation refactor; do not silently resolve them. A demonstrated
defect encountered during implementation follows the correction procedure there.

Use existing consumer tests first; add a missing preservation case before changing
its owner. Keep old and new state from becoming parallel authorities. After each
slice, run the repository closeout review and an independent Claude review (current
Opus, high effort), verify and resolve its findings, record the evidence and
any decisions, and update this prompt and the checklist before ending your pass.
Do not mark this feature shipped until [20: closeout](slices/20-closeout.md).

## What qualifies

A change earns its place when it deletes coordinated writes from consumers,
concentrates a duplicated lifetime, or makes a resource-bearing state carry the
resource it requires. Merely naming an optional value, a timestamp or a derived
status is not an improvement.

The [scope ledger](scope.md) accounts for every one of the original 85 audit
families: 16 selected and 69 retained. It is the canonical scope decision, not an
instruction to convert the entire audit. The selected families need 19 focused
implementation slices plus one integration closeout. Splitting them avoids asking
one change to redesign input, async admission and presentation together.

The desired result reads as code designed with these owners from the beginning:
local domain types, direct consumers, and no old representation synchronized with
its replacement. Retained mechanisms have a concrete consumer and contract. A
failed resource can still own cleanup obligations; it must not lose its handles
just because its visible state says failed.

## Slice graph and global TODO

Dependencies describe integration order, not a requirement to run parallel agents.
Slices touching the same file should be implemented sequentially. Independent
passes use isolated checkouts under implement-spec; share installed dependencies
when safe, never mutable build output. Avoid GPU contention or permanent debug routes.

| Done | Slice | Depends on | Review surface |
|---|---|---|---|
| [x] | [01 Client requests](slices/01-client-requests.md) | — | Authority command, preview and advance results |
| [x] | [02 Preparation worker](slices/02-preparation-worker.md) | — | Generated and uploaded battle handoff |
| [x] | [03 Authority resources](slices/03-authority-resources.md) | 01 | Pause, step, credit stall and restart |
| [x] | [04 Session startup](slices/04-session-startup.md) | 01, 02, 03 | First frame and scripted warm-up |
| [x] | [05 Uploaded scenario](slices/05-uploaded-scenario.md) | 02, 04 | Package → controller → battle |
| [x] | [06 Pointer gestures](slices/06-pointer-gestures.md) | — | Selection, facing and camera gestures |
| [x] | [07 Purchase placement](slices/07-purchase-placement.md) | 01, 06 | Ghost, preview and confirmation |
| [x] | [08 Order preview](slices/08-order-preview.md) | 01, 06 | Move/building answer and marker |
| [x] | [09 Released action](slices/09-released-action.md) | 08 | Acknowledgement correcting its own gesture |
| [x] | [10 Viewport replacement](slices/10-viewport-replacement.md) | 04, 06 | Replace scene while current view is owned |
| [ ] | [11 Submitted view](slices/11-submitted-view.md) | 10 | Picking and frame completion after submit |
| [x] | [12 Page audio](slices/12-page-audio.md) | — | Permission, mute and menu/battle handoff |
| [x] | [13 Battle audio](slices/13-battle-audio.md) | 12 | Ready graph, fresh evidence and departure |
| [x] | [14 Mechanics editor](slices/14-mechanics-editor.md) | — | Invalid text, current review and conflict |
| [x] | [15 Map draft/display](slices/15-map-draft.md) | — | Latest request versus last admitted map |
| [x] | [16 Map manual work](slices/16-map-operations.md) | 15 | Inspect, compare, sample and cancel |
| [x] | [17 Map artifacts](slices/17-map-artifacts.md) | 16 | Retention, release and server close |
| [x] | [18 Sound review](slices/18-sound-review.md) | — | Draft review, save and retained errors |
| [x] | [19 Audition session](slices/19-audition-session.md) | 18 | Loading, playback, stop and replacement |
| [ ] | [20 Integration closeout](slices/20-closeout.md) | 01–19 | Complete preserved player/tool journeys |

The [interactive roadmap](visualizations/roadmap.html) is a generated review aid;
the linked slice files and this checklist own the live plan.

## Human review and feedback

The first useful playable checkpoint is slice 01 in the existing authority lab:
commands, paused advancement, previews and withheld publication credit still work.
After slice 04, review lobby deployment, cancel/navigation, Restart and the menu
reel. Input, viewport, audio and editor lanes each have their existing real routes;
the slice files identify what to exercise. A new FSM dashboard would add another
surface to maintain and is not part of this work.

For any visual capture, compare the candidate to the same before image with
`compare-screenshots`; obtain an unprimed `screenshot-critique` as the last visual
acceptance check. Show useful evidence with `preview-shots`. Keep captures and
logs in ignored `throwaway/`, as the repository requires. A human checkpoint is
non-blocking: allow roughly five minutes for feedback while continuing independent
work, then decide from the evidence, record why, close the opened shots and proceed.
Silence never approves a changed requirement or a changed baseline.

## Scope boundaries and completion

The existing shared GPU asset, distant-impostor, water/native and launcher plans
retain their own work. This plan does not change asset sharing, quality thresholds,
shaders, models, fixture values, battle tuning, native replay output, or publication
layout. Existing Rust phase owners, timing systems and journals stay authoritative.

There are no missing assets or credentials. Small controlled-response tests can
run before browser prerequisites are available. Materialize only the assets needed
by the owning browser scene when that gate is due; unavailable assets are a named
verification limitation, never permission to weaken a check.

Completion requires every selected owner migrated, obsolete state removed, every
scope row reconciled, preservation gates passed, and one full repository closeout.
Use `close-spec` only after implementation has shipped, rewriting the plan into
durable ownership rationale rather than retaining this build checklist forever.
