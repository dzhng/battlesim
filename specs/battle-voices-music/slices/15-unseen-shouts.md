# 15 — Unseen shouts (client); `cue_voices` deleted

**Contract this unlocks:** you hear unseen enemies shouting over there, muffled, from a direction. You learn nothing about who they are, what they said, or where exactly they are.

## Seam

The simulation half was built in slice 12: hearing emits an anonymous `SoundCategory::Shout` cue carrying listener, sector and band only, and slice 13 published it. This slice is the client half plus the deletions.

**Rust (owner change, 2026-10-12):** the native-parity work decides hearing cues in `crates/presentation/src/sound/frame.rs`. The table is `presentation.audio.cues` (`Cues` in `sound/tables.rs`); a cue plays by category, with a near/far gain, a far-band low-pass and a `repeat_s` limit. Add the `shout` cue kind there, through the same muffled far-band path, so one owner covers "heard but not seen".

**Client:**
- `fixtures/game.json` `presentation.audio.cues.sounds` maps `shout` to a **generic pool**: stressed barks drawn from all sets of the listener's *enemy* faction, picked at random.
- They are played through a ~600 Hz low-pass and the reverb send, at the cue's sector and band. They are not intelligible.

**Deletions:**
- `cue_voices`, which after the native-parity dedupe lives only in `crates/presentation/src/sound/synth.rs` (TS `synth.ts` is deleted by that work). Delete or replace it there; wait for the dedupe to land on `main`.
- Its `sounds.json` entry (around line 4564).
- The `default` and `infantry` rows that point at it (`game.json:870-881`).

Unseen infantry is then:
- moving: the existing `infantry_moving` footsteps cue;
- idle: silent;
- shouting: the new shout cue.

## Verification
- A sound-catalog test: nothing references `cue_voices`.
- A client test: a `shout` cue plays a muffled line from the enemy faction's pool at the cue's sector and band.

## Delegated to you
- the low-pass and reverb amounts, which the user judges by ear;
- the pool size.

## Checkpoint (non-blocking)
A skirmish with hidden enemies for the user to play.
