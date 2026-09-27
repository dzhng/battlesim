# 40 — Sound

**Status:** done (see `choices.md`, "Slice 40"; slice 39 not built yet, so reverse whine is a hook that defaults off). **Depends on:** 25, 26, 32, 39 (effects, movement and drive exist). **Lane:** presentation (audio). The last slice before 27 (user 2026-09-26).

## Contract

Give the battle sound, in three families (user):
- **Units:** infantry footsteps and kit, speech barks optional; vehicle engines at idle and under load, tracks and wheels, turret traverse, reverse whine.
- **Environment:** a summer countryside bed (wind, birds, insects, leaves), fire crackle and roar from burning wrecks, smoke rumble.
- **Effects:** gunfire per weapon kind, near and far; explosions, impacts per hit kind, ricochet whines, missile motor.

Sound must respect fog of war exactly as the picture does. It plays only from what the side's observation contains: its own units, identified enemies, published segments, blasts and known wrecks. **Unseen enemies are heard only through the sim's existing hearing cues** (`crates/sim/src/hearing.rs`, `ObservationFrame.audible`: a broad direction and distance band, never a position), played as vague sounds from that direction and band, never pinned to a spot.

## API seam

- `packages/battle-audio` (new, the one owner): Web Audio graph, listener = camera, positional panners with distance attenuation, and an air-absorption low-pass on far sounds; voice pool with priorities and a budget; master, units, effects and ambience buses. Presentation clock and pause from `TickInterpolator.time(now)`, so pause silences transients and holds loops.
- Sound derives from the same feed the visuals use: slice 25's `EffectPublication` (shots, segments, blasts), slice 26's smoke sources (fires), the pose driver's vehicle and soldier motion (engines, footsteps), and `audible` cues (the unseen). No second feed.
- Assets: CC0 only (for example Kenney audio, or freesound CC0), or project-made or synthesised. Each file is a reuse-manifest `third_party` entry with its sha256. Acceptance of each new source's licence is asked of the user non-blockingly. Assets are small Ogg/Opus files under `assets/audio/` (LFS), loaded through a small catalog.
- Fixture `presentation.audio`: bus levels, distance curves, budgets, and per-kind sound tables keyed like `presentation.effects`.
- Autoplay: audio starts on the first user gesture (the browser rule). The main menu and battle have a mute toggle and a master volume.

## Verification

- Web tests:
  - sounds only for events in the observation (metamorphic: an unseen enemy's shot makes no positional sound, only its audible cue);
  - the voice budget holds under a 100-a-side firefight;
  - pause silences transients;
  - reset clears.
- A scene renders the audio graph offline (`OfflineAudioContext`) for a scripted firefight. It checks non-silence, no clipping (peak below 0 dBFS), and levels per bus. It writes a short WAV to evidence for the agent to review.
- The agent listens only through measurements: spectra, level curves and onset alignment with the visual events. The user listens last, in the non-blocking checkpoint, where the agent attaches the WAV and a short screen capture.
- CPU cost recorded (audio thread and main-thread scheduling) in `frame-cost.md`.

## Decision budget

- **Delegated:** sound choice and mix, within the fixture; which CC0 sources.
- **Not delegated:** fog-of-war rules for sound (above), and CC0 or project-owned only.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Every existing scene and test. `bun run check` and `bun run verify` at closeout.
