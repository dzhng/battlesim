# HUD, chrome and app flow

## Next Agent Prompt

Status: implementation started 2026-10-04 in `/Users/server/dev/battlesim-hud`, branch `hud-chrome`, based on `86c8c863`. The mechanics refresh cleanup is already on main. Read this handoff and the owning slice before editing; preserve the [exploration decisions](unknowns-map.html). Start [01 navigation](slices/01-navigation.md), [03 audio](slices/03-audio.md), and [04 deck](slices/04-deck.md) independently. The coordinating agent owns [02 resources](slices/02-resources.md) and integration. Do not repeat the interview. Update this pickup and the evidence table before each checkpoint ends.

Global TODO:
- [ ] [01 navigation and visit lifetime](slices/01-navigation.md)
- [ ] [02 shared GPU, warm-up and required-resource failure](slices/02-resources.md)
- [ ] [03 shared audio and menu atmosphere](slices/03-audio.md)
- [ ] [04 joined compact deck and icon commands](slices/04-deck.md)
- [ ] [05 current screen composition and input ownership](slices/05-screens.md)
- [ ] [06 integration and closeout](slices/06-closeout.md)

No external blocker. Menu/music taste checkpoints are reversible and non-blocking. Scratch output goes in ignored `throwaway/`; only the frozen concept reference belongs in spec assets. Dependencies share main's installed tree; every checkout has its own generated/build output. All GPU work uses `/Users/server/dev/battlesim/throwaway/gpu.lock`.

## Contracts and scope

A navigation starts one visit. Publishing an admitted exact battle address preserves it; Main menu/Back discard it immediately. Forward or a later same-path navigation starts fresh exact inputs. Pause alone offers Resume. Use basic declarative React Router, retaining `battleLinks.ts` and the fixture registry. Unknown paths keep the existing menu fallback. No lobby, results screen, battle cache, touch gameplay, confirmation, retries or compatibility scaffolding.

The app retains its admitted GPU, page WASM/appearances and one audio context/bank. Viewports dispose scene registries, canvas configuration and listeners; battles release workers/world/map/voices/evidence. Warm only reusable resources after menu paint, never a map; kits remain demand-loaded. Required failure uses refusal and manual Reload. A lost/destroyed app device cannot be reused; real document departure tears down app resources and a BFCache restoration requests manual Reload rather than silently reviving destroyed handles.

Audio attempts unmuted playback as early as permitted and resumes on the first qualifying menu gesture under normal browser policy. Keep the restrained musical bed and subtle outdoor ambience through loading, fading when combat audio is ready. Persisted mute/master volume remain authoritative. Own/generate the minimal musical loop through the existing synthesis/catalog/bank pipeline; audition actual music and transition, preserving provenance.

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
| Implementation | Not started. |

Research: [declarative router](https://reactrouter.com/start/declarative/installation), [history implementation](https://github.com/remix-run/react-router/blob/main/packages/react-router/lib/router/history.ts), [Chrome autoplay](https://developer.chrome.com/blog/autoplay), [MDN autoplay](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Autoplay). Earlier map findings remain attributed; speculative implementation facts require proofs.
