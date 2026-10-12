# 09b — Chatter in Rust

**Contract this unlocks:** both the browser and the native client get their voice lines from one Rust arbiter, following the project rule "Rust as smart as possible, clients as dumb as possible".

**Waits on:** the native-parity work (branch `phase5f`) landing on `main`. It moves effect-sound decisions into `crates/presentation/src/sound/`: `frame.rs` arbitrates effect voices and emits start/glide/stop commands. The native-parity session will not port Chatter, so this port is ours.

## Seam
- `crates/presentation/src/sound/voices.rs` holds Chatter's rules exactly as they are in `packages/battle-audio/src/voices/chatter.ts`:
  - at most 4 lines;
  - priority by tier, then distance;
  - a line still waiting after 1 s is dropped;
  - a speaker never overlaps itself;
  - an order to a group collapses to one answer;
  - the take is chosen with seeded randomness.

  It emits start commands the same way `frame.rs` does. TypeScript only plays them.
- **Hold the behaviour first:** before deleting the TS, have `chatter.ts` write vectors (requests in, lines and drops out) in the pattern of `fixtures/sound-vectors.json`. Prove the Rust port reproduces them in `crates/presentation/tests/sound.rs`.
- Then delete the rules from `chatter.ts`, leaving playback only.
- The workbench's chatter preview uses the same Rust arbiter, through the presentation wasm.

## Verification
- The vectors pass in Rust.
- The TS tests for playback still pass.
- The chatter preview still runs.
- `node scene.mjs sound` passes.
