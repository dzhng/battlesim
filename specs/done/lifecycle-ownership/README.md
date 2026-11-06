# Lifecycle ownership

This refactor gives each selected browser, editor, audio, and map lifetime one local transition owner while preserving the simulation as the authority. Commands, observations, replay and fixture formats, artifact IDs and receipts, playback policy, cancellation policy, and deterministic battle outcomes remain unchanged. The cutovers are internal and direct: there is no compatibility layer, migration, shared FSM dependency, or second lifecycle framework.

## Why this shape

The original audit mixed real ownership defects with healthy orthogonal state. The shipped change addresses the 16 rows where transition policy was duplicated: request settlement, worker handoff, authority resources, session startup, uploaded play, pointer gestures, purchase placement, order preview and released actions, viewport scene admission and submitted snapshots, page and battle audio, mechanics editing, map draft/manual/artifact lifetimes, sound review, and audition sessions. The other 69 rows remain in their existing owners because counters, timers, queues, physical components, replay evidence, and already-cohesive phase owners express independent facts more clearly than a new enum would.

Each owner is deliberately local. Request families retain separate settlement rules; pointer buttons remain independent; map background work stays in `LatestRunner`; fixture publication remains the durable transaction authority; SoundBank, voice arbitration, Rust presentation, and simulation phase owners are not wrapped. This avoids coupling dimensions that only look similar in a large component.

## Invariants

- The simulation and native publication paths remain authoritative. Browser state observes them and never changes command ordering, tick timing, side knowledge, replay identity, digest bytes, or packed publication bits.
- Public HTTP, worker, observation, replay, fixture, map, sound, and audio contracts retain their existing shapes. Internal producers and consumers cut over together; no compatibility aliases remain.
- A late result can mutate only the operation or resource identity that admitted it. A failed resource retains the handle needed for cleanup. Retained last-good scenes, maps, drafts, and receipts remain visible under the same policies.
- Cancellation and promise settlement remain family-specific. Prepared cancellation stays silent; advance completion still waits for acknowledgement and publication release; map and audition cancellation retain their existing late-success behavior.
- A submitted camera, pick set, and frame completion refer to the scene that actually drew. Replacement scenes withhold picks and staged work until their first successful submission; this is the named submitted-view correction.
- Editor review certificates bind to the exact draft they previewed. Editing, conflict, refusal, or failed save does not silently replace the current draft.
- Page audio owns context and shared preparation; a battle owns its disposable observer graph and evidence. Menu playback ends only when admitted battle audio updates. Auditions own their load, playback, stop, and replacement identity; chatter and voice-generation queues remain separate.
- The 85-row audit remains closed at 16 Change and 69 Keep. Conditional draft ideas that were not independently justified remain retained, especially replay import, appearance installation, graphics detection, movement, script completion, and voice jobs.

## Code pointers and proof

The principal owners are greppable by these modules and types:

- `web/src/battle/sim/client.ts`, `authority.ts`, and `prepare/client.ts` own request settlement, admitted resources, and worker adoption.
- `apps/battle-lab/src/useSimSession.ts` and `routes/scenarioPlay.tsx` own session continuations and uploaded-route payloads.
- `apps/battle-lab/src/viewportInput.ts`, `orderPreview.ts`, `releasedAction.ts`, `viewportSceneAdmission.ts`, and `viewportSubmittedView.ts` own pointer, preview, acknowledgement, scene, and submitted-frame transitions. `ViewportFrames` remains the frame waiter owner.
- `packages/battle-audio/src/appAudio.ts` and `battleAudio.ts` own page and battle graph lifetimes. `apps/sound-workbench/src/soundWorkbenchState.ts`, `auditionSession.ts`, and `audition.ts` own sound review and auditions.
- `apps/mechanics-editor/src/editorState.ts`, `apps/map-workbench/src/editorState.ts`, `manualOperation.ts`, and `apps/map-workbench/artifacts.ts` own editor operations and map artifact storage. `LatestRunner` and `FixturePublication` retain their prior responsibilities.

Consumer tests pin behavior at the existing seams under `web/tests/`, including the client, preparation, authority, session, pointer, purchase, order-preview, viewport, audio, mechanics, map, sound, audition, and voice suites, plus map-workbench server and native tests. Mutation receipts for stale identities, skipped submissions, cancellation waiting, artifact retention, and audition cleanup live under ignored `throwaway/lifecycle-ownership/` evidence.

## Named corrections and dead ends

The refactor preserves behavior by default. Four implementation findings were named and tested separately: session warm-up continuation ownership, right-release adoption of the held gesture, stale audition/module-load and expected `AbortError` handling, and submitted-view replacement admission. The submitted-view correction records its trigger, previous outcome, affected picking/frame consumers, invariant, and regression tests in `decisions.md`.

A universal pending-request table was rejected because command, preview, advance, and replay namespaces have different settlement contracts. A single battle/editor FSM was rejected because pointer lanes, map retention, audio graphs, and simulation phases have independent lifetimes. Converting Rust movement, weapon, victory, or presentation fields was rejected because those fields carry physical or replay evidence that intentionally coexists.

## Verification record

Each implementation pass has focused consumer tests and review evidence; the final integration review reconciled all 85 scope rows and found no obsolete selected-owner symbols.

The once-only full `bun run check` reached the Rust suite, passed the non-audio tests observed before the audio fixture gate, and stopped because native audio fixtures are Git LFS pointers (`assets/runtime/audio/clips/aa-burst.wav`). The final `bun run verify` reached browser scenes and was blocked by unfetched runtime art, visual baselines, and audio. These are repository asset availability limits, not lifecycle test failures; no requirement or tolerance was weakened.

No visual provenance image belongs to this refactor: player-facing geometry, rendering, and the audio mix were intentionally preserved. Browser visual gates were attempted only through the repository verifier and could not load the required LFS assets.
