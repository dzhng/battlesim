# 04: sight, concealment, fog sweep and cloud visibility

## Contract

Smoke hides things the way D1 describes, in every rule that reads sight.
- **Sensing:** sensing and guided-weapon guidance read foliage plus smoke.
- **Concealment:** a target standing in smoke has concealment 0.1, so the HIDDEN
  flag lights up.
- **Fog sweep:** the simulation's fog sweep adds the exact chord through each
  cloud, and its cache notices clouds changing step.
- **Cloud visibility:** each side knows which clouds it can see (D12).
- **What smoke doesn't touch:** launch checks, rounds, movement and cover.

## Seam

- **Sight depth.** `WorldGeometry::sight_depth(a, b) -> f64` is
  `foliage_depth(a, b)` plus smoke depth. The smoke part samples the line every
  `FOLIAGE_SAMPLE_M` and calls `contract::smoke::piece_depth` with ground height
  at each sample, over all live clouds whether or not the map has forest.
  - These callers move to `sight_depth`:
    - `sensing::sees_point` (`sensing.rs:124`);
    - `guidance_clear` (`weapons.rs:914`);
    - the guided return-fire check (`weapons.rs:1008`).
  - These keep `foliage_depth`, and its doc becomes "tree crowns only":
    - `launch_clear` (`weapons.rs:532`);
    - `map_analysis` (`map_analysis.rs:71`);
    - mapgen.
- **Concealment.** `sensing::ground` (`sensing.rs:186-215`) takes the strongest
  of forest ground, garrison shelter and the strongest cloud covering the
  sample. `forest_ground` itself is unchanged, because it also drives movement
  speed and helicopter altitude.
- **Fog sweep.**
  - In `visibility.rs:200-231` each 8 m step adds the smoke `piece_depth` for
    that step's segment, at `sight_z`.
  - The `ObserverField` cache key (`visibility.rs:55-62`) gains
    `SmokeClouds::key_near(eye, reach)`, so only observers within reach of a
    changed phase sweep again.
- **Cloud visibility.** `sim::smoke::seen_by(field, cloud, fog_cell_m) -> bool`
  is true when the side's fog field marks any cell whose centre lies within
  `radius + fog_cell_m` of the cloud's centre (choices). A side always sees its
  own clouds.

## What you can run

`cargo test -p sim --test smoke`, the same file as slice 03, now with sight
tests.

## Tests (write them first)

These are end-to-end, with `test_smoke_*` carriers; thresholds come from the
carrier row.
- A carrier under fire pops smoke and stays put:
  - the enemy loses identification once the cloud blooms;
  - a `LastSeen` contact sits where the carrier vanished;
  - an enemy whose rounds can hit an area keeps hitting it there.
- The same carrier reverses past the cloud's edge: the enemy doesn't
  re-identify it through the cloud.
- An observer right next to the carrier, both inside the cloud, still sees it.
- From outside, an observer a few metres away sees a target at the rim; one
  farther off does not.
- The carrier's `concealed` flag (HIDDEN) is true while it stands in the cloud.
- Sight returns after the fade, and nothing of the cloud remains.
- Artillery and SAMs standing in or beside smoke still launch.
- A round fired through the cloud still hits.
- Wire- and laser-guided missiles neither launch into nor out of smoke, and an
  ATGM in flight drops for good when its line is cut. A fire-and-forget missile
  still hits.
- A carrier in smoke fires at a target its side has spotted (D9), and each shot
  refreshes its firing contact.
- **Fog sweep:**
  - a cached field equals a cold sweep at every step across a cloud's whole
    life;
  - an observer outside every cloud's reach keeps its cached field;
  - on one ray, the sweep's last visible cell agrees with `sees_point`.
- **Cloud visibility:**
  - a side behind a hill doesn't see the cloud;
  - a side in the open sees the cloud but not the unit inside it.

**Cost:** measure the simulation in instructions retired on one sample battle
with clouds, against the same battle without, and record both in
`choices.md`. This is a tripwire, not a target.

## Delegated

You may choose sampling details inside `sight_depth`, as long as the result
equals `piece_depth` summed along the line, and whether `seen_by` lives in
`sim::smoke` or next to the sweep.

## Must stay green

- Every pinned digest, since no test or capture unit carries smoke.
- The map analysis and mapgen reports, which must be byte-identical.

## Feedback that would change this slice

If the user overturns D9 (firing from inside smoke), add the rule here. It must
be expressed through data: a mount whose eye has smoke depth ≥ the full block
toward the target may not engage it.
