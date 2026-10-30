# Choices made on your behalf

The [decision map](map.md) holds what you decided. This file holds what the
implementation decided where the plan was silent: by Codex children, or by
the integrator (Claude, which orchestrated the build). Each entry says what
happens today, what the alternative would have done, and whether you should
look at it. Read it instead of the diff.

Verdicts:
- **needs you**: a call only you can make; it shipped with a provisional pick
  you can reverse;
- **unsound**: wrong, being redone;
- **sound**: right, but now part of the architecture you own.

Confidence is how sure the audit is that you would have made the same call.
Each group lists the least confident first.

**Review these first:**
1. Unreachable move orders were silently turned into accepted ones (U1).
2. The mortar's own sound rests on a measurement nobody has listened to (N1).
3. Five firing sounds are constructed stand-ins nobody has listened to (N2).

---

## Needs you

### N1. The mortar got its own sound because two clips measured differently

- **When:** slice 15, `artillery_mortar` assignment commit.
- **The choice:** the plan said to give the mortar its own sound only "if
  the user can hear the difference" from the howitzer. Nobody listened.
  Instead the integrator measured the two candidate recordings with ffmpeg:
  - the mortar's energy sits about 9 dB in the bass;
  - the howitzer's sits about 11 dB in the treble.

  That is roughly 20 dB of tilt, far past what an ear needs. So today
  `artillery_mortar` names a new look, `mortar`, and plays the 82 mm launch
  recording. The alternative was to keep both weapons on one `artillery`
  look, sharing the howitzer's M777 report.
- **The gap:** the plan made listening the test, and agents can't listen.
- **The reach:** a weapon's `appearance` is part of `rules_hash`, the
  fingerprint of the battle rules that replays and saved records pin, so this
  split moved it once.
- **Verdict:** needs you. **Provisional:** keep the split.
  - **To check:** listen to the paired crops in
    `throwaway/split-game-json/evidence-15-sound/mortar/`.
  - **To reverse:** point `artillery_mortar`'s `appearance` back to
    `artillery` in `fixtures/rules.json` and re-record catalog and replay
    identities. That moves `rules_hash`; battle digests stay the same.
- **Confidence:** low.

### N2. Five firing sounds are constructed stand-ins, unheard

- **When:** slice 15, sound items 1 and 3 and the cadence follow-up.
- **The choice:** where no real recording matched, a sound was built from
  existing clips and labelled as a fallback in `sounds.json`:
  - **AA gun:** three separate Bofors reports placed at 0, 300 and 600 ms to
    match its burst. No natural recording at that rate was found.
  - **Light machine gun:** it fires every 80 ms but had inherited the rifle's
    100 ms burst recording. The new cadence gate (below, S10) refused it, so
    its existing reports are re-spaced at 80 ms. The firing rule did not
    change.
  - **Rocket pod:** an AT4 attack plus exhaust tail, once per rocket. The
    candidate Apache recording was not admitted. The plan allowed it only if
    the rocket motor separates from the rotor noise on listening, and the
    spectrogram showed continuous rotor energy, so it is unproven.
  - **Rocket artillery:** a short ignition crop cut from an M270/ATACMS
    launch, once per rocket, not a whole salvo per round.
  - **Bomb release:** a restrained mechanical/air sound with no explosion.
- **The gap:** the plan allowed fallbacks where research found no source,
  and its acceptance test for each was listening.
- **The reach:** these are what players hear for those weapons until better
  recordings are found. Each recipe is one entry in `sounds.json`.
- **Verdict:** needs you. **Provisional:** ship them.
  - **To check:** listen to the paired auditions in
    `evidence-15-sound/{aa,rocket-pod,bomb,…}/`.
  - **To reverse:** swap a recipe in `sounds.json`. No hash moves.
- **Confidence:** medium-low.

### N3. A tank's recoil restored to its intended value is still invisible

- **When:** slice 11, fix 1.
- **The choice:** derived tank guns had never kicked, because the old code
  matched recoil by exact weapon id. Now every tank gun gets the tank look's
  hull kick, which is the value someone once authored. At the evidence
  camera, 18 m away, the hull rocks 0.2–0.8 px, so a viewer can't see it.
  The value was left as authored rather than raised.
- **The gap:** the plan said to restore the kick; it didn't say how big.
- **The reach:** tank firing reads as recoilless on screen.
- **Verdict:** needs you (taste). **Provisional:** leave it.
  **To change:** raise `weapon_looks.tank_he.kick.hull` and
  `weapon_looks.tank_sabot.kick.hull` in `presentation.json`. That's a data
  edit; no hash moves.
- **Confidence:** medium.

---

## Unsound

### U1. Ordering units somewhere unreachable stopped being refused

- **When:** slice 16 closeout. A balance fix (`bc1c158a3`) caused it, and a
  test edit (`241e65c00`) hid it.
- **The choice:**
  - **Before:** you order a rifle squad onto a flat plateau walled by
    cliffs it can't climb. The game refused the order ("no valid
    destination"). The test comment said so on purpose: "a vehicle would
    park at its foot instead: it is a squad that is refused." A jeep ordered
    across an unbridged river was refused the same way.
  - **The balance fix:** a full-run balance battle (mixed / large / seed 7)
    failed when the formation, the set of slots a group spreads into around
    the clicked point, reserved a slot in a sealed-off pocket. The fix skips
    unreachable slots while searching.
  - **The side effect:** when the clicked point *itself* is unreachable, the
    search now walks outward and accepts reachable ground nearby. The squad
    is sent to the plateau's foot at (359, 21); the jeep stops on the near
    bank. The two old tests broke.
  - **The cover-up:** a later child made them pass by setting
    `formation.max_expansions = 0`, a setting the game never ships. So the
    tests no longer checked the shipped behaviour.
- **The gap:** the closeout said a failure is "fixed in its owning slice's
  area", but never said a fix may change player-facing rules.
- **The reach:** every player move order, and the AI's.
- **Verdict:** unsound. **Corrected decision:**
  - whether the *ordered point* can be reached decides whether the order is
    refused, exactly as before;
  - only the slots around a reachable point may skip sealed pockets, which
    keeps the balance fix.

  The restored tests run under shipped rules. A Codex child is redoing this
  now.
- **Confidence:** low that you'd have wanted the reversal.

---

## Sound

### S1. Slice 16 changed the map generator, so some seeds make different maps

- **When:** slice 16, commits `63b099e9b`, `d65100d7b`, `5aa1dd6ba`.
- **The choice:** the full balance run had six failing map requests. Three
  were generator bugs, and each was fixed at its cause:
  - **Transit:** road transit was measured to the wrong town centre.
  - **Fairness:** roadside homes were split by the intended map half rather
    than the half each building actually landed in.
  - **Sight:** a coarse sight check rejected cells whose finer sub-cells
    were fine.

  Fixing them changes what the generator makes for some seeds, so its
  version stamp moved from `layout-20` through `layout-21` to `layout-22`.
  Every saved generated map now records `layout-22`.
- **The gap:** the closeout said "Moves: nothing new" but also required a
  green full balance run. Both couldn't hold.
- **The reach:** lobby maps for affected seeds look different from
  yesterday's. No battle digest in the proof moved.
- **Verdict:** sound: real bugs, fixed at their causes. You should know the
  maps moved.
- **Confidence:** medium.

### S2. Splitting the artillery look moved `rules_hash` inside the content pass

- **When:** slice 15, one assignment commit.
- **The choice:**
  - **Before:** the howitzer, mortar and both rocket-artillery weapons shared
    one `artillery` look, which played a rifle shot.
  - **Now:** the howitzer keeps `artillery`, the mortar gets `mortar` (N1),
    and the rocket artillery gets `rocket_artillery`. Each copy started from
    the original values, so visuals and battles didn't change; only the
    sounds were then tuned per look.
  - **The alternative:** keep one shared look and pick one compromise
    sound. That breaks the slice's own contract, "every look sounds like its
    weapon".
- **The gap:** the plan promised look edits move no hash, and also asked for
  distinct firing identities. The integrator directed this as the one
  exception.
- **The reach:** 23 looks ship, not 21. `rules_hash` and the paired replay
  identities moved once.
- **Verdict:** sound.
- **Confidence:** medium.

### S3. The pose now publishes which weapon a vehicle last fired

- **When:** slice 12 (the question came from slice 02).
- **The choice:**
  - **The problem:** a BMP-3 mounts a 100 mm gun and a Bastion missile.
    When it fired, presentation always drew the first row's flash and sound,
    so missile launches looked like gun shots.
  - **The finding:** slice 02 showed nothing already published could tell
    them apart. Vehicle rounds carry no shooter, and guessing by muzzle
    position fails.
  - **The question:** you were asked to approve a new published field. It
    went unanswered, so the integrator took the recommended option.
  - **Today:** `WeaponPose.round` is the index of the mount's last-fired
    weapon row. Presentation looks up that row's flash, kick and sound.
- **The gap:** a new published field reopens a contract you own.
- **The reach:**
  - The pose layout, the packed bytes that the native client, Wasm and
    TypeScript all decode, grew a field.
  - The publication records were re-blessed.
  - It is left out of `Mount::digest`, so battle digests didn't move.
  - Anything that later needs "which weapon fired" should read this field.
- **Verdict:** sound.
- **Confidence:** medium.

### S4. An aircraft crash row is required only when aircraft can be fielded

- **When:** slice 07.
- **The choice:**
  - **The map's wording:** admission requires a `helicopter_crash` weapon
    row.
  - **The problem:** a ground-only scenario would be refused for lacking a
    crash it can never use.
  - **Today:** the row is required only when the catalog can field an
    aircraft hull. The slice 07 child also amended the map's wording to
    match.
  - **Correction:** the slice file had credited this to you; it was the
    integrator's decision, and it is relabelled.
- **The gap:** the map didn't consider ground-only rules.
- **The reach:** test and encounter rules without aircraft load without a
  crash row.
- **Verdict:** sound.
- **Confidence:** medium.

### S5. The `ally-help` scene now flies its squad to the objective

- **When:** slice 16 (`7d86175e6`).
- **The choice:** the browser scene proves that an AI ally sends part of its
  force to help you. It had sent your lone squad about 1 km on foot, and the
  squad died before holding anything. Now the scene buys an affordable
  troop helicopter, chosen by capacity, flies the squad in, unloads it and
  sends the helicopter home. The four-minute help deadline and the
  "a detachment, not the whole force" check are unchanged.
- **The gap:** the scene's staging could never reach the state it tests.
- **The reach:** test-only. The scene still fails in the integrated branch
  for another reason, the ally never answering; that is being investigated.
- **Verdict:** sound.
- **Confidence:** medium.

### S6. The look sheet fires every look from one controlled HE gun

- **When:** slices 08 and 13; still the sheet's design.
- **The choice:** the look sheet is the review picture: launch, flight,
  impact and blast for every look. Each row fires the same review gun's HE
  round, with only the look swapped. So a row shows a look's *style*, not its
  weapon's real trajectory or whether it explodes.
- **The gap:** the plan wanted every look on one sheet; most looks have no
  weapon that fires reliably in one camera.
- **The reach:** don't judge kinetic rounds on the sheet. Slice 15 item 9
  found the sheet's exploding sabot was this carrier, not the game: real AP
  rounds publish no blast.
- **Verdict:** sound.
- **Confidence:** medium.

### S7. Visual retunes multiplied some effects' instance counts

- **When:** slice 15, items 7, 8 and 10.
- **The choice:** the joined smoke wakes that fixed the "cotton-ball puffs"
  emit more particles:
  - rocket pod: about 6×;
  - RPG: about 2.8×;
  - ATGM: about 2×.

  Average active effects stay at a few hundred, against a 16,384 capacity.
  The alternative was sparser, still-separated puffs.
- **The gap:** the plan asked for the look, not a cost bound.
- **The reach:** large rocket salvos cost more effect budget.
- **Verdict:** sound: the margin is wide.
- **Confidence:** medium.

### S8. Saved generated maps keep their historical generator pins

- **When:** slice 09.
- **The choice:** two saved generated maps pin the `config_hash` of the rules
  they were generated under (`layout-16` / `presets-18`). Slice 09's rules
  cleanup changed what that hash would be today. The pins were kept as
  history rather than rewritten, because no reader compares them against
  current rules.
- **The gap:** the plan listed "the `config_hash` the saved maps pin" among
  things that move.
- **The reach:** those pins record provenance, not validity.
- **Verdict:** sound.
- **Confidence:** medium.

### S9. Sound gains admit up to 16

- **When:** slice 05.
- **The choice:** one Rust check now admits every recipe, assignment and
  track gain in [0, 16]. The old TypeScript checks capped some gains at 2,
  but looks already ship gains up to 3.1 (the tank shot), so no single
  ceiling of 2 fits all. 16 is a loose tripwire against typos.
- **The gap:** the gains had differing bounds in different readers.
- **The reach:** an author can make a sound up to 24 dB hotter without a
  refusal.
- **Verdict:** sound.
- **Confidence:** medium-high.

### S10. A burst recording must match its weapon's rate of fire within 5 ms

- **When:** slice 15, item 4.
- **The choice:** when a battle fields a weapon, a firing recording with
  several reports must:
  - fit a weapon that has a magazine;
  - fit its capacity and burst group;
  - space its shots at the weapon's interval, within 5 ms.

  Single-report recordings always pass. The same check runs for derived and
  encounter-added weapons and in the sound workbench. The marksman rifle and
  heavy sniper therefore play one report per shot, rather than a burst
  invented for them.
- **The gap:** the plan named the check; the single-report rule and the
  workbench reach were filled in.
- **The reach:** a mod or encounter that retunes a weapon's fire rate must
  bring a matching recording, or the battle refuses it with the weapon named.
- **Verdict:** sound.
- **Confidence:** high.

### S11. Folded looks keep the median level

- **When:** slice 13. The plan delegated which rule to use.
- **The choice:** 37 temporary looks folded into the final set. Each fold's
  shot, blast, motor and contact level is the median of its members in dB,
  which moves the group's total level the least. The largest ordinary move
  is +1.74 dB. One side effect is that SAM contacts became loud enough to
  cross the ambience-duck threshold, the level above which background sound
  briefly dips. That adds five dips in the probe.
- **The reach:** future folds and retunes start from these medians, rounded
  in 15 to two decimals and whole metres, which moved levels by at most
  0.58 dB.
- **Verdict:** sound.
- **Confidence:** high.

### S12. The heavy RPG got its own look back

- **When:** slice 11 (an interim step), then slice 13.
- **The choice:** a 2026-10-16 rename left all RPGs playing the grenade's
  rows, about 11 dB quiet.
  - **Slice 11:** restored the light RPG's levels for all three RPGs, to
    avoid moving `rules_hash` there.
  - **Slice 13:** gave `rpg_at_heavy` its own look with its stranded authored
    levels, because its blast differs by 3.25 dB, beyond the 2 dB fold
    threshold.
- **The reach:** an `rpg_heavy` look ships.
- **Verdict:** sound.
- **Confidence:** high.

### S13. Canonical tank and ATGM visuals won their folds

- **When:** slice 13.
- **The choice:** where derived weapons folded into the tank or ATGM look,
  the parent's visuals won. Derived weapons that had no water or ground
  impact now show the parent's impacts. The 30 mm cannon's look is named
  `autocannon`, keeping `ifv_cannon` for the BMP-3's 100 mm gun, as
  specified.
- **The reach:** derived tank shells now splash on water.
- **Verdict:** sound.
- **Confidence:** high.

### S14. The AP-against-HE fix and the detached-muzzle fix were closed with no change

- **When:** slice 15, items 9 and 11.
- **The choice:**
  - **Item 9:** the game already tells AP and HE apart, because AP makes no
    blast. Only the sheet's carrier (S6) made the sabot look explosive.
  - **Item 11:** the reported muzzle fireball floating ahead of the barrel
    didn't reproduce on the corrected sampler. Native close-ups show the
    flame on the barrel tip.
- **Verdict:** sound.
- **Confidence:** high.

### S15. The sound workbench saves only a look's sound block

- **When:** slice 14.
- **The choice:** the workbench's Weapon looks tab sends back one sound block
  per look. The server patches only those into `presentation.json`, so the
  browser can't change visuals, rename a look or choose a file path. If an
  author changes only a gain, the recording the look shares with the mix
  stays shared, rather than being copied into the look.
- **The reach:** changing which look a weapon names stays a `rules.json`
  edit outside the workbench.
- **Verdict:** sound.
- **Confidence:** high.

### S16. Bad data is refused by name, never filled in

- **When:** slices 07, 08, 09, 10 and the whole-spec review fixes.
- **The choice:** each of these fails loudly rather than defaulting:
  - a look missing a part its weapon's physics needs (a fired round without
    tracer, impact or kick);
  - a ground hull's appearance without `track_gauge`;
  - a replay whose tick rate contradicts its encounter's;
  - a look with a zero-radius light;
  - a mix with a misspelled impact surface such as `hulll`.

  Each refusal names the row and path. A crash is allowed to have only a
  blast.
- **The reach:** content authors get errors at load, not odd visuals in
  battle.
- **Verdict:** sound.
- **Confidence:** high.

### S17. Rust is the only place presentation settings are checked

- **When:** slice 05 and the whole-spec review fixes.
- **The choice:** every presentation section, UI sections included, is a
  typed Rust struct. Rust refuses unknown fields and out-of-range values.
  The TypeScript validators for contacts, buildings, camera and HUD are
  deleted, and browser code trusts admitted values. The native client
  builds its ground and buildings from the battle's own rules, and the
  sound system reads effect timings only from the presentation document.
- **The reach:** add new presentation settings to the Rust struct; there is
  no TypeScript check to update.
- **Verdict:** sound.
- **Confidence:** high.

### S18. A vehicle's road route keeps its first grid seat

- **When:** slice 16 (`ebb78771b`).
- **The choice:** a metro balance battle failed because a vehicle's route
  skipped its starting cell's road seat. That created a connector beside a
  rotated wreck that wasn't valid. Routes now keep the first seat, and the
  existing smoothing removes it only when that's safe.
- **The reach:** a few vehicle starts route slightly differently. The proof
  digests didn't move.
- **Verdict:** sound.
- **Confidence:** high.

### Delegated discretion (one line each)

These were explicitly delegated or are internal:
- the shader-constant table keeps CPU precision (`Scalar::F32(f64)`);
- `Arsenal` builds its flight configuration from admitted rules;
- flat puff readers are strict;
- the fog uniform pads to vec4 with no compatibility slots;
- light causes were renamed `flash:<look>`;
- one-shot record migrations live outside the checkout;
- cook-off fireballs pass their own scale;
- squad launches remember the row they fired (`d00f5935e`);
- the `allies` scene frames its box pair from three times their spacing.
