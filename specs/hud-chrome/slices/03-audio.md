# 03 · Shared audio and menu bed

Contract: one page audio context/bank survives menu → loading → battle. Each battle has fresh SoundFrame evidence/voices; exit removes battle voices/nodes without closing page audio. Music plus subtle outdoor ambience prepares/attempts early when unmuted; first permitted menu gesture resumes blocked context without another in-game click. Keep music through loading, fade when battle audio is ready, restore it on menu return. Existing mute/master volume and source calibration stay authoritative.

Seam: battle-audio owns app audio lifetime and per-battle SoundFrame; existing bank/catalog/synth/settings remain owners. Reuse or explicitly dispose mixer graphs, no retained orphan limiters/reverbs/panners. No visit cancellation attached to shared bank work. Own audio package, fixtures/sounds.json, soundFeed and an app-audio React integration surface; root router wiring is coordinated. The user-supplied menu recording uses the existing catalog/preparation/bank path; no new playback service.

First proof: exiting/recreating battle observers on one page retains prepared buffers/context while old voices/evidence disappear. Then blocked resume, first gesture, mute/unmute, disposal during prepare, real pagehide/BFCache and transition. Use existing battleAudio/soundBank/soundFrame/catalog/asset checks; do not loosen contracts. Generate an offline listening WAV plus transition sample, report loudness/seam/clipping and offer non-blocking audition. Browser proof uses fresh isolated profile and normal autoplay policy, silently capturing output.

Delegated: musical notes/timing and initial restrained mix, reversible after listening review; exact app-vs-battle graph implementation under bounded lifecycle proof. Playable artifact is the real menu with continuous audio into battle. Root may integrate public app-audio component after navigation commit.

Verification: write-tests red/green at the consumer seam, narrow types/lint/format, review and independent code review before commit, then audit choices. Preserve exact battle identities, deterministic replay outcomes, side-visible authority and manual mechanics reload. Scratch evidence remains ignored.

## Implementation checkpoint · 2026-10-04

Implemented in branch `hud-audio`; production router/session wiring is owned by
the coordinating pass. `AppAudioProvider`, required `useAppAudio` and
`AppAudioScreen` live in `apps/battle-lab/src/AppAudio.tsx`. Screen policy is
`menu` (start/restore bed), `loading` (retain it until the first live battle
update) or `other` (no menu bed). `createBattleAudio(tickHz, appAudio)` consumes
the retained owner; battle disposal disconnects its full mixer and tails.

Focused audio checks pass. The repeated-visit graph test is proven red when
mixer disconnection is removed. Independent Codex review found interrupted
context resumption retaining stale evidence; a red/green regression now proves
resumption clears it. Quick mute/unmute also has a red/green suspension-race
regression. Context admission failure stays explicit without repeated attempts.
Real pagehide leaves audio terminal; BFCache Reload is the root boundary's job.

Offline listening and transition WAVs, browser probe and measurements live in
ignored `throwaway/audio-review/`. A fresh isolated Chromium profile, normal
autoplay policy and muted output prove preparation before permission and
activation from the first menu click, with no browser errors. No full check,
full verify or live playback in the user's browser was run.

The user supplied `menu.mp3` after rejecting both synthesized candidates.
The entire recording now owns the `menu_music` recipe. The pinned source and
prepared mono loop preserve attribution, full duration and tonal balance through
the existing preparation path; a 100 ms crossfade softens the repeat. The rejected
cue and its helpers are deleted. Music starts at its introduction; repeated
ambience/engine loops retain their staggered phases. The music mix is raised,
while outdoor ambience stays quiet. No additional music player or scheduler exists.

Recording proof: catalog admission, music tonal preservation and source-offset
tracers reproduced red then green. Focused asset/bank/app/battle checks pass.
Actual browser fetch/decode and offline menu/transition/wrap rendering pass.
Samples have no clipped frames or browser errors; the full-track raw repeat seam
is 0.000122. This Chromium profile permitted immediate audio despite its normal
activation policy; first input preserved the running context. The blocked-permission
unit proof remains green. Pagehide/BFCache stays terminal. Independent review found
no actionable defect and reproduced the exact prepared bytes and hashes.
Production integration and full-spec closeout remain with the coordinating pass.
