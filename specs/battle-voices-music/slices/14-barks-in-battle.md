# 14 — Barks and music in battle

**Contract this unlocks:**
- Seen shouts play in the world, heard near the camera.
- Own barks drive the music mood.
- Music ducks under voices.

**Owner change (2026-10-12):** once the native-parity work lands, which bark plays and how loud is decided in Rust (`crates/presentation/src/sound/`, through 09b's `voices.rs`); `BattleAudio` only plays. Read the seam below as the behaviour to build, placed in Rust.

## Seam

**`BattleAudio`** turns each published `Shout` into a `VoiceRequest`:
- **Tier:** `own_bark` for `Own`, `other_bark` for `Seen`.
- **Set:** from the `assignments` in `voices.json` for the shout's `kind` and the speaker's faction.
- **Position:** the shout's position. Distance to the camera ranks requests within a tier (slice 09).
- **Playback:** through the voices bus and the existing spatial path (panning, distance attenuation, distance reverb send level as for other unit sounds). The voices bus itself has no reverb send.

**Music in battle:**
- `MusicPlayer` (slice 07) plays the `tension` mood when the battle starts.
- `MusicDirector` (slice 08) is fed every **own** B5–B9 shout and crossfades between moods.
- The `menu` mood fades out as today (`appAudio.ts:162-167`).

## What the human hears
A skirmish with shouting units, the mood moving to combat when your units take fire, and the music sitting under the voices.

## Verification
- A `BattleAudio` test: published shouts become requests with the right tier and set; own B5–B9 shouts reach the director; enemy ones don't.
- One sound scene in which a shout plays and the mood changes.
- A load-cost sample: battle audio's `ready` time is unchanged with voice packs present. Packs load after `ready`.

## Delegated to you
- the spatial curve, which reuses the existing unit sound curves;
- the duck depth, within the `game.json` values.

## Must stay green
- `web/scenes/sound.mjs`;
- the `battleAudio` tests;
- slice 10's responses.

## Checkpoint (non-blocking)
A skirmish for the user to play; wait about 5 minutes, then proceed.
