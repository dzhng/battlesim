# Choices

Decisions an implementing pass made where its slice was silent. One entry each: date,
slice, the decision, the evidence, the alternative it rejected. Planning decisions live in
the [README](README.md#decisions).

## 2026-10-10 — slices 02–03 (top attack)

- **Top-attack rows turn fast (300–360°/s), not at the generic 60°/s.** At 60°/s the turn radius
  (about 190 m) is too wide to pull into the dive cone, so the missile loops or ploughs short
  (`assets/top-attack/sweep-slow-turn.txt`). At 360°/s, loft 60 m and dive 40°, every range from
  100 to 900 m hits the roof at 46° or steeper (`sweep-fast-turn.txt`). Rejected: a stateless
  glide-path aim, which never looped but struck at only 15–30°.
- **`dive_deg` is the minimum impact angle;** real impacts land between it and about twice it.
- **A released missile drops its loft** and goes to ground at its fixed point, as the contract says.
- **Overhead launch needed no launch-check change.** A garrisoned launcher's rounds already pass
  its own building, crowns only block sight and trunks are upright, so the climb clears what the
  straight launch line clears (tests in `garrison.rs`, `guidance.rs`). If a lofted launch check is
  ever added, the loft must join the launch-solve cache key (`solve_launch_past`).
- **Refusal tests live in `top_attack.rs`**, not `flight_load.rs` (that file is the load report).

## 2026-10-10 — slice 05 (Challenger 3 pilot)

- **Challenger 3 extends the Challenger 2 TES record,** not the advanced-MBT profile: fewer overrides
  (frame, mounts, mobility, HMG inherited). Sets hp 135, armour 250/125/45 (front/side/roof),
  advanced tank rounds, 600 m sight, `trophy_aps`; mobility stays 24/55 km/h (same powerpack).
  Numbers are guesses for the closeout balance report.
- **Accepted: at mid range the Challenger 3 reads nearly as a Challenger 2.** Two critique passes
  ranked "turret too low"; the turret is already raised to the shared 2.49 m box top. The real
  vehicle is the same hull and turret with Trophy and a new gun, so a taller turret would mean
  changing a frame the two share for a difference the real tanks don't have.
- **Wreck footprint 1.7 m** (fallen skirts lie 1.63 m beside the hull; the KF51 uses 1.6).
- **Paint stays British green** like its live peer, though the photos show grey and sand.
- **Shared helper fix:** `catalog_frames._variant` read mount labels as ids, so every live `run`
  export failed; it now keys mounts by id (Challenger 2 TES re-export byte-identical).
- **Trophy hardware became named nodes with meshes beneath,** as on the Abrams: plain named boxes
  merged into the turret in tiering and failed `fit.part_nodes`.

## 2026-10-10 — slice 04 (top-attack view)

- **The climb aims one climb's run ahead at the loft height,** so the missile rises at the dive
  angle (39° at dive 40), holds, then dives. The first aim climbed on a ~7° ramp at 500 m and
  read as a flat shot at play camera (`assets/top-attack/view-flat-before-after.png`). Rejected:
  a higher loft (180 m still gave 11° at 900 m) and storing the launch origin (not needed).
  Every range 100–900 m still strikes the roof at 45° or steeper.
- **Starting rows for slice 11:** turn 360°/s, loft 60 m, dive 40° (never below 300°/s at dive 40).
- **The lab guidance line stays straight to the commanded point** (game-ui): drawing the flown
  path would copy the dive geometry into the renderer.
- **Open, not this feature's:** at the play camera nothing shows height (no missile shadow or
  drop line), so the rise reads mainly through the hook onto the tank. That is missile art.
- **The ambush lab gained a "Top attack" variant** (saved encounter with pinned rules) rather
  than a new route.

## 2026-10-10 — slice 10 (infantry kits)

- **The Javelin's European look is Flecktarn** (the only European look; Europe's kits wear it).
- **Flagged: the Akeron team wears Flecktarn, not a French uniform.** `infantry-appearances.json` has
  no French look and there is no French uniform print. A French look is its own art task.
- **Guard looks are the army's recon patrol,** following the TOW and Kornet teams.
- **Carried kits show the launcher slung on the back;** a rifle-only carried look read as a rifleman.
- **The Javelin sight unit is dark,** not tan: tan vanished against the tan uniform.
- **Accepted after the last critique:** at game distance the Javelin, Akeron and the two Javelin
  uniforms are hard to tell apart (TOW and Kornet share that limit); the slung tube's lower end stands
  off the back; the carried Akeron tube looks tan in strong light.
- **Tooling:** `asset sheet --clips` sheets a loose skinned GLB; `infantry_equipment.py <kit> --receipt`
  writes a kit's receipt once every look is exported.

## 2026-10-10 — slice 11 (publication contract change)

- **A contact's `heard` weapon-row bitmask is published as an exact limb pair, and the weapon-row
  cap rises from 24 to 32.** `heard` went out as one f32, exact only to 2^24, so rules refused a 25th
  row; `javelin` and `akeron_mp` make 25. The contacts group gains one field (`heardLo`/`heardHi`,
  the encoding body ids already use). Digests are unchanged (`heard` was already hashed as a u32).
  Rejected: one shared top-attack row for both teams, which would make the Akeron a copy of the
  Javelin and leave no headroom (the helicopter spec adds rows next).

## 2026-10-10 — slice 09 (M1E3)

- **Extends the SEP v3 Trophy record** (two overrides: the HMG pivot on its mid-roof pedestal, and
  30 km/h off road against 27, as the M1E3 is meant to be lighter; a guess). Armour stays at SEP v3
  level (front 190), below the Challenger 3's 250; the closeout balance report tunes it.
- **Kept the inherited 2.44 m hull box,** not the old 2.30 m: the art fits it without moving roof
  gear under dressing.
- **Mount id stays `HMG`** (not `RWS`) so the appearance mount map matches its peers.
- **Trophy sits on the flat turret sides** (radar at x −1.10, y ±1.70), via a new `at` argument on
  `abrams.trophy_station`; the old rear-corner spot would overhang the short turret. The Abrams family
  re-exports byte-identical.
- **Dropped:** the critique's smoke-rack bracket and lighter turret face (outside slice 01's ranking).

## 2026-10-10 — slice 07 (BRM-3K, Type 15)

- **Type 15: deck lowered 1.56 → 1.42 m, turret raised to about 2.42 m;** reshaping the turret alone
  still read as a CV90120. Accepted: it reads as a mid-size MBT, not clearly lighter than a T-90M —
  its real hull is the T-90's length, so a smaller model would mean a false frame.
- **Type 15 numbers:** 100 hp, armour 110/60/35/25, 30/70 km/h, medium weight, eye 2.44 m. Role
  `mbt`: no light-tank role exists and nothing in the simulation reads roles.
- **BRM-3K takes the BMP-3's hull numbers** (90 hp, 75/40/25/20, 28/65 km/h), eye 2.3 m, role
  `recon`, cost 180 unchanged.
- **Wreck footprint tolerances** 0.8 m (BRM-3K) and 1.0 m (Type 15), from thrown tracks and turrets.
- **Planning error corrected:** the recipe named the catalog smoke test as each card's proof, but it
  walks test units only by design. Closeout runs it over the twelve as a scratch check.
- **Sample:** at 450 m the Type 15 kills IFVs and dies to MBTs head-on (`assets/07/duels.txt`).
