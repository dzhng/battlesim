# 17b — Rotor sound

**Status:** planned. **Depends on:** 06. **Owns:** D23.

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
