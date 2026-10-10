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
