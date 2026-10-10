# 17b — Rotor sound

**Status:** done (catalog, prepared loop and `air_*` rows; audition by measurement, not yet by ear in the workbench or the `air` scene). **Depends on:** 06. **Owns:** D23.

**Evidence.** `soundFrame` "a hovering helicopter keeps sounding its rotor…" went red (the helicopter sounded `engine_small`, the default vehicle's), then green; `vehicleClass` "each weight of airframe has its own rotor sound row" passes. `assets.py check` passes (232 clips, 59 pinned sources). The narrow sound tests pass: `soundFrame`, `vehicleClass`, `soundCatalog`, `soundBank` (every synthesized baseline, now including `rotor`), `soundAssets`, `soundFeed`, `soundWorkbenchServer`, `soundWorkbenchEditor`; `tsc --noEmit` is clean.

**What shipped.** Source `uh60-b-roll` in `fixtures/sounds.json` (`Public-Domain-US-Gov`, sha256 pinned, the trial download kept byte for byte). Clip `rotor-uh60` is a crop of source frames 950400–1094400 (19.8–22.8 s at 48 kHz) under the `running-gear` profile. The recipe `recorded-rotor-uh60` replaces a new synthesized baseline `rotor` through `effects.rotor`. `presentation.audio.vehicles` has `air_light`, `air_medium` and `air_heavy` rows, with the rotor in their `engine` slot (see [choices](../choices.md)).

**Audition notes (measured with ffmpeg; no GUI opened).**
- *Voices and music:* none. Every stretch of the take holds its 300 Hz–3 kHz band within 0.4–0.8 dB (sd, 100 ms windows). No envelope periodicity in the 3–8 Hz syllable range is stronger than r = 0.13, and nothing tonal rises over the noise floor. No masking was needed.
- *Cuts:* the B-roll is edited at about 4.0, 8.9, 17.5 and 23.0 s. Each shows as a step of 5 dB or more in one band within 100 ms. The cut at 4.0 s is not in the brief.
- *Crop:* the stretch before the first cut (0.2–3.9 s) is steady but has no blade beat (envelope autocorrelation r ≈ 0.06), so it reads as rushing air and turbine. The 18–23 s stretch beats at 17.2 Hz (r = 0.31), which is exactly a UH-60's four blades at 258 rpm. The crop is the last 3 s before the 23 s cut. Its 20.5–20.9 s high-frequency flare (+4 dB above 4 kHz) is tamed by the profile's 5 kHz low-pass.
- *Loop seam:* the prepared loop is 139200 frames (2.900 s), exactly 50 blade passes of 58.0 ms, so the tool's 100 ms crossfade joins two tails in the same blade phase. Played twice through, the beat holds at 17.24 Hz (r = 0.32) across the seam. The sample step at the seam is 0.020, below the loop's median step of 0.035. The 20 ms level dips 1.8–4.6 dB in the crossfade, the same equal-gain crossfade dip as the tank tracks loop (3.2 dB).
- *Levels:* peak −2.9 dBFS (the tool's 0.72 headroom), RMS −11.7 dBFS, crest factor 8.9 dB. For comparison, the tanks-close tracks loop is RMS −15.0, crest 12.1. The bank's source calibration evens these out before the row gains apply.
- *Mix:* row gains are 0.45, 0.55 and 0.65 for light, medium and heavy, against a tank's 0.3–0.6 engine plus 0.45 tracks. The playback rate is 1.1, 1.0 and 0.9: a lighter rotor beats faster, and a heavier one beats slower and lower. **Open:** confirm the levels by ear in the workbench (Movement → `air_light` → Drive) and in `/lab/air-hover`.

## Contract

Helicopters sound like rotors, not tyres. The rotor bed loops while they fly and follows the existing engine-load model.

## API seam

- Prepare the recording through the [sound workbench](../../../apps/sound-workbench/README.md). Credit it and pin its hash in `fixtures/sounds.json` with a `Public-Domain-US-Gov` licence label, following the `missile-live-fire` precedent.
  - Source: "UH-60 Blackhawk B-Roll", credited to Master Sgt. Jason Stadel, 2017, <https://www.youtube.com/watch?v=vvB-8c9KX-c>.
  - The 28 s take has camera cuts at about 9 s and 23 s, so loop the steady stretch before the first cut.
- `presentation.audio.vehicles` gets `air_*` rows (the classes from slice 06) with `running` set to the rotor sound.

## What you can run or see

Audition in the sound workbench, then in the `air` scene.

## Verification

- The `sounds.json` provenance check passes.
- Audition notes go in this slice.
- If the take has voices or music, find another US-government take rather than masking them.

## Delegated to the implementer

The loop crop, the gain, and the near/far split.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Every other vehicle's sound assignment.

## Feedback that would change this slice

A different recording from the user.
