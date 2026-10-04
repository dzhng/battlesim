# HUD, chrome and app flow

## Next Agent Prompt

Status: active goal in `/Users/server/dev/battlesim-hud`, branch `hud-chrome`.
Navigation and retained audio lifetime are integrated. The user supplied `menu.mp3`;
its full recording replaces both rejected synthetic cues through the existing bank.
Root resource/shell/input changes pass focused and actual GPU visit-cycle checks,
but remain uncommitted. Deck final evidence is undergoing fresh visual critique.

Pickup: finish and commit the recording pass; integrate the reviewed deck; commit
root resource/input/editor-navigation work. Then finish current screen composition
and integrated journey, followed by the one full check/verify and closeout. Preserve
`BrowserRouter unstable_useTransitions={false}` and the [exploration decisions](unknowns-map.html).
Do not repeat the interview. External skill edits are unrelated: never stage/revert them.

Global TODO:
- [x] [01 navigation and visit lifetime](slices/01-navigation.md)
- [ ] [02 shared GPU, warm-up and required-resource failure](slices/02-resources.md)
- [ ] [03 shared audio and menu atmosphere](slices/03-audio.md)
- [ ] [04 joined compact deck and icon commands](slices/04-deck.md)
- [ ] [05 current screen composition and input ownership](slices/05-screens.md)
- [ ] [06 integration and closeout](slices/06-closeout.md)

No external blocker. Menu/music taste checkpoints are reversible and non-blocking. Scratch output goes in ignored `throwaway/`; only the frozen concept reference belongs in spec assets. Dependencies share main's installed tree; every checkout has its own generated/build output. All GPU work uses `/Users/server/dev/battlesim/throwaway/gpu.lock`.

## Contracts and scope

A navigation starts one visit. Publishing an admitted exact battle address preserves it; Main menu/Back discard it immediately. Forward or a later same-path navigation starts fresh exact inputs. Pause alone offers Resume. Use basic declarative React Router, retaining `battleLinks.ts` and the fixture registry. Unknown paths keep the existing menu fallback. No lobby, results screen, battle cache, touch gameplay, confirmation, retries or compatibility scaffolding.

The app retains its admitted GPU, page WASM/appearances and one audio context/bank. Viewports dispose scene registries, canvas configuration and listeners; battles release workers/world/map/voices/evidence. Warm only reusable resources after menu paint, never a map; kits remain demand-loaded. Required failure uses refusal and manual Reload. A lost/destroyed app device cannot be reused; real document departure tears down app resources and a BFCache restoration requests manual Reload rather than silently reviving destroyed handles.

Audio attempts unmuted playback as early as permitted and resumes on the first qualifying menu gesture under normal browser policy. Keep the restrained musical bed and subtle outdoor ambience through loading, fading when combat audio is ready. Persisted mute/master volume remain authoritative. Use the user-supplied Battlefield 2 menu recording through the existing catalog/bank pipeline, starting at its introduction; audition actual music and transition, preserving provenance.

The deck joins the existing selection panel and commands at bottom center. Use the frozen [concept](assets/compact-deck-reference.png) for composition, not as a fixed 612 × 80 cap. Keep one InfoPanel vocabulary and readable facts; allow rich states and multi-selection to grow/wrap. Icon commands have name/shortcut tooltips on hover and keyboard focus. Remove only visible Move/Garrison; right-click keeps both. Retain contextual Deploy/Pack/Leave building, partial reach, shortcuts and replay read-only behavior. Captions follow the actual deck through layout; readout occlusion hooks stay intact.

## Graph and owners

01 navigation, 02 resources, 03 audio and 04 deck may progress independently with separate owned files/checkouts. Integrate their public seams before 05. 05 depends on navigation/deck and freezes their accepted contracts while styling each current menu, loading/refusal, pause and replay surface. 06 follows all passes.

Use existing owners: route registry and typed URL parser; WASM page cache; AppearanceLibrary; GPU allocation tracker and scene registry; sound catalog/bank/settings and SoundFrame; SelectionCard/InfoPanel and command bindings. No parallel panel renderer, navigation framework, preload scheduler or music service.

## Verification and reviews

Load write-tests before behavior changes and prove each tracer test red then green. Load game-ui before UI and renderer before viewport/resource work. Each pass runs review (shape, diff, docs), independent Codex review for substantive changes, and audit-choices into [choices](choices.md). Commit each clean pass and continue. Narrow tests/scenes suffice until closeout. Run root check and verify once when implementation is finished; presentation/audio need no balance report.

Every visual acceptance uses a matched production-route before/after pixel comparison, compare-screenshots against the relevant reference/crops, and an unprimed screenshot-critique as the last visual check. Use preview-shots for a compact review set. Give the user a non-blocking review window while continuing unrelated work; record the reversible decision if no response and close Preview. Music gets an actual listening sample and the same non-blocking taste checkpoint. Tests run headless, without playing sound or writing real user settings.

## Evidence

| Pass | Evidence / current state |
| --- | --- |
| Plan | Three independent drafts reconciled: minimal cuts, risk-first proofs, seam ownership. Read real route, GPU/audio and panel owners. React Router declarative docs/source confirm replacements generate keys; visit identity therefore cannot blindly follow replacement keys. |
| Navigation | Integrated navigation commit `63bdd72a`. Red/green history/publication/cancellation/replay proofs, real worker exit/Back/Forward and replay digest gates pass. Independent review found rapid-return retention; synchronous route navigation and permanently retired continuations resolve it. Follow-up found no remaining navigation defect. Existing benchmark-loading polling can time out under concurrent test compilation; no threshold changed. |
| Resources | Device/appearance/viewport checks red→green, including canvas release, lost-device refusal, demanded kits and manual recovery. Independent review caught hot-reload cleanup destroying page GPU; fixed and regression-tested. Actual foundation GPU visits pass: one admission, zero device destroys, scene allocation baseline after exits and no menu battle worker. Cache-removal mutation reproduces the failure. |
| Audio | One context/bank and bounded per-battle graphs integrated; 44 focused tests and normal-autoplay Chromium probe pass. Interrupted resume review finding fixed. Both synthetic cues rejected; user-supplied recording integrated with red/green source-offset and tonal-preservation proofs. Actual fetch/decode, offline menu/transition/wrap rendering, no clipping/errors and terminal departure pass; independent review clean. |
| Input | Focused pause shortcut and camera-cover regressions reproduced. Low-level guards green; controlled pause/session/viewport wiring passes combined owner/input tests; real composed journey pending deck integration. |
| Deck | Complete final capture set is undergoing unprimed critique; integration remains pending. |

Research: [declarative router](https://reactrouter.com/start/declarative/installation), [history implementation](https://github.com/remix-run/react-router/blob/main/packages/react-router/lib/router/history.ts), [Chrome autoplay](https://developer.chrome.com/blog/autoplay), [MDN autoplay](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Autoplay). Earlier map findings remain attributed; speculative implementation facts require proofs.
