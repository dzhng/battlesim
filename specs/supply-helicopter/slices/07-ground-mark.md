# 07: Landed, it wears the ground mark (visual variable: the marker)

**Contract.** (T1) While grounded, the heli's battlefield marker is the ground
vehicle's: no air ring, no dotted drop line. Click rings and the always-shown
aircraft marker follow the same per-unit fact. In the air, nothing changes.

**Seam.** The client sites that decide "airborne" from the type read
`grounded`:
- `apps/battle-lab/src/battleOverlay.ts` (~336 `orderView`, ~489 `aircraftLayer`);
- `apps/battle-lab/src/sideInstances.ts` (~124 enemy click rings);
- `packages/battle-renderer/src/orderOverlay.ts` (~239-249, ~416, ~427, ~470,
  ~516, ~652-666) and `unitCircle.ts` (~124), through the view they're given.

Before closing, grep `airborne` across `web/`, `apps/`, `packages/` and
`crates/presentation`. Every type-keyed hit is classified as either "is an
aircraft type" (stays) or "in the air now" (reads `grounded`).

**Run / see.** `heli`: at touchdown the drop line and air ring give way to the
ground marker the truck wears. Selected, it shows the truck's reach ring.

**Verify (test first).**
- `web/tests/` (the aircraft-marks test): a grounded rotor aircraft gets no drop
  line and one ground marker; an airborne one is as before.
- `supply.mjs::heli`: drop-line count before and after touchdown; frames
  `marks-air` and `marks-landed`.
- `compare-screenshots`: `marks-landed` against the truck's `serving` frame
  (same marker family, same zoom), and slice 06's `heli-landed` against
  `marks-landed` (only the marker differs).
- **Last check:** an unprimed `screenshot-critique` on `marks-landed`. Show the
  user inline; non-blocking.

**Digest.** None in the sim.

**Delegated.** Whether the drop line fades over the last metre or cuts at
touchdown.

**Stays green.** `air-hover`, `air-hud`, `cursor` scenes; the overlay tests.

**Feedback that would change it.** If the user wants the heli identity kept on
the ground marker.
