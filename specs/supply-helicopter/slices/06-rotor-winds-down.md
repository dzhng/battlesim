# 06: The rotor winds down and up (visual variable: the rotor)

**Contract.** (Q6, T1, P12) The drawn rotor speed (0 to 1) is full until
touchdown. While grounded and deploying, it eases to 0 by full set-up; while
grounded and packing, it eases back to 1 over the pack. As the speed falls, the
blur hands off to still, drawn blades with no frame where the rotor vanishes.
Own units read their own published state, and identified enemies read slice
05's fields. The simulation names no animation.

**Seam.**
- One helper in `crates/presentation` maps (grounded, progress, target, the
  type's `drop_seconds` and `seconds`) to rotor speed. It is the only rotor-speed
  calculator; slice 08's sound reads its output.
- `crates/presentation/src/units/driver/vehicle.rs` (~47-110):
  - `airborne` is recomputed every frame from `grounded`, no longer frozen at
    first sight;
  - `a.rotor` advances by `tip_mps * speed * dt`;
  - the aircraft attitude's `Flight` state resets on takeoff;
  - `gear_up` stays for life (helis never stow gear).
- `FeedUnit::grounded` (today set only by the showcase) becomes the same input,
  so the parked showcase aircraft is the zero-speed case of one rule.
- Blur handoff: `units/articulation.rs` (~126-131) and its TypeScript mirror
  `packages/scene-assets/src/articulation.ts` (~334), kept in step. Also
  `units/drawn.rs` (~213: keep drawing a grounded rotor that still turns) and
  the disc in `effects/frame/draw.rs` (~121-151).
- **Probe first.** Before writing the handoff, drive the speed from a lab URL
  parameter on a grounded test heli. Capture a strip at speeds 1.0, 0.8 … 0.0 at
  normal size and run an unprimed `screenshot-critique` ("does anything pop
  between frames?"). Pick the crossover from that, and record it here.

**Run / see.** `heli` and `heli-fire` (red view): the blades blur through the
drop, slow from touchdown, stand still once set up, and wind up on "Lift off"
before it climbs.

**Verify (test first).**
- `crates/presentation/tests/vehicle_poses.rs` or a `pose_behaviour.rs`:
  - `a_landing_rotor_slows_from_touchdown_and_stands_still_once_set_up`;
  - `a_packing_rotor_winds_up_before_it_climbs`;
  - `an_enemy_landed_heli_s_rotor_follows_its_seen_pose`;
  - `a_heli_first_seen_on_the_ground_spins_up_when_it_packs` (the first-sight freeze).
- Effect test: blur never shows on a still rotor, and a fast rotor never shows
  still blades. This extends the existing still-vs-fast rotor test.
- `supply.mjs::heli`: pixels round the mast change between ticks in flight and
  are unchanged once set up (the air-hover probe in reverse).
- **Before vs after:** run `compare-screenshots` on:
  - slice 02's frames against these at the same ticks (only the rotor differs);
  - `heli-landed` against the showcase's parked rotorcraft;
  - `heli-drop` against air-hover's in-flight blur;
  - also prove the air-hover production frame is byte-identical (helis that
    never land must not change).
- **Last check:** an unprimed `screenshot-critique` on a 4-frame strip (drop,
  touchdown, half set up, set up). Show the user inline; their review is
  non-blocking (~5 min, then decide and record here).

**Digest.** None in the sim. Presentation only.

**Delegated.** The easing curve; the blur threshold, taken from the probe.

**Stays green.** `air-hover`, `showcase`, `air-crash` scenes; presentation pose
tests.

**Feedback that would change it.** A pop at the handoff, or a spin-down that
reads too fast or too slow.
