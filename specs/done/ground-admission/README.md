# Ground admission and top-attack missiles

Twelve roster cards that needed no new movement layer are playable: the T-14 and T-15
Armata, BRM-3K, Type 15, M10 Booker, CV90120, Centauro II, Challenger 3, M1E3, EBRC
Jaguar, and the Javelin and Akeron MP teams. Each has catalog physics and a model judged
in battle beside its live peers. Three of them fire a new kind of missile: one that
shoots up, holds its height and dives onto the roof. Fixed-wing aircraft, drones,
artillery and air defence stay disabled behind their own mechanics; helicopters were
admitted by their own feature.

## Why top attack works this way

**The path is a property of the weapon row, steered without state.** A guided row may
carry `top_attack { loft_m, dive_deg }` (`TopAttack`, `crates/contract/src/ballistics.rs`).
`FlightConfig::profile` (`crates/sim/src/flight/mod.rs`) is the only admission: it refuses
the field on an unguided row, with a loft that is not positive, or with a dive not between
level and vertical, and resolves it to `Loft`. `Guidance::aim` is the only code
that knows the geometry. While the launcher supports the missile, it aims at the loft
height one climb's run (`height_m / slope`) ahead, never past the commanded point. So the
missile rises at the dive angle, holds the loft, and dives once the point lies `dive_deg`
below its horizon. `Flight::acceleration` steers at `aim`, so flight and the APS look-ahead
see one path. A missile fired too close for a full climb aims straight at the lofted point
above its target, so it flies a lower arc without a separate rule.

**The roof is struck by geometry, not by rule.** Which face a round hits comes from where
it hits (`face_toward`, `crates/sim/src/units.rs`). Missiles detonate on contact and
pierce by that face's armour (`meet_hull`, `pierces`, `crates/sim/src/damage.rs`). Nothing
in battle, damage, protection, publication or the renderer asks whether a round is top
attack.

**Guidance and protection are unchanged, by the user's choice.** The gunner must keep
sight: the launcher renews the commanded point, and moving, Stop or lost sight releases
the missile, which then drops its loft and flies at its fixed point. Fire-and-forget was
offered and declined (a second mechanic that makes AT teams much stronger). Trophy-style
APS (`capabilities.active_protection`) still intercepts a diving missile. Top attack wins
by finding thin roof armour, not by beating APS. An APS elevation arc was offered and
declined (a third mechanic that rebalances every Trophy unit).

**Top attack must pay off.** Before this work the generic `atgm` pierced 300 against a
thickest front of 260, so a dive changed nothing. The `javelin` row (and `akeron_mp`, which
extends it) pierces 200. That is below the 220–260 fronts but still above the 190 front of
every Abrams, the Leopard 2A6 and the T-80BVM, and far above every roof. A Javelin kills a T-90M
by diving onto it ([sample](assets/11-at-teams/battle-sample.txt)).

## Invariants

- A top-attack row turns fast (the shipped row: 360°/s). At the generic 60°/s the turn radius
  (about 190 m) cannot pull into the dive cone, so the missile loops or ploughs short
  ([slow sweep](assets/top-attack/sweep-slow-turn.txt),
  [fast sweep](assets/top-attack/sweep-fast-turn.txt)). Tune the row's `turn_deg_s`,
  `loft_m` and `dive_deg`; never add a phase or a special case. `dive_deg` is the
  minimum impact angle.
- The loft enters the digest only when present (`Projectiles` digest,
  `crates/sim/src/flight/mod.rs`). A battle firing no top-attack row keeps its digest.
- `GuidedMissile.point` (`crates/contract/src/observation.rs`) and `GuidedView.point`
  (`web/src/battle/sim/observation.ts`) are the commanded point, not where a lofted missile
  is heading. Presentation never recomputes the dive. The lab guidance line stays straight
  to the commanded point.
- A top-attack launch is checked along the straight launch line (`solve_launch_past`,
  `crates/sim/src/flight/solve.rs`), and the climb clears what that line clears: a
  garrisoned launcher's rounds pass its own building, crowns block only sight, and trunks
  stand upright. If a lofted launch check is ever added, the loft joins that solve's cache
  key.
- Weapon rows are numbered in name order. Adding a row shifts the bits of a contact's
  `heard` mask, so combat digests and the paired publication records move, while fog does
  not. Admitting a unit type grows the published kind list, which moves the
  publication hash but not the digest. Re-record (`BLESS_PARITY`) rather than hand-merge.

Tests: `crates/sim/tests/top_attack.rs` (the aim table, refusals, roof dive, front hit
when flown direct, short and maximum range, digest). Top-attack cases in `guidance.rs`
(payoff, release, gunner handoff), `protection.rs` (Trophy intercepts a dive) and
`garrison.rs` (a garrisoned launcher).

## How a card is admitted

A roster record owns a live card's physical frame: hull, eye and mounts, with `extends`
naming a live sibling where the frame is shared, otherwise a profile in
`fixtures/units/ground/profiles.json`. `assets/catalog.json` owns its appearance and wreck.
`fixtures/units/model-manifest.json` lists disabled cards only. The family script exports
through `vehicle_export.run`, which reads the frame from `fixtures/catalog.json` through
`catalog_frames.family_variants`. So the catalog is regenerated before any export, and the
script keeps no frame of its own. A source folder equals its family name, and one script
maps to one folder (`armata.py` is the shared platform; `t14.py` and `t15.py` export).
`asset validate --type` holds the art to the live 0.1 m fit (the disabled export allowed 6%).
Part hardware such as Trophy hangs under named empties, or tiering merges it away.

By the catalog tests' design no committed test walks the faction roster. The smoke test
walks test units, so an admission is proven by resolving it, and the twelve were proven by
a scratch run of the smoke routine over the admitted cards: every mount fires, every card
reaches its goal.

Rules that changed: the longest tracked hull is 9.6 m (`hull_limits`, half length 4.8 m)
so the real 9.5 m T-15 fits. The new `light_tank` and `wheeled_tank_destroyer` profiles
carry sensors and sound only. A turreted carrier's gun pivots on the hull origin, because
the simulation turns a carried mount about it, so off-centre turrets were moved (the
Centauro's 0.70 m forward).

## Dead ends

- **Aiming at the loft above the far target.** The first steering climbed on a ~7° ramp at
  500 m, which reads as a flat shot at play camera
  ([before and after](assets/top-attack/view-flat-before-after.png)). A higher loft only
  steepened short shots (180 m still gave 11° at 900 m).
- **A glide-path aim.** It never looped, but struck at only 15–30°.
- **Explicit climb/cruise/dive phase state.** It would have added digest state and
  transitions for the same picture.
- **Widening the `heard` mask in this feature.** The 25th weapon row needed it. The
  helicopter work landed the same widening first (two 24-bit words, 48 rows).
- **A shared top-attack row for Javelin and Akeron.** It left no headroom and made the
  Akeron a copy. Instead `akeron_mp` extends `javelin` and differs in reach.
- **Shrinking the T-15 to the old 8.4 m hull limit.** A false frame, rejected for the
  limit rise.

## Visual provenance

Every model was judged at play camera beside a live peer and against its committed
reference photos (`assets/references/<family>/`). An unprimed critique was the last check.
The bar was calibrated against a control: the same critique on the live T-90M also said
"does not read as a T-90M". So a finding counted only where a card fell below its peer.

- [Triage montages](assets/triage/): every card before admission beside its peers. They
  drove the art-first passes: turrets too small for their hulls (T-14, T-15, BRM-3K,
  CV90120, Jaguar), the Type 15/CV90120 confusion and the M10's Abrams look.
- **User review**, in two zoomed crops flagged by the user:
  - [T-15 rear](assets/triage/t15-rear-zoom.png): the rear must rise to full height. Result:
    [before, after and reference](assets/06/t15-rear-zoom.png).
  - [T-90M turret](assets/triage/t90-turret-zoom.png): it read as a casemate slab. Results:
    [first pass](assets/06/t90-turret-zoom.png), [second pass](assets/06/t90-pass2-qfront.png)
    and [side](assets/06/t90-pass2-side.png).
  - The T-90M fix was in the turret: the photos put its skirt line at or above the
    model's, so lowering the hull was rejected.
- Per-card before, after and peer sheets: [05](assets/05/) (Challenger 3),
  [06](assets/06/) (Armata), [07](assets/07/) (BRM-3K, Type 15),
  [08](assets/08/) (M10, CV90120, Centauro II), [09](assets/09/) (M1E3 Trophy),
  [10](assets/10-infantry-kits/) (Javelin and Akeron kits),
  [12](assets/12/) (Jaguar).
- The top-attack picture: [launch](assets/top-attack/view-launch.png),
  [apex](assets/top-attack/view-apex.png), [impact](assets/top-attack/view-impact.png),
  [flight](assets/top-attack/view-flight.gif), and the
  [teams firing](assets/11-at-teams/) beside the TOW. They drove the change to the aim.
  The source for the real profile is the Javelin's roughly 150 m top-attack apex
  ([Wikipedia](https://en.wikipedia.org/wiki/FGM-148_Javelin),
  [GlobalSecurity](https://www.globalsecurity.org/military/systems/munitions/javelin-design.htm)).
  Game ranges are compressed, so the loft is tuned to read on screen (60 m), not copied.

[choices.md](choices.md) is the ledger of decisions made without the user, including what
was accepted short of the critique's wishes.
