# Smoke screen: final choices ledger

These are the choices the build made where the plan was silent, or where it
departed from the plan. The user's own decisions are in
[`decisions.md`](decisions.md) and are not repeated here. Choices marked
"coordinator" were made by the session that ran the build, not by a slice's
implementer.

This ledger replaces the per-pass one. Every entry was re-checked against the
shipped code on 2026-11-13. Some choices were superseded and are dropped here:
- the sound's "a look with no sound pops silently";
- slice 07's "puffs never lift the fog veil";
- the drawn cloud's growth from nothing;
- the scripted pop in the smoke lab;
- the lab's frame-cost workload;
- the unit-roster scene's own wait for the first observation, which staging
  already had;
- the closeout's "no picture re-blessed".

**Review these first:**
1. **N1:** the pop and hiss sounds were picked from waveforms; nobody has
   listened to them.
2. **U1:** three browser scenes that fail on staging too were left failing.
3. **S1:** a cloud's id counts every pop in the battle, including ones a side
   never saw.

**Counts:** 1 needs your call, 1 unsound, 41 sound (9 at medium confidence,
32 at high). Seven small housekeeping calls are listed in one line at the end.

---

## Needs your call

### N1. The pop sounds were chosen by reading waveforms, not by listening

- **When:** slice 10 (`bbbd7beeb`).
- **The choice:** A tank pops smoke. You hear its smoke dischargers fire, two
  thumps, then the hiss of the cloud. A squad's smoke is a grenade pop, then a
  hiss. Both sounds were generated with ElevenLabs, three takes each. No one
  could listen during the build, so the takes were judged by their shape: a
  loudness curve every 10 ms, where each sharp onset falls, a spectrum and a
  spectrogram.
  - **`smoke-discharger-3`** was kept. Its two launch thumps are at full level
    (0.08 s and 0.41 s), with low-frequency body. A broadband hiss follows at
    1.4 s and fades out by 5 s. Of the other takes, take 1's thumps sit 18 dB
    under its hiss, and take 2 is one boom with no hiss.
  - **`smoke-grenade-1`** was kept. It has a pin ping and a sharp pop (0.07 s,
    0.25 s), then a steady high hiss from 1.0 s. Of the other takes, take 2 is
    a boom with a reverberant tail, and take 3 only starts hissing at 2.9 s.
  - No take shows a voice or a musical line.
  - Each sound is a whole take, uncropped, through the flare burst's `launch`
    processing.
  - Both kept takes leave about a second between the launch and the hiss. The
    cloud also takes a second to bloom, so the gap may read as natural, or as
    a pause.
- **The gap:** slice 10 asked for listening in the sound workbench and the lab
  before choosing.
- **The reach:** every smoke pop in every battle plays these two sounds. All six
  takes are clips in `fixtures/sounds.json`, so swapping one is a one-word edit
  to the `smoke-discharger` or `smoke-grenade` recipe's `clips`.
- **Verdict:** needs your call: only a person can judge a sound.
  **Provisional call: keep the takes.** To reverse, audition the six clips in
  the sound workbench and name another in the recipe. If the gap before the
  hiss bothers you, the first thing to try is a two-layer design: a launch clip
  and a hiss clip, with the hiss started earlier.
- **Confidence:** low.

---

## Unsound

### U1. Three browser scenes that also fail on staging were left failing

- **When:** the closeout run (`d76357684`), re-checked after merging staging.
- **The choice:** The closeout ran every browser scene. Three failed, and each
  fails the same way, with the same reading, at the branch's base on staging:
  - **`geometry`:** the traversal overlay's "steep ramp" pixel reads
    rgb 44,13,6. The reference colour grade's deeper shadows darken the
    overlay's shaded face.
  - **`fog-look`:** "units standing in fog are never fogged" reads a soldier's
    darkest pixel at 25,29,2. The grade again: the check can't tell fog from
    shadow.
  - **`air-audio`:** the J-10 is never seen again within 60 s of losing it.
    Light can't explain this; its cause is unknown.

  None of them involves smoke. The build recorded them and left them for the
  grade's owner. Staging's own full run records the same three
  (`specs/done/tracked-armour-models/decisions.md`, T4).
- **The gap:** the spec said only "everything stays green".
- **The reach:** `bun run verify` stays red on three scenes. The next spec's
  closeout will hit the same three and have to tell them apart from its own
  failures.
- **Verdict:** unsound. AGENTS.md says a failure you see is yours to fix,
  whatever caused it, in its own commit. Two of the three are checks that fail
  on how the light looks, not on what they test.
  **Corrected decision:** fix each one on staging in its own commit, because
  all three reproduce there. Repair the `fog-look` and `geometry` checks so
  they no longer depend on the grade's light. Debug `air-audio`'s lost jet as
  its own task.
- **Confidence:** low.

---

## Sound, medium confidence

### S1. A cloud's id is the battle's pop counter, so a side can count pops it never saw

- **When:** slice 05 (`87f12cb51`); accepted by the coordinator in the review
  (`6175847bf`).
- **The choice:** Every cloud gets the next number from one counter the whole
  world shares: 0, 1, 2 and so on. Each published cloud carries its id. Say red
  sees blue's cloud 0, and next sees a blue cloud numbered 3. Red knows it
  popped none itself, so it can work out that two pops happened that it never
  saw. Who popped them, where, and what kind of unit stays hidden: a cloud row
  names no unit and no kind. The alternative is to number clouds per watching
  side. The publisher would then keep a table mapping each world id to each
  side's own number.
- **The gap:** slice 05 required that a cloud row never names its unit or kind.
  It was silent on ids.
- **The reach:** the firewall says the observation never leaks hidden state.
  This is a small, known exception, of the same kind that soldier ids and
  known-prop ids already make: they too count across the whole battle.
- **Verdict:** sound (coordinator): it matches the existing ids, and per-side
  numbering would add per-side id state across the observation, the
  presentation's Wasm module and the publication, for little gain. You may
  overturn it. To reverse, give the publisher a per-side id map and publish
  those ids.
- **Confidence:** medium.

### S2. The four unit-roster journey pictures were re-blessed without a fresh unprimed critique

- **When:** after merging staging (`77510e609`).
- **The choice:** The `unit-roster` browser scene photographs a player's
  first steps: zooming in, a purchase ghost, a placed tank, orders. Four of
  its pictures show the army deck. Smoke adds a Pop smoke button to that deck,
  so the deck is one button wider. One picture also shows an M1A2 SEP V2
  panel, which now has a *Smoke 2* row. The pictures differed from their
  approved versions by 4,468 to 8,648 pixels, and every differing pixel lay in
  the deck or that panel. The build blessed the new pictures after reading the
  diff alone. The usual rule asks for an unprimed critique of any visual change
  first.
- **The gap:** the spec named only the panels scene's pictures; it left the
  roster journey for the closeout's full run.
- **The reach:** these four pictures are now the approved standard for the
  roster journey.
- **Verdict:** sound (coordinator). Slice 08's two unprimed critiques already
  approved this exact button and row in the panels scene, at both widths, and
  nothing else in the four pictures changed.
- **Confidence:** medium.

### S3. The menu-reel captures were re-published natively, not re-recorded in a browser

- **When:** slice 05 (`87f12cb51`).
- **The choice:** The parity captures are recordings of the menu reel's
  battle, as published to the browser. Native tests replay them to prove the
  desktop client reads the same bytes. Smoke changed the publication layout,
  which adds a cloud group, a charges field and a list of look names, so every
  capture's bytes moved. `fixtures/README.md` says a layout change means a new
  browser run (`native-parity-capture`). That run needs the GPU, the full
  runtime assets and the web build, and the pass's disk couldn't hold them.
  Instead, `BLESS_PARITY=1` re-published each capture from its own battle,
  natively, and asserted that every sample's battle digest still matched the
  browser's recording.
- **The gap:** slice 05 said to re-record the captures in this slice. It
  didn't foresee the disk.
- **The reach:** the captures' poses, unit clips, cards and indicators are
  still the old browser run's. No capture battle pops smoke, so none of those
  can differ. Two earlier layout changes took the same route (`558db3389`,
  `1a8123141`), so `fixtures/README.md` now describes a rule the repository has
  stopped following.
- **Verdict:** sound (coordinator): it follows the precedent, and it proves the
  battle unchanged. The web parity suites then passed against a fresh Wasm
  build. To reverse, run `native-parity-capture` with LFS pulled. Then either
  bring the README in line with the native route, or keep using the browser
  run.
- **Confidence:** medium.

### S4. A losing AI unit with nowhere to back up still pops, and fights on from inside its cloud

- **When:** slice 11 (`2d1a32016`).
- **The choice:** The skirmish AI sees one of its tanks lose a fifth of its
  strength within 10 seconds. It pops the tank's smoke and orders a reverse
  40 m straight back, along the way the hull faces. The AI can't see the map's
  edge directly; it asks the route planner whether a route reaches that point.
  If none does (off the map, or blocked), the tank still pops but gets no
  reverse. It stays in its cloud: hidden, but still drawing fire at its last
  contact. In the sample battle, tanks hit at their map-edge entry often popped
  with nowhere to go. There were two alternatives:
  - try other bearings, as the AI's relocation rule already does when a unit's
    own cover blocks its fire;
  - don't pop at all when there is no way out.
- **The gap:** slice 11 delegated "straight back", suggesting it be clamped to
  the map.
- **The reach:** AI tanks can spend charges standing still. Clamping or
  searching bearings is a change local to `skirmish_ai/smoke.rs`.
- **Verdict:** sound. Popping in place still breaks the enemy's identification,
  which is realistic and better than nothing. Whether the AI pops too eagerly
  is slice 11's named feedback, and the answer is to retune its three numbers.
- **Confidence:** medium.

### S5. A cloud the side sees draws over the fog veil, reversing slice 07's written rule

- **When:** the slice 07 follow-up (`9e13f1b81`).
- **The choice:** The fog of war draws unseen ground under a hatched veil.
  Every drawn effect (a muzzle flash, dust, wreck smoke) lifts that veil by
  how much it covers. Only ground shadows leave the veil alone. Slice 07 said
  smoke-screen puffs must not lift it, so a cloud on unseen ground would stay
  hatched. On screen that broke the cloud apart. A unit in smoke is nearly
  blind, so most of its own cloud lies on ground its side can't see. It drew
  as a hatched lavender blob, split at the fog's edge, round a flat cream patch
  where the unit's own short sight reached. The change made smoke an ordinary
  effect and deleted the special puff marker. A side is published only the
  clouds it sees (D12), so drawing them reveals nothing. The ground and units
  under a cloud's thin edges stay veiled in proportion.
- **The gap:** slice 07 fixed the rule, but its unprimed critiques found the
  result misleading. The user was silent at the checkpoint.
- **The reach:** no effect has a per-kind fog case. Thin edge puffs over
  unseen ground keep a faint hatch, which is the veiled ground showing
  through.
- **Verdict:** sound: one rule for every effect, and no information leaks. The
  smoke scene checks both sides' fog-mask view through the cloud. Decided on
  the evidence while the user was silent.
- **Confidence:** medium.

### S6. Every visual checkpoint was decided on the evidence while the user was silent, with three known imperfections left

- **When:** slices 06, 07, 08 and the 07 follow-up.
- **The choice:** At each visual slice the shots were opened for the user for
  about five minutes. There was no reply each time, so the look was accepted
  on before/after diffs and unprimed critiques. The triage is in each
  `assets/<slice>/critique.md`. Three things the critiques saw were left as
  they are:
  - **Inside the cloud,** the existing last-contact marker's red hatch over
    the fog's grey hatch reads busy.
  - **Round a hull inside its cloud,** a faint rectangle of thinner smoke
    shows, and the barrel stays crisp over the smoke. The cause is that the
    hull's top hides the camera-facing puffs behind it. A flat puff can't
    wrap a hull. It reads as "the tank is in there", and the panel's HIDDEN
    row is the authoritative signal anyway.
  - **The smoke mark** (bar and panel) is filled and heavier than its
    line-drawn neighbours. Outline puffs at that size read as rings.
- **The gap:** the checkpoints were non-blocking by design.
- **The reach:** the white-grey billow (64 puffs at opacity 0.6, albedo
  0.82/0.84/0.87) and the smoke mark are now the game's look for smoke.
- **Verdict:** sound: the critiques found the cloud clean, distinct from wreck
  smoke, and readable at far zoom. Retuning the look is data in
  `smoke_screen_looks`.
- **Confidence:** medium.

### S7. The AI's smoke shipped at its starting tuning; the balance report can't show its effect on who wins

- **When:** slice 11, and the closeout's paired balance report (`d76357684`).
- **The choice:** The AI counts a unit as losing when it drops 20 % of its full
  strength within 10 s (`smoke_loss_share` 0.2, `smoke_window_s` 10). It then
  backs it out 40 m (`smoke_reverse_m`): far enough to clear a 15 m cloud with
  room to spare. These are the plan's starting guesses. The closeout ran the
  tournament corpus twice, with smoke switched off and on. Only
  `air_reinforcement` changed (57 to 60 accepted AI orders). No corpus battle
  reaches a result inside its 60 s horizon, so the report shows that pops
  happen and replay, not whether they help.
- **The gap:** the closeout said not to retune, and none was done.
- **The reach:** smoke's effect on balance is unmeasured. A retune is a
  separate pass: three numbers in `fixtures/skirmish-ai.json`.
- **Verdict:** sound: the values are data and the plan delegated them.
- **Confidence:** medium.

### S8. The frozen generation rules were edited by hand to drop the deleted field

- **When:** slice 03 (`59b7c0608`); settled by the coordinator.
- **The choice:** The map-generation parity corpus keeps a frozen copy of the
  physical rules it was recorded with:
  `fixtures/parity/map-layout/physical-rules.json`. The spec's firewall listed
  it as frozen. Smoke deleted `PropBody.lifetime_s` (D19). The repo doesn't
  decode old fields (D25), so the copy's 58 `"lifetime_s": null` lines no
  longer parsed. Those 58 lines were removed by hand, and nothing else. A
  map's generation identity hashes the whole physical catalog, so the seven
  `generate-map` records' identity hashes moved. The ten plan-only records,
  and the generated maps themselves, are unchanged. The two saved skirmish
  records (`skirmish-objectives.json`, `skirmish-purchases.json`) lost the
  same 58 keys each, and their digests didn't move.
- **The gap:** the firewall didn't foresee a schema deletion reaching a frozen
  file.
- **The reach:** `fixtures/README.md` asks for exactly this: a frozen file
  gets a deliberate hand edit for a schema change, never a regeneration.
- **Verdict:** sound (coordinator).
- **Confidence:** medium.

### S9. Two new test units moved one pinned battle's digest by shifting catalog order

- **When:** slice 03 (`59b7c0608`); settled by the coordinator.
- **The choice:** The lab and the tests need smoke carriers, so two new test
  rows were added: `test_smoke_tank` and `test_smoke_squad`. Existing test
  units stay without smoke, so their digests hold (D23). Catalog rows are
  numbered in name order, though. The new rows sort before `test_supply`,
  `test_tank` and `test_transport_heli`, so those three types' numbers rose by
  two. The battle digest folds a type's number when a side identifies a unit,
  which affects four pinned records:
  - in `scenario-overrides.json`, the one pinned test-unit battle whose units
    identify each other, the digests changed from tick 200;
  - `stream` and `contact-lifecycle` changed only the published kind;
  - `exact-numbers` changed only its embedded test documents.

  All four were re-recorded. Renaming the rows to sort last (`test_zsmoke_*`)
  reproduced every committed record exactly, which proves catalog order was
  the only cause.
- **The gap:** the firewall promised unchanged test-unit digests. It didn't
  foresee that a digest pins catalog order.
- **The reach:** any new test row that sorts early moves the same records.
- **Verdict:** sound (coordinator): the spec's names beat names chosen to
  dodge a pin, and a pin on catalog order is fragile anyway.
- **Confidence:** medium.

---

## Sound, high confidence

### The cloud and sight

#### S10. A cloud changes in steps, not smoothly, everywhere that reads it

- **When:** the plan (write-spec), built in slice 01 (`e7ef55eba`).
- **The choice:** A cloud grows to full size in `bloom_steps` (4) equal steps
  over `bloom_s` (1 s). It holds, then thins in `fade_steps` (5) steps over its
  last `fade_s` (10 s), and is gone at `life_s`. Thinning scales its density
  and its concealment shortfall (`1 − concealment`) together. Sight, the fog
  sweep, the GPU fog, the drawn cloud and the sound all ask
  `contract::smoke::state` for the same step. A smooth fade would change
  every tick, and the GPU fog would then rebuild the eye maps near it every
  tick.
- **The gap:** D6 gave the times but not their shape.
- **The reach:** the GPU fog's rebuild budget assumes a handful of changes per
  cloud. A smooth fade would mean measuring that budget again.
- **Verdict:** sound.
- **Confidence:** high.

#### S11. Overlapping clouds add their smoke

- **When:** the plan; slice 01.
- **The choice:** A sight line through two overlapping clouds crosses both
  chords, and their depths add, so the overlap is denser. Concealment differs:
  a target standing in two clouds takes the stronger one, not the product
  (`sensing::ground`'s "strongest wins").
- **The gap:** D1 gave one cloud's density.
- **The reach:** stacked pops thicken the screen, which a viewer expects.
- **Verdict:** sound: D1 taken literally.
- **Confidence:** high.

#### S12. A side sees a cloud when it sees the ground at the cloud's rim

- **When:** the plan; slice 04 (`235a44a86`).
- **The choice:** Red sees blue's cloud when red's fog map marks as seen any
  fog cell whose centre lies within the cloud's current radius plus one cell
  of its centre (`sim::smoke::seen_by`). A side always sees its own clouds. A
  cloud hides its own middle, so only its near rim can be seen. The rule uses
  the cloud's current radius, so a cloud still blooming is only as wide as it
  has grown.
- **The gap:** D12 said "can see where the cloud stands" without saying how.
- **The reach:** this is the one test that decides which clouds are
  published, drawn and heard.
- **Verdict:** sound: one owner, the fog sweep the side already runs.
- **Confidence:** high.

#### S13. Smoke enters sight as its own depth term, and only sight reads it

- **When:** the plan; slices 01 and 04.
- **The choice:** `WorldGeometry::sight_depth` is the depth of tree crowns
  (`foliage_depth`) plus smoke, sampled along the line every 1 m like foliage,
  with each piece's chord exact. Three things read it:
  - sensing;
  - wire- and laser-guided missiles' guidance;
  - the guided return-fire check.

  Three things keep reading tree crowns only:
  - the launch-path check (`launch_clear`), so artillery and SAMs still launch
    from inside smoke;
  - map analysis;
  - map generation.

  A target's concealment reads only whether it stands inside the cloud's
  circle, at any height. Every ground sample stands below every cloud's top,
  and airborne units skip ground concealment anyway.
- **The gap:** the plan found that the two guided-missile checks read foliage
  directly. That needed wiring the walk had called free.
- **The reach:** a guided missile drops for good when smoke cuts its line. A
  fire-and-forget missile (one with a seeker) flies on.
- **Verdict:** sound.
- **Confidence:** high.

#### S14. The fog sweep counts smoke before deciding a cell is visible, and re-sweeps only observers near a change

- **When:** slice 04.
- **The choice:** The sweep walks each ray outward 8 m at a time. At each
  step it adds the smoke chord from the previous step, at the height the line
  passes, before asking whether the cell is still within reach. Foliage is
  added after that check, as before. So a cell is judged by the exact smoke
  between the eye and it, which is what `sensing::sees_point` counts. A test
  holds a ray's last visible cell to the last one sensing sees. Each
  observer's cached field is keyed by a hash of the ids and steps of the
  clouds within its reach (`SmokeClouds::key_near`), so an observer away from
  every cloud keeps its cache. With no clouds the sum is bit-identical to
  before.
- **The gap:** the plan didn't say where in the step smoke counts.
- **The reach:** a cloud changing step re-sweeps every observer in reach. One
  sample battle, with 16 clouds popped out of the fight, ran 17 % more
  instructions. That is a tripwire, not a budget.
- **Verdict:** sound.
- **Confidence:** high.

#### S15. The world keeps each cloud's current state, and the side that popped it

- **When:** slice 03 (`59b7c0608`).
- **The choice:** `SmokeClouds` works out each cloud's step once per tick, in
  `advance`, and its readers (`live`, `states`, `near`, `key_near`) take no
  tick. Sight reads the clouds many times a tick. Each cloud also records its
  popping side, for the "own clouds are always seen" rule. A cloud is not a
  body: popping one or losing one bumps no obstacle or navigation revision. A
  planning snapshot of the world, used to plan movement, holds no clouds.
- **The gap:** the plan left storage to the slice.
- **The reach:** movement and air planning never see smoke, and side planners
  never get clouds they may not see.
- **Verdict:** sound.
- **Confidence:** high.

#### S16. A battle that never pops smoke keeps its old digest exactly

- **When:** slice 03.
- **The choice:** The clouds fold into the slot the deleted prop expiries
  used. That is their count, then each cloud's id, side, centre, pop tick and
  every number of its spec. The next cloud id folds only once a cloud has been
  popped, so a smoke-free battle folds just the zero count, as the empty
  expiry list did. A unit's charges and its smoke resupply clock fold only
  when it carries smoke.
- **The gap:** D23 asked for this outcome; the encoding was the slice's.
- **The reach:** smoke-free records never move. Equal digests still mean
  equal clouds.
- **Verdict:** sound.
- **Confidence:** high.

#### S17. Popping is an instant change beside the unit's orders, and a rider or a fallen unit can't pop

- **When:** slice 03.
- **The choice:** `Battle::apply` handles `Order::PopSmoke` before the unit's
  ordinary orders and leaves them alone, so a moving tank pops and keeps
  driving its queued route. `sim::smoke::pop` takes the rules, which hold the
  cloud's spec and the tick rate. A unit off the field (a rider, or one
  destroyed) pops nothing. A unit with no charge left is skipped silently,
  with no refusal.
- **The gap:** D21 set the shape; the signature and the off-field case were
  the slice's.
- **The reach:** smoke never cancels an order. The AI must still send its
  reverse as a second order.
- **Verdict:** sound.
- **Confidence:** high.

### The GPU fog

#### S18. The fog-of-war cost gate said keep exact circles

- **When:** slice 02 (`7edeb52ca`), confirmed in the slice 06 lab.
- **The choice:** Slice 02 was a kill gate. If the GPU fog couldn't take a
  live list of circles within the frame budget, the plan would stop. Native
  measurement on the endurance field (100 eyes, up to 12 clouds stepping
  through their lives) passed:
  - the parity test was green;
  - no frame rebuilt more than the 48-eye budget;
  - eyes out of every cloud's reach never rebuilt;
  - the worst fog frame (5.5 ms) sat far inside the 33 ms a frame gets at the
    30 FPS floor.

  In the lab, an all-eyes rebuild went from 0.47 to 0.76 ms with twelve
  clouds standing, under the 1.55 ms tripwire. So the march has no per-ray
  cull of circles.
- **The gap:** the plan delegated the verdict to the measurement.
- **The reach:** if the frame cost ever reads high, the first lever is to skip
  `fogSmokeStep` on steps outside every circle's span. That leaves parity
  untouched.
- **Verdict:** sound.
- **Confidence:** high.

#### S19. Clouds reach the GPU fog through the publication, and change it only near them

- **When:** the plan, confirmed in slice 02; built in slices 02 and 06
  (`eb7768ea5`).
- **The choice:** The presentation crate already decodes each publication
  into the observation it keeps, and takes the side's eyes from it. The clouds
  take the same path. On each new publication, `prepare_fog` turns each
  published cloud into its state at the publication's tick and hands the list
  to `FogSight::set_smoke`. That compares whole states by id. When a cloud
  comes, steps or goes, the eyes whose map reach meets its old or new circle
  join the moved eyes' rebuild queue, oldest first. No map is dropped, because
  a dropped map rebuilds at once, past the budget. Neither the browser's nor
  the native client's foliage builder changed.
- **The gap:** the plan had to confirm the data path, and left the dirty-eye
  rule to the slice.
- **The reach:** the fog has one input for clouds, the published observation,
  as for eyes.
- **Verdict:** sound.
- **Confidence:** high.

#### S20. The GPU takes the circles in a new storage buffer and their count in a spare word

- **When:** slice 02 (`a0f5f9568`).
- **The choice:**
  - **The records.** Each cloud reaches the GPU as a 32-byte `FogSmoke`
    record (centre, radius, height, depth per metre) at binding 8 of the
    fog's build layout. Their count sits in what was a padding word of
    `FogParams`, so every upload without clouds is byte-for-byte unchanged,
    and the fog-sight vectors held.
  - **The maths.** The WGSL `fogSmokeStep` measures each chord along and
    across the step, not through the quadratic's discriminant. It is the same
    chord in exact arithmetic, but f32 keeps its digits at map-scale
    coordinates. The parity tolerance is 1e-5 × (1 + depth).
  - **The height.** The march counts smoke at the same point its canopy test
    uses: the end of each step, raised to the terrain horizon.
- **The gap:** the plan delegated the layout, the binding and the packing.
- **The reach:** the build layout now uses all 8 storage buffers WebGPU allows
  by default. The next fog input needs a merged buffer or a raised limit.
- **Verdict:** sound.
- **Confidence:** high.

### Publication

#### S21. The observation reuses the contract's cloud type, and the publication indexes looks by name

- **When:** slice 05 (`87f12cb51`).
- **The choice:**
  - **The type.** `ObservationFrame.smoke_clouds` is a list of
    `contract::smoke::SmokeCloud`, the type the law reads. The slice's planned
    `ObservedSmoke` was the same type field for field. The popping side lives
    beside it in the world, never in it.
  - **The charges field** is `OwnUnit.smoke`, never `smoking`: that already
    means damage smoke.
  - **The row.** A published cloud row is `id, x, y, z, popped, radiusM,
    heightM, lifeS, bloomS, fadeS, bloomSteps, fadeSteps, depthPerM,
    concealment, look`, in plain floats, which are exact below 2²⁴.
  - **The looks.** `look` indexes the layout's `smokeLooks`: every look the
    catalog's smoke capabilities name, deduplicated, in name order. So the
    layout depends on the rules alone, like `roundKinds`.
  - **The packer.** It takes that look table, refuses a cloud whose look isn't
    in it, and every other caller passes an empty table.
  - **Vectors.** The codec vectors' input frames carry one cloud and a charge
    count, so both decoders exercise the new group.
- **The gap:** the slice delegated field order and packing.
- **The reach:** this is the closed publication contract for smoke. A second
  layout change for smoke (unseen pops heard, say) would re-publish every
  capture again.
- **Verdict:** sound.
- **Confidence:** high.

### Drawing

#### S22. A cloud's look is a presentation row its capability names, with its own budget

- **When:** slice 07 (`d3a613c7a`).
- **The choice:** The simulation's cloud names a look (`discharger` for
  vehicles, `grenade` for infantry). Each look is a `smoke_screen_looks` row in
  `fixtures/presentation.json`, holding:
  - albedo and opacity;
  - puff count and puff size range;
  - swirl rate and height band;
  - softness;
  - the pop sound.

  The two rows differ only in puff size. All smoke screens together keep to
  `smoke_screen_budget` (1024 puffs, about sixteen full tank clouds), apart
  from the wrecks' 4096. Past the budget, every cloud thins alike. Fielding
  sends every look row to the effects, so an enemy's cloud always has its
  look, and it refuses a fielded unit whose cloud names no row.
- **The gap:** the slice delegated sizes and counts.
- **The reach:** this is the weapon-look pattern. A new smoke kind is a row,
  not code.
- **Verdict:** sound.
- **Confidence:** high.

#### S23. The drawn cloud thins optically, and stands at the law's first step from its pop

- **When:** slice 07, and the 07 follow-up.
- **The choice:**
  - **Thinning.** Each puff's opacity follows the cloud's density optically,
    `1 − (1 − opacity)^density`, not `opacity × density`. With dozens of
    overlapping puffs the linear form left the half-faded cloud looking full.
    Optical depth linear in density is also what sight uses.
  - **Growth.** The drawn cloud's size is
    `min(stepped size, max(first step, age / bloom_s))`. It stands at the
    first step's circle on its pop tick, as concealment does, then grows
    smoothly inside the stepped circle. Before this, HIDDEN lit a tick before
    any smoke showed.
  - **Shape.** The swirl's internal shape is four constants in
    `effects/frame/smoke_screen.rs`: the flipbook frames a puff stands at, how
    far it wanders, its lowest centre, and a rim taper.
  - **Draw order.** Screens draw after every other effect, unsorted and
    premultiplied.
  - **Bounds.** A test holds every puff inside the circle and below the top
    for the cloud's whole life.
- **The gap:** the slice delegated the swirl law's shape.
- **The reach:** the drawn cloud can never show smoke where sight has none.
- **Verdict:** sound.
- **Confidence:** high.

### Controls and the lab

#### S24. One smoke mark for the button and the panel row; the button is a plain command after Reverse

- **When:** slice 08 (`5c6910967`).
- **The choice:** POP SMOKE sits right after REVERSE. It is never armed, and
  it is enabled when any selected unit has a charge. H and the button share
  `useUnitControl.popSmoke`, which sends one `pop_smoke` naming only the
  selected units that have a charge. The tooltip reads "Pop smoke (H)". The
  button and the panel's *Smoke n* row share one icon, `states/smoke`: a
  banded grenade with puffs rising from its mouth. The plan had a separate
  `pop_smoke` HUD icon. Critiques read a cloud over an arrow as "upload", a
  cloud over a canister as "cloud storage", and two shapes for one concept as
  two concepts. A reinforcement card (`arrivingUnit`) shows its type's smoke
  capacity, as it shows active protection's.
- **The gap:** the slice delegated the drawing and the tooltip.
- **The reach:** the bar already draws Deploy and Afterburner with state marks,
  so this is that pattern.
- **Verdict:** sound.
- **Confidence:** high.

#### S25. The smoke lab is the real battle view on a small test map

- **When:** slice 06; the scripted pop replaced in slice 08.
- **The choice:** `/labs/smoke-screen` is the production `BattleView`, with a
  labelled side switch that watches the same battle as red. Its map is a new
  400 × 260 m `smoke-screen` test map with a road and a wood. The layout
  mirrors the mock: blue west with two smoke tanks and a smoke squad, red's
  tank and AT team about 160 m east. Blue's grass tank shells a red jeep into
  a burning wreck, for a shot of the cloud beside wreck smoke. The browser
  scene selects the road tank from its army card and presses H at tick 120.
- **The gap:** the slice delegated the layout and how the pop is triggered.
- **The reach:** the lab is the one place to see the whole moment, from both
  sides, with real input.
- **Verdict:** sound.
- **Confidence:** high.

#### S26. A side's contact memory belongs to that side across a view switch

- **When:** slice 06 (`3840ddb56`), in its own commit.
- **The choice:** The first lab that switched sides drew red's last-contact
  marker over blue's own tank, because the contact presentation kept red's
  memory. `ContactPresentation.update` now takes the observation and forgets
  its memory when the report belongs to another side.
- **The gap:** a bug no earlier page could reach.
- **The reach:** any page that switches sides is safe.
- **Verdict:** sound.
- **Confidence:** high.

### Roster and supply

#### S27. Each carrier authors its smoke on its own row; a child row restates only its capacity

- **When:** slice 09 (`6a0813ead`).
- **The choice:** The 26 roster carriers each author `capabilities.smoke` on
  their row, never on a shared profile. Profiles mix in units that shouldn't
  carry smoke: the IFV profile also covers SAM launchers. A child row that
  inherits a parent's smoke overrides only `capacity`: the Abrams SEP v2
  Trophy and SEP v3 Trophy, the Challenger 3 and the Leopard 2A8. The M1E3
  inherits the SEP v3 Trophy's 3 unchanged. The resolved charges match D2's
  bands and lists exactly: 19 tanks, 4 IFVs and 3 infantry rows. The assault
  squad is one row serving Europe and Eastern.
- **The gap:** the slice delegated key order.
- **The reach:** the cloud's numbers repeat on every parent row. A retune of
  the cloud itself is a script over rows, not one edit.
- **Verdict:** sound: it follows the repo's "values live on the unit".
- **Confidence:** high.

#### S28. A smoke charge's price and refill time follow the flares'

- **When:** slice 09.
- **The choice:** Every roster carrier refills one charge in 2 s of service
  for 2 stock, the same as a flare (`rules.json` `flare_service_s`,
  `stock_per_flare`). The values live on the capability. `stock_per_charge`
  is a whole number (`u32`), not the plan's float, like every other stock
  price. Smoke is served after replacement soldiers (D24). The test carriers
  keep 5 s and 5 stock, so tests can tell the two apart.
- **The gap:** the slice said to follow the flares, and to choose and record
  the values.
- **The reach:** one refill rule for both countermeasure charges.
- **Verdict:** sound.
- **Confidence:** high.

#### S29. The test carriers: D6's clouds, and a smoke tank no roster deck offers

- **When:** slice 03.
- **The choice:**
  - **`test_smoke_tank`** has 2 charges and **`test_smoke_squad`** (four
    riflemen) has 1. Both use D6's cloud numbers.
  - **`roster: null`.** The tank sets it, as `test_apc` does. Without it, a
    test deck that patches `test_tank` into a roster also offers the smoke
    tank. The AI test deck then bought the smoke tank in place of
    `test_tank`, because the cheaper id wins a tie.
  - **Refusal tests.** Three catalog refusal tests now accept a refusal that
    names the patched type or any type extending it. A patch to `test_tank`
    also reaches `test_smoke_tank`, and the first type in order is the one
    named.
- **The gap:** the plan named the rows, not their values.
- **The reach:** tests that patch `test_tank` reach the smoke tank too.
- **Verdict:** sound.
- **Confidence:** high.

#### S30. Smoke moves the game's battle digests: named, and only one corpus row

- **When:** slices 09 and 11.
- **The choice:**
  - **Live battles.** Every battle on the live catalog that fields a carrier
    folds its smoke state, so skirmish and balance battles hash differently
    from slice 09 on.
  - **Pinned records.** The pinned roster records carry their own rules
    snapshot, so they replay as recorded, and no pinned record moved.
  - **The corpus.** The AI's smoke moved one tournament row,
    `air_reinforcement`, and every row's replay check matches.
  - **Unchanged.** Test-unit and capture-unit digests, and the menu reel's
    event log.
- **The gap:** D23 required the shift to be named.
- **The reach:** anyone comparing a skirmish digest across this change sees
  it move.
- **Verdict:** sound.
- **Confidence:** high.

### The AI

#### S31. The AI watches each unit's strength over a sliding window

- **When:** slice 11 (`2d1a32016`).
- **The choice:** At each AI decision (every 5 s), `HpWatch` records each own
  unit's strength: hull health, or the sum of its soldiers'. A unit is losing
  when the window's peak minus its strength now is at least
  `smoke_loss_share` of its full strength. When a unit pops, its samples
  reset, so the loss that spent one charge never spends a second. A unit's
  samples go when it leaves the frame. The plan had a single
  `(hp, tick)` baseline reset each window, which misses a loss that straddles
  a reset. The rule reads only how much was lost, so friendly fire counts
  (D22). A frame-level test documents that. No random draw is made.
- **The gap:** the slice delegated when samples are dropped.
- **The reach:** one or two samples per unit; the rule is AI-only.
- **Verdict:** sound.
- **Confidence:** high.

#### S32. The AI's pops go first in one order, and a unit backing out gets no other order

- **When:** slice 11.
- **The choice:** Every losing unit pops in one `PopSmoke`, placed first in the
  decision's orders, before even a purchase. Each unit's reverse follows. The
  order rate limit can drop a reverse but never the pop (D21). A unit is
  "backing out" when it was ordered back at this decision, or is reversing
  toward a goal it can still reach. The AI orders no other reverse. Such a
  unit is skipped at the top of the per-unit loop, beside passengers, not
  later at the relocation step. Otherwise a loaded carrier's unload move,
  issued in the same decision, would overwrite its reverse. A passenger never
  pops; a garrisoned squad may.
- **The gap:** the plan said where precedence must hold, not how.
- **The reach:** a carrier on its errand resumes it after backing out.
- **Verdict:** sound.
- **Confidence:** high.

#### S33. A system test holds that a human player's units never pop on their own

- **When:** the whole-spec review (`ee0b7cdcb`).
- **The choice:** In a team skirmish, blue's human and AI players each buy a
  smoke tank. A red human's four test tanks shoot both. The AI's carrier pops
  through its own order. The human's carrier loses more than the AI's
  threshold and keeps every charge. The seam under test is the skirmish turn's
  filter that runs the AI only on its own player's units. Removing the filter
  fails the test. The test raises the smoke tank to 500 hp in its own rules,
  so both carriers survive long enough to lose a fight.
- **The gap:** D3's firewall had no test.
- **The reach:** a regression here means a player's units act alone.
- **Verdict:** sound.
- **Confidence:** high.

### Sound

#### S34. The pop's trigger lives in the sound frame, reading the clouds already published

- **When:** slice 10 (`bbbd7beeb`); the review (`ece2665db`).
- **The choice:** The plan had the effects publication emit a pop event. The
  sound frame (`sound::smoke_pops`) instead reads the clouds the side was
  published. A cloud the side sees while the law says it is still blooming,
  and hasn't been heard, plays its look's sound at its centre, keyed by its
  id. The frame keeps each heard cloud whole and forgets it once its bloom is
  over, asking the same law. A cloud first seen after its bloom is silent. The
  sound frame already owns tick time, the "already heard" state of every other
  one-shot, and its reset on a new battle.
- **The gap:** the slice placed the trigger in the effects publication.
- **The reach:** the publication layout, both clients' publication types and
  the browser's `soundFrame.ts` are unchanged. There is no second copy of the
  trigger to mirror.
- **Verdict:** sound.
- **Confidence:** high.

#### S35. The recording belongs to the look and the level to the mix, and every look must have a sound

- **When:** slice 10; the review (`65712615b`).
- **The choice:** `smoke_screen_looks.<look>.sound` names a library recipe.
  `audio.json` gains `smoke_pop: { gain: 0.8 }`, between the flare (0.75) and
  the cues, mirrored in `audioMix.ts`. Sound admission refuses an unknown or
  looping pop sound. The review made `sound` required, reversing slice 10's
  "no sound pops silently". Fielding already refuses a cloud with no look row,
  and admission admits every row's sound, so every published cloud has a pop
  and the frame looks it up without a guard. The clips were generated through
  `bun run generate effects`, which reaches ElevenLabs through the Bento
  service proxy with this machine's login. No key was copied.
- **The gap:** the slice listed the mirrors, not the level row's shape.
- **The reach:** a new smoke look must name a sound.
- **Verdict:** sound.
- **Confidence:** high.

### Closeout and names

#### S36. One name per smoke: renames as a hard cutover, and the publication keeps its names

- **When:** the review (`af2630a03`); coordinator.
- **The choice:** Renamed:
  - presentation `smoke_looks` → `smoke_screen_looks`, `SmokeLook` →
    `SmokeScreenLook`;
  - the lab route `/labs/smoke` → `/labs/smoke-screen`
    (`routes/smokeScreen.tsx`);
  - the test map and its encounter `smoke` → `smoke-screen`;
  - the browser scene `smoke.mjs` → `smoke-screen.mjs`.

  These follow the repo's casing: kebab-case ids and folders, camelCase route
  files. The map's identity doesn't name its folder, so its hash held. The
  publication's `smokeLooks`, `smokeClouds` and `smokeCharges`, and
  `sim::publication::smoke_looks`, keep their names. They are the closed
  publication contract, where wreck smoke is `smokes` and damage smoke
  `smoking`, so nothing collides. Renaming them would re-publish every capture
  for no change in meaning.
- **The gap:** the build had used "smoke" alone for things that sit beside
  wreck smoke.
- **The reach:** "smoke screen" is the concept's name in presentation and the
  lab. "Smoke" is its name in the simulation and the publication.
- **Verdict:** sound.
- **Confidence:** high.

#### S37. The kill gate's cost workload was deleted once its verdict was recorded

- **When:** the review (`a24a43524`); coordinator.
- **The choice:** The smoke lab had a 100-eye frame-cost mode (`?cost`,
  `SMOKE_COST`, `encounters/cost.json`) that only slices 02 and 06 needed. It
  is gone, and the lab plays its one encounter. The shared lab probes it used
  stay, because the fog scene uses them.
- **The gap:** the plan didn't say what happens to gate scaffolding.
- **The reach:** re-measuring needs a new workload, or the fog scene's own.
- **Verdict:** sound.
- **Confidence:** high.

#### S38. Two tournament corpus seeds the generator now refuses were repinned

- **When:** the review (`b76ec885c`), its own commit; coordinator.
- **The choice:** The balance report's corpus pinned `team_3v3` (mixed large,
  seed 8) and `team_5v5` (mixed xl, seed 9). Both had drifted into seeds the
  map generator legitimately refuses: one fails a fairness check, the other
  fits no mirrored entry pair. They failed the same way on staging. They were
  repinned to the nearest admitted seeds, 11 and 10. A test now checks that
  every corpus row seats its players, and the tournament's refusal carries the
  generator's diagnostics.
- **The gap:** a pre-existing failure the closeout's report needed fixed.
- **The reach:** the team rows now play in the balance report.
- **Verdict:** sound: a failure seen was fixed in its own commit.
- **Confidence:** high.

#### S39. The tutorial teaches "pop smoke, then reverse out"

- **When:** the review (`eda5837cc`).
- **The choice:** One short item after the reverse advice. It says a unit
  whose panel shows *Smoke* pops a thick cloud with the key from
  `CommandBindings`, that the enemy loses sight of it while it sees almost
  nothing out, and to pop smoke, then reverse out. The supply item says trucks
  refill ammunition and smoke. It gives no list of carriers, because who
  carries smoke is data.
- **The gap:** D18 asked for smoke wherever units fight. It said nothing of
  teaching.
- **The reach:** the tutorial stays right when the roster changes.
- **Verdict:** sound.
- **Confidence:** high.

#### S40. A smoke refusal names its field path

- **When:** the review (`8c4a24ef4`).
- **The choice:** A bad capability reads `capabilities.smoke.<field> …`, and
  a bad cloud reads `capabilities.smoke.cloud.<field> …`. The cloud's own
  validator names only the field, because it doesn't know where it sits, and
  the capability adds the prefix.
- **The gap:** the slice delegated message order.
- **The reach:** this matches the flare and protection refusals beside it.
- **Verdict:** sound.
- **Confidence:** high.

#### S41. The tests prove behaviour through fakes the tests define

- **When:** slices 04, 06 and 10.
- **The choice:**
  - **Fire-and-forget.** The case uses a test-defined variant of the team's
    missile with an infrared seeker. The shipped Javelin keeps its gunner's
    sight here, and a SAM duel against a helicopter proved unreliable.
  - **Contacts.** The under-fire test pins a firing contact (the carrier
    shoots back) and the reverse-out test pins a last-seen contact. Both are
    D10's "existing last-contact marker".
  - **Fog rebuilds.** "Eyes near a cloud rebuild, eyes away don't" is held by
    a presentation test through a real publication, not by new fog-sight
    vectors, which would be a second pinned recording.
  - **The own-pop sound.** That case runs a real battle, because whether a
    side sees its own cloud is the simulation's rule.
- **The gap:** the slices named what to prove, not how.
- **The reach:** none pins a roster value.
- **Verdict:** sound.
- **Confidence:** high.

---

**Housekeeping (7), all sound:**
1. Files over the size budget were split by concept in the same commit as the
   change that needed the room: `fog/eye.rs`, `catalog/charges.rs`,
   `publication/report_row.rs`, `reader/report.rs`, the flare and protection
   readiness, `tests/publication/paired_records.rs`,
   `skirmish_ai/relocation.rs`, `tests/artillery.rs` and `sound/scripted.rs`.
   The ratchet measures a commit against its parent, so a split committed
   first frees no room.
2. `Battle::add_prop` was deleted with the prop expiries.
3. The smoke test carriers' unit icons were generated.
4. The panels workbench gained "Smoke ready" and "Smoke spent" cases.
5. `crates/sim/README.md` states supply's whole service order in one
   sentence.
6. Lab scratch evidence moved under `throwaway/evidence/smoke-screen/`.
7. The scene's `POP_TICK` is 120.
