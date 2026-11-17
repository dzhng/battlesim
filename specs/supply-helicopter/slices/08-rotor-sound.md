# 08: The rotor sound fades with the rotor

**Contract.** (Q6) The rotor loop's gain and pitch follow slice 06's rotor speed
down to silence. The class `air_light_logistics` (derived by
`UnitType::vehicle_class`, unchanged) has its own row in `fixtures/audio.json`,
so a supply heli no longer falls back to the truck engine.

**Seam.**
- The presentation emitter record (`crates/presentation/src/records.rs` ~334)
  gains rotor speed in its spare slot. It is filled at
  `presentation/sound.rs` (~70-94) from the slice 06 helper, and passed through
  the scripted path (`crates/presentation-wasm/src/lib.rs` ~436,
  `packages/battle-audio/src/soundFrame.ts` ~371).
- `sound/frame/loops.rs` scales the engine loop of an aircraft by rotor speed.
  An aircraft with no rotor-speed input (jets) is unchanged.
- `fixtures/audio.json`: `air_light_logistics`, copying `air_light`'s rotor
  recording.
- `web/tests/vehicleClass.test.ts` stops skipping air classes. Instead it
  asserts that every class the resolved catalog derives has an audio row. This
  is a generated guard, not a pinned list.

**Run / hear.** `heli`: the rotor bed fades as the blades stop and swells back
on "Lift off".

**Verify (test first).**
- Sound tests in `crates/presentation/tests/sound*.rs` or `packages/battle-audio`:
  rotor gain is monotone in rotor speed and zero at rest; a transport heli in
  flight is unchanged.
- The `vehicleClass` guard above.
- The `air-audio` scene or a `supply.mjs` loudness probe: silent at set-up, and
  in flight equal to the transport.

**Digest.** None in the sim. The emitter layout and an audio row change.

**Delegated.** The fade curve.

**Stays green.** `air-audio` and `sound` scenes; calibration tests.

**Feedback that would change it.** If the user wants an idle whine on the
ground. That was offered as an option in the walk and declined (Q6, option B).
