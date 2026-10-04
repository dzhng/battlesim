# 03 · Shared audio and menu bed

Contract: one page audio context/bank survives menu → loading → battle. Each battle has fresh SoundFrame evidence/voices; exit removes battle voices/nodes without closing page audio. Music plus subtle outdoor ambience prepares/attempts early when unmuted; first permitted menu gesture resumes blocked context without another in-game click. Keep music through loading, fade when battle audio is ready, restore it on menu return. Existing mute/master volume and source calibration stay authoritative.

Seam: battle-audio owns app audio lifetime and per-battle SoundFrame; existing bank/catalog/synth/settings remain owners. Reuse or explicitly dispose mixer graphs, no retained orphan limiters/reverbs/panners. No visit cancellation attached to shared bank work. Own audio package, fixtures/sounds.json, soundFeed and an app-audio React integration surface; root router wiring is coordinated. A small authored synthesis loop in existing synth/catalog pipeline is the reversible source choice; no third-party provenance burden or new playback service.

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

Music tone/mix is **not accepted**: the user requested louder music and a different
tone after the first listening sample. The coordinating pass directed a wholly
original military-score mood: low brass harmony, a small horn phrase, restrained
field percussion and a stronger pulse. The revised cue uses the existing
synth/catalog recipe and outdoor ambience. It introduces no imported material,
music scheduler or new playback owner. Offline 30-second menu and 20-second
transition WAVs are ready in `throwaway/audio-review/`; the original drone mix
is retained there for comparison. Continue integration and offer the new sample
for the non-blocking listening checkpoint.

The original-score revision's independent review found nonzero percussion tails
being truncated. Drum/snare buffers now fade their final portion to zero before
loop mixing. The output is finite and deterministic, and preparation remains a
bounded, once-per-bank synthesis task.
