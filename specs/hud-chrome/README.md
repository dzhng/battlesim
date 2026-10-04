# HUD, chrome and app flow

## Next Agent Prompt

Implementation is complete in `/Users/server/dev/battlesim-hud`, branch `hud-chrome`.
The selected army roster, centered controls and whole-selection attack-move are
verified. Full native suites and 157 web files / 1,143 tests pass. Final format,
lint and types pass; lint reports three nonfatal warnings. All browser scene
failures have been corrected with their affected gates green; city visits cover
all 15 source sets, 83 templates and 664 authored state/tier tuples within the
unchanged budget. The unrestricted all-set developer city lab remains refused.

Final fresh critique inspected all 54 captures and native/2x crops. Actual-Tab
recaptures resolve pressed/focus ambiguity; no concrete visual defect remains.
Health value updates are tested, but this visual matrix has only full health bars.
Independent code reviews resolved tooltip ownership and icon hit-target findings.

Next: archive the spec using the audited rationale/choices drafts, preserve the
chosen B/bottom references, correct historical map links, audit archived claims,
commit closeout and remove integrated auxiliary worktrees. Keep the HUD worktree;
do not merge to main implicitly. Never stage unrelated skills or LFS materialization.

Global TODO:
- [x] Navigation and visit lifetime
- [x] Shared GPU, warm-up and required-resource failure
- [x] Shared audio and full supplied menu recording
- [x] Army cards above compact centered bottom controls
- [x] Screen composition and input ownership
- [ ] Archive rationale, audit final docs and commit closeout

## Contracts and scope

A navigation starts one visit. Publishing an admitted exact battle address preserves it; Main menu/Back discard it immediately. Forward or a later same-path navigation starts fresh exact inputs. Pause alone offers Resume. Use basic declarative React Router, retaining `battleLinks.ts` and the fixture registry. Unknown paths keep the existing menu fallback. No lobby, results screen, battle cache, touch gameplay, confirmation, retries or compatibility scaffolding.

The app retains its admitted GPU, page WASM/appearances and one audio context/bank. Viewports dispose scene registries, canvas configuration and listeners; battles release workers/world/map/voices/evidence. Warm only reusable resources after menu paint, never a map; kits remain demand-loaded. Required failure uses refusal and manual Reload. A lost/destroyed app device cannot be reused; real document departure tears down app resources and a BFCache restoration requests manual Reload rather than silently reviving destroyed handles.

Audio attempts unmuted playback as early as permitted and resumes on the first qualifying menu gesture under normal browser policy. Keep the restrained musical bed and subtle outdoor ambience through loading, fading when combat audio is ready. Persisted mute/master volume remain authoritative. Use the user-supplied Battlefield 2 menu recording through the existing catalog/bank pipeline, starting at its introduction; audition actual music and transition, preserving provenance.

The army deck shows ALL observed own units as vertical role/silhouette cards
with health and selection state. Full existing InfoPanel facts open above their
card on hover and keyboard focus. A small icon-only command row sits below the
cards, centered; no visible selection-count text. Empty selection preserves
the roster and hides commands; replay preserves roster/facts without commands.
Plain card click selects, Shift-click toggles. Long armies overflow horizontally,
never wrap. No visible Move/Garrison; right-click retains both. Contextual
Deploy/Pack/Leave building,  shortcuts, hearing captions
and readout occlusion remain. The earlier compact selected-facts concept is a
historical baseline, not the current design target.

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
| Input | Focused pause shortcut and camera-cover regressions reproduced. Low-level guards green; controlled pause/session/viewport wiring passes combined owner/input tests; real composed journey passes with covered inputs, restart, Back/Forward and replay. |
| Deck | Integrated `ae5e87aa`; 48 owner tests, complete28-state clean fresh review, matched production/world/terrain comparisons and reachability gates pass. Preview automation could not verify its review window. |

Research: [declarative router](https://reactrouter.com/start/declarative/installation), [history implementation](https://github.com/remix-run/react-router/blob/main/packages/react-router/lib/router/history.ts), [Chrome autoplay](https://developer.chrome.com/blog/autoplay), [MDN autoplay](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Autoplay). Earlier map findings remain attributed; speculative implementation facts require proofs.
