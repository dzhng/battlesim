# Choices

Decisions made without the user while building ground admission, checked against the
shipped code. The user's own decisions (sight-supported guidance, APS still intercepts, no
compatibility or migration, the T-90M and T-15 fixes they asked for) live in the
[README](README.md). Least confident first.

## Worth your look

1. **The M10 Booker still reads as a small Abrams at play camera.** The nose was raised and
   blunted and the turret made boxier, but it keeps the US tan, and the real M10's turret
   follows Abrams styling. Two more critiques pulled opposite ways (the turret too tall vs
   taller), so iteration stopped. Alternative: a different paint, rejected because it would
   merge the M10 into the green Type 15 and CV90120.
2. **Several models were accepted short of what the last critique wanted.**
   - The Challenger 3 is nearly a Challenger 2 at mid range. The real one is the same hull
     and turret with Trophy and a new gun.
   - The T-14 is still hull-dominated, and the T-15's Kornet tubes don't show at play camera.
   - The Type 15 reads as a mid-size MBT, not clearly lighter. Its real hull is the T-90's
     length.
   - The T-90M's cheeks are blunter than the real ones.
   - The Jaguar's turret could stand taller (that needs the 2.8 m frame grown to about 2.9 m).
   - The Javelin and Akeron kits are hard to tell apart at game distance, as the TOW and
     Kornet are.

   Each needs a frame change or another art round.
3. **Every number is a guess.** No roster balance report exists: the battle sweep fields
   test units only. Balance rests on each card's small battle sample.
   - Main figures:
     - Type 15: hp 100, armour 110/60/35/25.
     - M10: hp 95, 120/55/35/20.
     - CV90120: hp 90, 110/50/30/20.
     - Centauro II: hp 90, 70/40/25/15, 30/90 km/h.
     - Jaguar: hp 80, 50/30/20/15.
     - BRM-3K: the BMP-3's hull numbers.
     - T-14: the KF51's numbers.
     - T-15: the Puma's.
     - Challenger 3: hp 135, front 250.
     - M1E3: the SEP v3's, but 30 km/h off road.
   - Costs stayed as authored, per faction.
4. **The Javelin and Akeron pierce 200 and deal 55.** That is below the 220–260 fronts but
   above the 190 front every Abrams carries (SEP v2 through M1E3), the Leopard 2A6's and
   T-80BVM's 190 and the T-72B3's 160, so those die head-on too. Every supported shot dives, so this matters only for a released,
   flat missile. At 55 a T-90M takes three roof hits and an Abrams two. Whether one roof hit
   should kill is open.
5. **The Akeron reaches 900 m and the Javelin 800 m, past the team's 600 m sight,** so the
   extra reach needs another unit's eyes. That is intended (vision is shorter than range),
   but it means the Akeron's edge shows only with recon.
6. **The Akeron team wears German Flecktarn,** because no French uniform look exists. A
   French look is its own art task.
7. **The longest tracked hull rose from 8.4 m to 9.6 m** so the real T-15 fits. The limits
   are written to sit at the roster's extremes, and every hull-at-the-limits test passes at
   the new size. It does widen what the map generator and route planner must carry.
8. **Off-centre turrets were moved onto the hull origin:** the M10 0.15 m forward, the
   CV90120 0.10 m back, and the Centauro 0.70 m forward (further forward than the photos).
   The simulation turns a carried mount about the hull origin, so an off-centre carrier gun
   fails the muzzle check. The rule was left alone. Any future off-centre turret with a
   second mount meets the same limit.

## Sound

9. **Top-attack tuning: turn 360°/s, loft 60 m, dive 40°, minimum range 100 m.** The
   sweeps show every range from 100 to 900 m striking the roof at 45° or steeper. 100 m is
   the shortest range at which every shot dove.
10. **The climb aims one run ahead, so the missile rises at the dive angle.** The first
    aim's 7° ramp read as a flat shot. One pure method, with no new state.
11. **Overhead launches needed no launch-check change:** the climb clears what the straight
    line clears.
12. **Both AT teams fire from the shoulder** (`stand_aim`; the TOW gunner kneels), with two
    AT riflemen like the TOW team; role `at`; 4 rounds at 20 supply each.
13. **The Akeron row and gunner extend the Javelin's,** so the top-attack tuning has one
    home.
14. **Roles:** the M10, CV90120, Type 15 and T-15 are `mbt`. The rules never read roles;
    the scripted defender (AT teams pick `mbt` targets, a hurt `mbt` falls back) and the
    skirmish AI's reinforcement picks do. The Centauro is `light_vehicle`, the BRM-3K and Jaguar
    `recon`. No new role.
15. **The Centauro fires the MBT gun rows,** since its 120/45 takes tank ammunition.
16. **Frames take the art's measure** where the photos allow: the Centauro is
    8.12 × 3.16 × 2.97 m, the CV90120's box 2.65 m, the M1E3 keeps the inherited 2.44 m.
    Sights above the box are dressing.
17. **Wheeled edge wear is 0.6, like the VBCI and Boxer** (the Humvee keeps the 0.35 default). The Jaguar's pale
    patches came from the old 1.0.
18. **The Challenger 3 and M1E3 extend their live siblings,** for fewer overrides. Trophy
    sits on the M1E3's turret sides via an `at` argument on `abrams.trophy_station`.
19. **The CV90120 references are its own photos** (CC-BY-SA-3.0, front and cheeks). No
    licensable side or rear photo exists.
20. **The ambush lab's top-attack variant flies the Javelin row's tuning** rather than its
    own copy, and the lab's guidance line stays straight.

## Left as found (not this feature's)

- The ATGM launch-flash sprite has hard edges and no backblast (the TOW shares it). At play
  camera nothing shows a missile's height (no shadow or drop line).
- AT teams (TOW and Kornet as well) stall in Aiming at some close target layouts.
- Three Trophy station builders exist (`abrams.py`, `leopard.py`, inline in
  `challenger_3.py`). One shared builder would mean re-exporting three families.
- The mount frame key is `name` but holds the catalog mount id
  (`catalog_frames._variant`, `vehicle_export.rig`).
- `armata.py` still keys its art by T-14 and T-15 card id.
- The `unit-roster` scene's ghost check fails on the helicopter feature's Apache wreck
  (fog keeps 84% of its pixels stable against a 90% bar).
- Panel baselines predate the helicopter feature's `test_heli` card (it wraps the idle and
  contact panels to a second row), and several UI baselines differ by 1–5 pixels on this
  machine; neither is this feature's.
