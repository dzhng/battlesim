# Choices: unit models (final ledger)

The decisions in the shipped work, re-audited against the final code at
closeout. Earlier per-pass entries were consolidated: a provisional call is
shown at its end state, and anything a later pass replaced or reverted is gone.
These are choices only. Test results, measurements and review evidence live in
the commits, not here.

How to read it:

- **The user's decisions** come first. They are inputs, not audit items, and
  each is marked with its date.
- **Open for the user** lists the calls that only the user can make. Each one
  has the provisional state the work ships with, and how to reverse it.
- **Sound** is the architecture the user now owns, least confident first.

A few terms used throughout:

- **Frame**: a unit's physical box in the simulation. It is made of the hull's
  half extents, its eye height, and each mount's pivot and muzzle. It lives in
  the unit fixtures (`fixtures/units/roster/*.json`), and the simulation shoots
  and sees by it.
- **Art**: the GLB model the renderer draws over the frame. The validator
  (`asset validate`) checks that art fits its frame to within
  `tolerances.hull_extent_m` (0.1 m).
- **Dressing**: small parts named `dressing_*` (antennas, a commander's head,
  a spare wheel). They may stand outside the hull box by a set allowance, so
  they can't read as cover.
- **Tier**: one of four levels of detail. Tier 0 is close up and tier 3 the
  far view. The renderer picks a tier by how tall the unit is on screen.
- **Disabled card**: a roster card that can't be bought yet (aircraft,
  helicopters, support, drones, prototypes). It has a model and an icon, but
  no unit type in the simulation.

**Review these first.** These are the three least confident calls:
[edge wear differs by lane](#edge-wear-strength-differs-by-lane),
[wreck footprint tolerances](#each-wreck-carries-its-own-footprint-tolerance-measured-from-its-art),
and [menu units on test hulls](#menu-units-keep-the-test-hulls-so-their-art-overhangs).

## The user's decisions

These are the user's own calls, as they were given. They are listed so that a
reader knows they were chosen, not inferred.

- **Roster-only game content** (2026-10-06). The units a player sees are the
  official roster. The pre-roster generic units (`tank`, `jeep`, `supply`,
  `rifle`, `recon`, `at`) stay only as test units: "tests use fake units
  unless there is an explicit reason to use the roster; the roster changes all
  the time." They are not remodelled. The US light vehicle is the M1151
  HMMWV.
- **Convert or delete, never leave** (2026-10-06). Every place a generic unit
  or the village reached the game was turned into roster units or deleted.
- **A battle without factions is refused** (2026-10-06: "obviously invalid").
  `/battle` without a faction is an error that names the problem. It never
  silently defaults to US against Eastern.
- **No compatibility** (the user's standing default). There are no id
  aliases, no village replay reader and no storage migration. Saved replays
  from before are refused.
- **The word "village" stays as the generator's settlement class**
  (2026-10-06: "if the word village is used in this context it's perfectly
  ok"). `fixtures/map-presets.json` places villages in ordinary Play maps, so
  the class is game content and keeps its name.
- **The developer menu ships in production builds** (2026-10-06; the audience
  is technical). Its tools are labelled as tools.
- **One spec, not two** (2026-10-06). Roster-only content was briefly its
  own spec, then merged into this one.
- **Two catalogs** (option A, 2026-10-06). The game's catalog holds only the
  roster, and a separate test catalog adds test units. The user rejected one
  catalog with a `test` flag, because one missed check would leak fakes into
  the game.
- **Reuse what the generic units earned** (2026-10-06: "the generic tank is
  basically an Abrams with nice death animations"). Their tuned sounds seed
  the vehicle sound classes, and their crew idea puts crew on roster vehicles.
- **Reference photos are resized** (2026-10-06: "resize them into something
  reasonable").
- **Paint is the vehicle's real nation's, and the US is desert tan** (option
  C, 2026-10-06). The user rejected one colour scheme per faction.
- **All 86 disabled cards are remodelled, with icons** (option B,
  2026-10-06).
- **Scarce references: use what exists and generate the rest** (option A,
  2026-10-06). Missing views are generated with the latest gpt-image through
  the duet CLI, conditioned on real photos, labelled as generated, and never
  trusted over a real photo.
- **A coordinator plus a wide worker pool** (option A, 2026-10-06, "and
  parallelise even more").
- **The look is stylised strategy-game readability** (direction D of four
  generated directions, 2026-10-06). That means real layouts, bold bevels,
  simplified shapes, painted edge highlights and moderate weathering.
  Exaggeration never changes a frame.
- **Players zoom in to admire units, and the target machine is the Mac mini**
  (2026-10-06).
- **No per-unit variation yet** (2026-10-06: "can look same for now").
- **Every vehicle has its own wreck** (2026-10-06, overriding the
  recommendation of a wreck per size class): "you should be able to tell which
  unit died by looking at the wreck."
- **The menu reel is a film** (2026-10-06, overriding the recommendation). It
  may use any units with any stats, and anything may change, but every
  approved shot must stay exactly the same.
- **Every unit switches detail with zoom** (2026-10-06).
- **No number is a requirement** (2026-10-06): "numbers like the 50 MB
  download limit are arbitrary... I don't want you to spend too much time
  over-optimizing for some random number that you came up with yourself."
  Budgets, limits, ratios and allowances are loose tripwires. When one is
  exceeded and nothing visibly suffers, raise it.
- **The unknown-unknowns sweep's recommendations** (2026-10-06). The user
  agreed with these: shared texture files and a download limit; the HMMWV gets
  its real frame; dressing gets its own allowance; the battle session owns
  its catalog and the test catalog is resolved at run time; presentation is
  derived from physics and base weapons, with no new catalog field; roster
  base soldiers are abstract; `recipe_id`/`encounter_seed` leave the request;
  the name is "test unit", not "stand-in"; `market-town-test` holds the
  `assault` encounter.
- **US prototype armour** (2026-10-07: "We did include T14/T15, so we need
  prototype US"). M1E3 and M10 Booker were added as disabled cards.
- **A light tank per faction** (2026-10-07). Eastern has the Type 15 and the
  US the M10 Booker. Europe gets the CV90120, which was the coordinator's pick.
  The user then asked for the Centauro II as well.

## Open for the user

Each entry ships in the provisional state described. Nothing here blocks
anything.

### Edge wear strength differs by lane

- **When:** the pilot, then the tracked and wheeled lanes (slices 14–16).
- **The choice:** After the pilot, the coordinator asked the lanes to push
  toward the look target's painted edge highlights. Each lane picked its own
  strength for `materials(chip=)`, which sets how much of the paint's convex
  edges wear to a lighter tone. Tracked and tracked-like families use 1.0,
  wheeled families and the Strykers 0.6, and the Abrams ×3 and the HMMWV keep
  the default 0.35. Park an Abrams beside a Leopard 2 and the Leopard's
  edges read much brighter. That is a lane difference, not a real one.
- **The gap:** The look was given as an image and a word ("gentle painted
  edge highlights"), not a number, and no one owned the cross-lane value.
- **The reach:** Every future family copies one of three values.
- **Verdict:** needs-user. Provisional: as shipped. To reverse: pick one
  value, make it the `materials` default, drop the per-family overrides, and
  re-export every family (exports are repeatable, so only the art moves).
- **Confidence:** low.

### The M10 Booker reads as a small Abrams

- **When:** prototypes and light tanks (slice 20).
- **The choice:** The M10's photos show a squat turret with a long bustle box
  and a tall, blunt, bolted nose. The model has these features, but they are
  less pronounced, so at battle distance it reads as a shrunken Abrams. Its
  commander's head barely shows.
- **The gap:** Taste. The detail bar asks for "recognisable", and this
  vehicle really does resemble the Abrams.
- **The reach:** The card is disabled. If it becomes playable, players will
  need to tell an M10 from an Abrams at a glance.
- **Verdict:** needs-user. Provisional: accepted for a disabled card. To
  reverse: exaggerate the nose height and the turret's length-to-height ratio
  in `m10.py` (bevels and proportions only, inside its frame).
- **Confidence:** low.

### The Stryker Dragoon's gun sits 0.4 m low

- **When:** pilot (slice 14). Left alone at the pilot checkpoint.
- **The choice:** The Dragoon's frame puts its 30 mm gun axis at 2.19 m and
  its eye at 2.48 m (mount `autocannon`, `pivot_m` z 1.716 in
  `fixtures/units/roster/us.json`). On the real vehicle the MCT-30 turret sits
  on a 2.30 m roof, so the gun is about 0.4 m higher. Picture the model drawn
  on the common Stryker hull: the gun would come out of the roof, not out of
  the turret. So `stryker.py` draws the Dragoon's hull 0.35 m lower
  (`DRAGOON_ROOF` 1.95 against `ROOF` 2.30), and the gun leaves the turret
  correctly. The cost is that the Dragoon squats next to its sister vehicles.
  The other Stryker variants have the opposite problem: their box is 2.64 m
  tall against a real 2.30 m roof, so their roof is drawn true and the remote
  station's fixed adapter rises to fill the box.
- **The gap:** The spec forbids moving any frame to fit art, but the frame
  itself disagrees with the photos.
- **The reach:** Raising the pivot changes where the Dragoon shoots from and
  what it can see over. That moves battle outcomes and digests, so it needs a
  balance sample.
- **Verdict:** needs-user. Provisional: the frame stays and the hull is drawn
  low. To reverse: raise the pivot to about 2.10 m (gun axis about 2.6 m),
  draw the Dragoon on `ROOF`, and run a quick balance sample.
- **Confidence:** medium.

### The T-15's turret module sits 0.4 m low

- **When:** disabled ground cards (slice 18).
- **The choice:** The T-15's frame (`armata.py` `T15_DIMENSIONS`,
  `T15_MOUNTS`, copied from the old roster manifest) puts the gun pivot at
  1.82 m on a 3.5 m box. The photos show the Bumerang-BM module standing
  higher. The model is drawn on the frame, so the module sits low on the deck,
  and the commander's sight on its pedestal reaches the top of the box.
- **The gap:** The same as the Dragoon's: frames don't move to fit art.
- **The reach:** The card is disabled, so nothing in the simulation reads
  this yet. It matters only when the T-15 gets mechanics.
- **Verdict:** needs-user. Provisional: the frame stays. To reverse: restate
  the mounts in `armata.py` and re-export.
- **Confidence:** medium.

### Wheeled frames disagree with their photos

- **When:** the wheeled lane (slice 16).
- **The choice:** Each of these vehicles is drawn on its frame, not on its
  photos:
  - **LAV-25A2:** the frame's turret ring is at amidships and its muzzle sits
    0.46 m past the bow. The drawings put the ring about 0.85 m further back.
  - **ZBL-08:** the ring is about 1 m ahead of the photos' and the muzzle
    0.25 m past the bow. The roof is lowered from about 2.3 to 2.20 m so the
    gun clears it.
  - **VBCI:** the ring is about 0.4 m ahead of the photos'.
  - **BTR-82A:** the frame is 2.41 m tall with the gun axis at 2.0 m. The
    photos show a roof near 2.1 m and a turret top near 2.6 m, and put the
    ring about 1 m further back. The roof is drawn at 1.90 m under a low
    turret.
  - **Tigr-M:** the hull is 2.0 m tall against a roof near 2.45 m. The wheels
    are drawn true and the cab compressed, so it reads squat, and the gunner
    barely clears the roof.
  - **Fennek:** the hull is 2.29 m tall against a roof near 1.85 m. The roof
    is drawn true, and the folded sensor head reaches the top of the box.
- **The gap:** The same: frames don't move to fit art.
- **The reach:** Each fix is a frame change. That changes sight lines and
  hit boxes, so it moves digests and needs a balance sample.
- **Verdict:** needs-user. Provisional: every frame stays. Comments in each
  family script (`lav.py`, `zbl08.py`, `btr.py`, `tigr.py`, `fennek.py`) say
  where the art bends. To reverse: correct the frame per vehicle, re-export it,
  and sample.
- **Confidence:** medium.

### Tracked frames disagree with their photos

- **When:** the tracked lane (slice 15).
- **The choice:**
  - **Leopard 2:** the box is 3.0 m tall, but the real roof is about 2.6 m.
    The art stands at its real height inside the box, so the box is taller
    than what you see. A shot can therefore hit air above the turret.
  - **Ajax:** the frame gives the turret one mount, so the remote weapon
    station is drawn fixed to the turret and does not turn on its own.
  - **Puma:** the MELLS (Spike) launcher is a mount at 1.08 m to the left, on
    the turret's edge. Drawn there, it overhangs the side a little.
- **The gap:** The same: frames don't move to fit art.
- **The reach:** A shorter Leopard box changes what hits it. A second Ajax
  mount is a mechanics change, because it lets the vehicle fire two weapons
  on separate aims.
- **Verdict:** needs-user. Provisional: the frames stay. To reverse: change
  each frame or mount, then re-export and sample.
- **Confidence:** medium.

### May a wreck scatter debris wider than its hull?

- **When:** wrecks (slice 11). Tightened in the tracked and wheeled lanes.
- **The choice:** Picture a Challenger brewing up. In a film its armour packs
  and a track lie a few metres off. Here every wreck must stay within its
  hull's footprint plus its own measured tolerance (see
  [the wreck tolerances entry](#each-wreck-carries-its-own-footprint-tolerance-measured-from-its-art)).
  Two lane passes widened tolerances to fit thrown doors and packs. Both were
  reverted, and the debris was pulled in instead: doors land on the roof,
  crates fall into the cargo bed, and panels are tucked into the wheel gaps.
  Dressing can't carry debris either, because `wreckage.burn()` deletes every
  `dressing_*` node and the wreck footprint check measures every part.
- **The gap:** The spec says never widen a tolerance. It doesn't say whether
  a wreck's scatter is part of the physical box.
- **The reach:** A wreck is a prop the simulation can use as cover. Debris
  drawn wider than its box would show cover that isn't there, or the box
  would have to grow, which changes cover.
- **Verdict:** decided by the user (2026-10-08): "yes, maybe debris that
  scattered can just disappear?" Debris may scatter wider than the hull, as
  presentation only, and fade out after it lands; the wreck's physical box
  and cover don't grow. Built in [follow-up 02](#follow-up-02): a `debris`
  state held to its own allowance, thrown by one helper, sinking away after a
  hold.

### The NH90's width

- **When:** closeout, aircraft dimensions (slice 19).
- **The choice:** Every aircraft and helicopter card's stated length, width
  and height was checked against a published source, and four helicopters
  were rebuilt. The NH90 was left at 3.6 m wide over its sponsons. Vertipedia
  gives 4.62 m without saying what that measures. All five photos in
  `assets/references/nh90/` are the same image, so the tail can't be judged.
  The library's `gaps` records this.
- **The gap:** No source explains the number, and the references can't settle
  it.
- **The reach:** The card is disabled, so nothing reads the width yet.
- **Verdict:** needs-user. Provisional: 3.6 m. To reverse: find a second
  photo or drawing, restate the card's dimensions and re-export.
- **Confidence:** medium.

### Inherited sound choices play a burst for every round

- **When:** presentation by property (slice 01).
- **The choice:** Weapons without their own sound row now inherit their
  ancestor's (`inheritRows`, `inheritWeaponChoices`). The marksman rifle and
  heavy sniper fire single shots but inherit `rifle-combat`, a three-round
  recording. The autocannon (one round every 0.2 s) inherits `hmg-combat`
  (one every 0.1 s). So each single shot plays a whole burst. This is not
  new, because before slice 01 all three fell back to `defaults.default`,
  which is also `rifle-combat`.
- **The gap:** A fix means picking recordings, which is a call to make by
  ear.
- **The reach:** Every roster sniper and autocannon sounds wrong until this
  is fixed.
- **Verdict:** needs-user. Provisional: as is. To reverse: give
  `marksman_rifle`, `heavy_sniper` and `autocannon` their own rows in
  `fixtures/sounds.json` `defaults`.
- **Confidence:** medium.

## Sound

Least confident first.

### Menu units keep the test hulls, so their art overhangs

- **When:** the menu reel (slice 08).
- **The choice:** The menu films the old generic armies in roster looks
  (`fixtures/units/menu/units.json`). For example, `menu_us_tank` is
  `test_tank` wearing the Abrams. The plan said menu units take the roster
  hull. That was tried: the Paris Corner log stayed identical, but on Market
  Town, at tick 219, an HMG round struck the Abrams' longer hull instead of
  missing the test tank's. The battle draws every shot's scatter from one
  shared random stream, so one changed draw retimes everything after it, and
  13,820 of 18,187 events differed. So the menu units keep the test hulls, and
  the reel is the approved one bit for bit. The cost is that the drawn Abrams
  overhangs its box by 0.47 m at each end. The HMMWV overhangs by 0.9 m at each
  end, 0.25 m per side and 0.7 m above, and its drawn HMG stands about 1 m
  above the jeep's mount. The menu set is not fit-checked. The gate is
  `crates/sim/tests/menu_reel.rs` (see
  [the exact-shot test](#the-menu-reel-is-pinned-by-its-event-log)).
- **The gap:** "Every shot exactly the same" and "the roster hull" could not
  both hold.
- **The reach:** Any later change to the menu's units must keep the test
  hulls, or the reel must be re-recorded and approved again.
- **Verdict:** sound. The user's film rule outranks the plan's hull rule.
- **Confidence:** medium.

### Each wreck carries its own footprint tolerance, measured from its art

- **When:** wrecks (slice 11). Re-measured in the pilot.
- **The choice:** The live vehicle's fit check leaves out barrels and roof
  weapons, because they are mounts. A wreck is one static prop, and its
  footprint check (`footprintFindings`) measures every part. A burnt tank with
  its gun over the bow is therefore wider than its hull. So each wreck
  appearance in `assets/catalog.json` has a `tolerances.footprint_m` set at
  creation to its measured worst overhang, rounded up to 0.1 m, plus 0.1 m.
  For roster wrecks this runs from 0.3 m (IFVs) to 2.7 m. The test tank's wreck
  uses 3.0 m. The lanes then added debris, and every lane wreck was held to
  the tolerance it already had; widenings were reverted.
- **The gap:** The rule "never widen a catalog tolerance" was written for
  live vehicles. Wrecks had no tolerance until this spec made one per wreck.
- **The reach:** A wreck's tolerance is how far its art may stand outside the
  box the simulation uses as cover. A long-barrelled wreck draws metres of
  barrel that blocks nothing, which reads right because a barrel isn't cover.
  This is the base the [debris question](#may-a-wreck-scatter-debris-wider-than-its-hull)
  sits on.
- **Verdict:** sound. It measures a real property (the barrel), and it is
  fixed at creation, not raised to fit later art.
- **Confidence:** medium.

### Disabled cards were accepted at a lower detail than roster units

- **When:** disabled cards (slice 18), prototypes (slice 20).
- **The choice:** The 86 disabled cards and the four new ones each have their
  own model at their real size, their own wreck, real tiers, black tyres and
  dark glass. They are recognisable by silhouette. They carry less detail than
  the rebuilt roster units: jets have shallow fuselages and small canopies,
  truck cabs are boxes with windows, drones are simple primitives, and there
  are no markings or insignia. The first things to raise when a family becomes
  playable: fuselage depth and canopy size for jets, hull detail and truck cabs
  for ground support, and the M10's nose and turret and the M1E3's remote
  station.
- **The gap:** The detail bar was written for roster vehicles that players
  zoom in on, but disabled cards only show in the picker and the icons.
- **The reach:** Making a card playable includes bringing its model up to
  the roster bar.
- **Verdict:** overruled by the user (2026-10-08): "Disable cards needs to be
  same detail bar - they will be implemented next". Every disabled card is to
  be raised to the roster bar; the next spec owns that work.

### The M10 is built to 7.0 × 3.4 × 2.9 m, the middle of sources that disagree

- **When:** prototypes and light tanks (slice 20), corrected after closeout.
- **The choice:** The M10 was first built 8.10 m long, measured off a photo
  by guessing the size of its road wheels. That made it a metre too long, and
  its library quoted a third set of numbers. Nobody publishes official
  figures (GDLS never did, and the programme was cancelled in 2025). The
  sources that do disagree: EDR Magazine and MilitaryFactory say 6.85 m long,
  2.4 m wide and 3.65 m tall, and Overt Defense says about 9 m with the gun
  forward, 3.2 m wide and 2.5 m tall. So `m10.py` now builds to 7.0 m long
  (the 6.85 m hull plus its towing lugs), 3.4 m wide (over the skirts; 2.4 m
  is narrower than the ASCOD hull it grew from, and the front photos show it
  wider) and 2.9 m tall (to the top of the commander's sight). Everything
  else is laid out in proportion to that length off the side photo, not
  shrunk as a block. That gives six road wheels a side, each 0.60 m across at
  a 0.82 m pitch, the turret amidships with its roof at 2.5 m (Overt's
  height), the gun axis at 2.10 m and the muzzle 1.45 m past the nose. The
  library's `gaps` cites every source and how they disagree.
- **The gap:** No source is authoritative, so the frame is a judgement, not a
  quote. 3.65 m only fits if the M2 and mast are counted.
- **The reach:** When the card gets mechanics, its unit type's frame should
  start from these numbers and from the reasons given in the library.
- **Verdict:** sound. The script and its library now give the same numbers,
  and the library says where each one comes from.
- **Confidence:** medium.

### Disabled-card frames are stated per card and held to 6%

- **When:** disabled cards (slice 18), closeout (slice 19), review.
- **The choice:** A disabled card has no unit type, so it has no frame in the
  fixtures. Each family script states its card's length, width and height as
  `DIMENSIONS` (and `MOUNTS` for turreted cards).
  `vehicle_export.run_disabled` refuses a model more than `FRAME_TOLERANCE`
  (0.06) off any dimension. Six cards (T-14, T-15, Type 15, BRM-3K, Jaguar,
  Challenger 3) copied their values from the old roster manifest. The rest
  took them from published figures or measured them off a photo. The aircraft
  were checked one by one against published sources at closeout, and four
  helicopters more than 5% off were rebuilt (AH-64E, AH-1Z, UH-1Y, Mi-35M).
  Each card's library `gaps` names its source. The tolerance was 0.05 and was
  raised to 0.06 because the M-SHORAD and M1129 stand on the pilot Stryker
  body, which is 5.1% over its stated width. Rotorcraft are measured without
  blades, to the top of the rotor head.
- **The gap:** Disabled cards had no physical source of truth.
- **The reach:** When a card gets mechanics, its frame comes from the catalog
  like any unit's. Then these numbers become a first draft, not an authority.
  The six copied cards' receipts still say the frame came from the archived
  manifest until they are next exported.
- **Verdict:** sound. One owner per card (its script), checked against a
  cited source.
- **Confidence:** medium.

### Drones are drawn as the type each army fields

- **When:** disabled ground cards (slice 18).
- **The choice:** Drone cards name a class, not a type. Scout drones are
  drawn as the Skydio X2D (US), Parrot ANAFI USA (Europe) and a DJI Mavic 3T
  class (Eastern). Anti-armour loitering munitions are the Switchblade 600,
  Hero-120 and Lancet. Anti-personnel ones are the Switchblade 300 and, for
  Europe and the East, an FPV quadcopter with an RPG warhead. Drones stand on
  the ground with their rotors still.
- **The gap:** The cards don't say which airframe.
- **The reach:** A later mechanics spec may pick other types and re-model
  them.
- **Verdict:** sound. Reversible.
- **Confidence:** medium.

### The four new cards' costs, families and labels

- **When:** prototypes and light tanks (slice 20).
- **The choice:** The M1E3 sits in the Abrams family as an unavailable
  variant, like the Challenger 3 in the Challenger's, at cost 460 (the SEPv3
  Trophy is 450). The M10 Booker is its own family at 240 (the Type 15 is 230).
  The CV90120 is its own family `cv90120` at 250, so the picker shows Europe's
  light tank apart from the CV90 IFVs. The Centauro II is its own family at
  240. A new planned label `tank-gun` covers a 120 mm gun on a light chassis,
  beside `light-tank-gun` for 105 mm. The paint is US tan for both US cards
  (the photos show green prototypes), Swedish splinter for the CV90120, and a
  new `italian_vegetata` scheme for the Centauro. Each card's mounts are stated
  from its photo, so the turret traverses and the wreck can throw it.
- **The gap:** The user named the vehicles, not their place in the ladder.
- **The reach:** A disabled card fights no battle, so the costs and labels are
  read only by the picker's dimmed card and by a later mechanics spec.
- **Verdict:** sound.
- **Confidence:** medium.

### Which army wears which uniform and rifle

- **When:** the infantry pilot (slice 14) and lane (slice 17).
- **The choice:** Europe wears the Bundeswehr's look: Flecktarn, G36, and the
  G28 for marksmen. Europe's cards come mostly from Germany, and the Leopards'
  three-tone green agrees. US cards split by service. The Marine squad, Force
  Recon and the M107 snipers wear MARPAT with the M27. Army scouts, the TOW team
  and the M110 squad wear OCP with the M4A1. Every Eastern kit wears Russian
  EMR with the AK-12, including the FN-6 team, because no Chinese print exists.
  Team weapons were kept, because their silhouettes already matched the
  references.
- **The gap:** The roster names cards, not uniforms.
- **The reach:** A new army or card picks a row in
  `infantry_equipment.py` `ARMIES`/`KITS`.
- **Verdict:** sound.
- **Confidence:** medium.

### Paint where the photos disagree with "the real nation"

- **When:** materials (slice 13) and the lanes.
- **The choice:** One look per variant, in its nation's scheme. Some calls
  were needed. The Bradley is US tan, though the M2A4's European photos show
  green. Both CV90s are Swedish splinter. Exhibition finishes are not followed
  (the BRM-3K's and 2S19's desert camouflage, the Skyranger's digital
  demonstrator scheme). Russian green was darkened to an olive
  (0.058, 0.068, 0.043) because the first value read lime under the game's
  warm sun. Running gear (hubs, louvres, stowage) stays dark olive and
  untinted on every scheme. Each library's `gaps` names what the photos
  contradict.
- **The gap:** "The real nation's" doesn't pick between a nation's several
  schemes.
- **The reach:** Schemes are data (`textures.SCHEMES`), so a new one is one
  recipe.
- **Verdict:** sound.
- **Confidence:** medium.

### Generated views stand in where no licensable photo exists

- **When:** the Challenger 3 (slice 18) and the M1E3 and CV90120 (slice 20).
- **The choice:** The Challenger 3 has no photo under an allowed licence, so
  its library holds two Challenger 2 TES photos of the hull it keeps, plus two
  generated views of the new turret. The M1E3 has one public-domain photo,
  plus the SEPv2/SEPv3 hull photos and two generated views. Where a generated
  view disagreed with the photo (six skirt panels against four), the photo
  won. The CV90120 has one photo and one generated quarter view. The
  reference contract refuses a view that has only generated images, which is
  why the real hull photos stand beside them.
- **The gap:** Which views to generate, and from what.
- **The reach:** The pattern for any future family with scarce photos.
- **Verdict:** sound. It follows the user's option A.
- **Confidence:** medium.

### The wheeled medium class gets a turret loop

- **When:** presentation by property (slice 01).
- **The choice:** The plan said `wheeled_medium` takes the supply truck's
  sound row, which has no turret loop. But most of its hulls (Stryker, BTR,
  Boxer) have turrets, and they would traverse in silence. The row in
  `fixtures/game.json` `presentation.audio.vehicles.wheeled_medium` plays the
  shared `turret` loop at gain 0.08, between the light (0.05) and tracked
  (0.12) classes.
- **The gap:** The plan's row choice missed the turrets.
- **The reach:** The physical rule would be a turret loop whenever the hull
  has a turret mount. That is a later sound pass, retuned by ear.
- **Verdict:** sound.
- **Confidence:** medium.

### Stress battles carry factions they ignore

- **When:** the request contract (slice 07).
- **The choice:** Every `PrepareBattleRequest` now needs `factions`. The
  benchmark's `city-contact` and the endurance lab prepare a stress scene on
  standard (non-skirmish) geography. The old contract check "factions need
  skirmish geography" would have refused them, and changing their map would
  change the benchmark's workload. So the check moved to preparation and
  became a property of the resolved map: a map with no admitted blue skirmish
  base is refused at stage `encounter`. Stress requests carry
  `factions: ["us", "eastern"]`, which the stress scene ignores.
- **The gap:** Stress runs aren't battles, but they used the battle request.
- **The reach:** The alternative was a separate stress message (a larger
  change to the worker, client, report and scenes). If stress gets its own
  message later, the dummy factions go.
- **Verdict:** sound.
- **Confidence:** medium.

### The street test map is the old village ground under a test name

- **When:** retiring the village (slice 06). Extended at closeout.
- **The choice:** `fixtures/maps/street` is the village's ground, byte for
  byte, labelled `category: "test"`. Its encounters are `street` (the fog
  labs' fight), `attack` (the old "ordinary" variant as data), `advance`
  (`attack` plus one opening move, so it plays itself), `lean`, and `rear`
  (`street` with soft vehicles pulled out of sight, so each side holds ground
  the other never saw, for the `ground` scene's check). Moving these checks to
  another map would have re-staged every coordinate-anchored scene (26 tours,
  the ground rig, the fog labs' camera).
- **The gap:** The plan said delete the village, but its ground anchored most
  browser checks.
- **The reach:** The old arena survives as test content. Nothing of it
  reaches the game, and `git grep -i village` lists only the settlement class
  and history.
- **Verdict:** sound.
- **Confidence:** medium.

### Scenes and tests were fixed in the scene, not in product code

- **When:** closeout test and scene fixes.
- **The choice:** Web tests that render a page now await its catalog set
  (`web/tests/support/router.tsx` `renderSettled`), because a synchronous
  `act` never retries a `use()` that suspended. The alternative was marking
  the set's promise fulfilled in `sets.ts`, which would put product machinery
  in for a test-environment property. Tests that faked WebAssembly use the
  real loader. The `contacts` scene waits up to 5 s for a callout that
  arrives one animation frame after its observation; a missing one still
  fails. The `workbench` scene serves its synthetic bake under `test_tank` and
  `test_rifle`, because the workbench loads only art its test units wear. The
  `generated` scene asks for the skirmish profile, and its town spiral circles
  a building that has a neighbour.
- **The gap:** Each check pinned something a slice changed on purpose.
- **The reach:** None in product code.
- **Verdict:** sound.
- **Confidence:** medium.

### Trucks stay off the support-truck chassis

- **When:** closeout (slice 19).
- **The choice:** `roster/truck_chassis.py` draws the support cards' trucks:
  rails, axles, a fender over every wheel, and one of three generic cabs. The
  HEMTT, MAN HX and Ural have reference-built cabs, their own rails and axles,
  and bar treads. Only the wheel-axle-rail loop is common, and it differs in
  every dimension. Standing them on the chassis would change their art, and a
  shared loop with a parameter per dimension would not be simpler.
- **The gap:** The closeout asked for one truck owner "if the wheeled trucks
  can stand on it".
- **The reach:** Two owners of truck geometry remain, on purpose.
- **Verdict:** sound.
- **Confidence:** medium.

### Look-alike helpers in family scripts were left

- **When:** whole-spec review.
- **The choice:** `glacis_z` (×12), `wheels`, `trophy_station`, `gun_120` and
  `remote_station` share names across family scripts but differ in every
  number. Six hand-drawn spare wheels sit beside `truck_chassis.spare_wheel`.
  `aircraft_parts.fit` has its own `dark` material. `wreckage.TURRET`/`BARRELS`
  restate `vehicle_export.RIG_NODES`' names, because wreckage cannot import the
  exporter. Folding any of them would re-export art for no visible change.
- **The gap:** The one-owner rule versus "don't move art for no reason".
- **The reach:** A future family copies whichever local helper is nearest.
- **Verdict:** sound.
- **Confidence:** medium.

### Catalog sets are declared twice, in Rust and TypeScript

- **When:** session catalog (slice 04), test units (slice 05), review.
- **The choice:** Which folders each set adds is written in
  `crates/sim/src/fixtures.rs` `CatalogSet::own_roots` and in
  `web/src/battle/catalog/compose.ts` `SET_FOLDERS` (test: `units/test`; menu:
  `units/test`, `units/menu`). Each names the other. The browser's Vite globs
  in `sets.ts` must be literal, so a shared data file would still leave a
  third copy. `sessionCatalog.test.ts` checks that a page and a native tool
  resolve the test and menu sets to the same units.
- **The gap:** Vite can't glob from data.
- **The reach:** Adding a set means touching three places, and the test
  catches a mismatch.
- **Verdict:** sound.
- **Confidence:** medium.

### Faction colour at long range is the side tint's job, not the uniform's

- **When:** infantry pilot and lane (slices 14, 17).
- **The choice:** At the opening camera a soldier is 1–2 px. Beyond about
  250 m, soldiers draw as impostor cards (flat pictures baked from the model),
  and those already carry the uniform's average colour. No bake or tint change
  makes a 2 px mark show a camouflage, and brightening prints past their real
  colour would break the close-up. So the work does nothing at that range. Red's
  side tint also warms EMR toward olive. That is left, because the tint is the
  battle's rule.
- **The gap:** The bar said armies read apart, without saying at what range.
- **The reach:** Telling who a far mark belongs to stays with the side tint
  and the callouts.
- **Verdict:** sound.
- **Confidence:** medium.

### Black rubber and dark glass are held by measured brightness

- **When:** materials (slice 13).
- **The choice:** Every material names a role in glTF `extras.role` (`rubber`,
  `glass`, `paint`, `steel`, `track`, `fabric`, `skin`, `marking`;
  `MATERIAL_ROLES` in `schema.ts`). The validator computes what a surface
  draws, averaged over its triangles' area, the way the model shader does.
  Rubber fails above luminance 0.035 (`RUBBER_MAX_LUMINANCE`), and glass above
  0.025 or above roughness 0.3. Measuring showed the old tyres were already
  near black on average. What read grey on screen was the painted rim
  covering 55–70% of the wheel face, which is geometry, and the rebuilds fixed
  it. The bound allows a little dust and refuses a tyre filmed with dust all
  over. Leather and polymer take no role.
- **The gap:** "Tyres are black" needed a measurable rule.
- **The reach:** Every future model is held to it, and an unknown role is an
  error.
- **Verdict:** sound.
- **Confidence:** medium.

### Dressing allowances

- **When:** art rules (slice 12).
- **The choice:** Dressing is an empty node named `dressing_*` with its meshes
  under it. Bulky dressing may reach 0.3 m past any hull face (`bulky_m`). Only
  thin parts (at most 0.15 m across, `thin_m`) may rise further, up to 4 m over
  the roof (`thin_top_m`), so there is one antenna per dressing node. It is
  measured at rest on tier 0, like hull fit. The 0.3 m sets how far a
  commander stands out of a hatch (head and shoulders), and that reads right.
  The old `hull_top_m` override is gone, so hull fit has one tolerance on
  every face.
- **The gap:** The numbers.
- **The reach:** Tripwires (the user's rule). Raise one if a real part needs
  it and still can't read as cover.
- **Verdict:** sound.
- **Confidence:** medium.

### The HMMWV's new frame numbers

- **When:** the pilot (slice 14). The frame change itself was agreed by the
  user in the sweep.
- **The choice:** The HMMWV wore the JLTV's model and frame. Its hull is now
  4.9 × 2.2 × 2.0 m (half extents [2.45, 1.1, 1.0]), eye 2.42 m, HMG pivot
  2.28 m, muzzle [1.35, 0, 0.32]. The eye, pivot and muzzle are the old
  manifest's. The size comes from the references (a side drawing scaled by
  the 37-inch tyre, and the published 193 × 87 in). The box top is the cab roof
  plus the turret ring. The gun shields turn with the gun, so they belong to
  the mount, outside hull fit. The manifest's 5.2 m length counted the rear
  spare wheel, which is now dressing. A quick sample of static duels showed no
  balance concern.
- **The gap:** Which sources to trust for the numbers.
- **The reach:** The digest of any battle with an HMMWV moved. No committed
  record held one.
- **Verdict:** sound.
- **Confidence:** medium-high.

### No full balance run at closeout

- **When:** closeout (slice 19).
- **The choice:** The only roster physics change is the HMMWV frame, which
  was sampled in the pilot. The repo's balance workload (`battle_sweep`) plays
  test units, which were renamed but not changed. AGENTS.md runs only what a
  change can move, so the full report was skipped.
- **The gap:** The closeout listed "run the balance report once".
- **The reach:** If a later change moves roster physics, the report runs then.
- **Verdict:** sound.
- **Confidence:** medium.

### Kits: one table, scouts keep their ruck, disabled teams get one look

- **When:** the infantry lane (slice 17) and disabled infantry (slice 18).
- **The choice:** `infantry_equipment.py` `KITS` and `LENGTHS` replace the old
  infantry manifest. Each kit names its weapon, whether its men are scouts or
  carry grenades, and its armies (the first is the default look, the rest are
  faction looks). A scout's ruck and boonie win over his army's hydration pack
  (`infantry_kit.look_of`). The five disabled launcher teams share one
  `shoulder_tube_equipment` builder and get only active look `a`, with no
  carried kit, because nothing draws one yet.
- **The gap:** How to structure the kit data once the manifest went.
- **The reach:** A new kit is one `KITS` row.
- **Verdict:** sound.
- **Confidence:** medium-high.

### Truck wheel counts come from the frame, not the photos

- **When:** the wheeled lane (slice 16).
- **The choice:** The MAN HX's licensable photos mix 4x4 and 6x6 trucks. The
  frame's 10.34 m length is the HX77 8x8's, so it has eight wheels, and the
  photos settle only the cab and body. The Ural's axle positions are the side
  photo's, not the old helper's.
- **The gap:** The photos disagreed with each other.
- **The reach:** None beyond these two models.
- **Verdict:** sound. The frame is the authority.
- **Confidence:** medium-high.

### The menu reel is pinned by its event log

- **When:** the menu reel (slice 08).
- **The choice:** `crates/sim/tests/menu_reel.rs` replays each backdrop scene
  natively, using the menu set, its seed, and the reel's last tick. Its event
  log (fire, hit and what was hit, fell, died, by tick) must equal
  `crates/sim/tests/fixtures/menu-reel/<map>-<encounter>.log` line for line,
  after recasting the filmed `test_*` cast to the menu units. A followed unit
  must be alive when its shot opens. "Alive throughout" would fail the
  approved cut, which has unit 0 cook off mid-shot. `BLESS_MENU_REEL=1`
  re-records, for an approved reel only.
- **The gap:** How to prove "every shot exactly the same" cheaply.
- **The reach:** Any change to rules the menu battle uses fails this test.
  That is the point: it means re-approving the reel.
- **Verdict:** sound.
- **Confidence:** high.
- **Re-approved:** user, 2026-10-08: main's sight-gap rule closes the gate
  tank 1 watched squad 36 through; the reel stays exact through shot 12 (the
  missile scene included), squad 10 shifted 0.5 m E / 2 m N and tank 1's
  planning paced by scripted moves; shots 13 onward re-approved as they now
  play. The Market Town log is unchanged through line 13150; it first differs
  at tick 2625 (`2625 fire 3 hmg`).

### Faction looks on one soldier appearance

- **When:** the infantry pilot (slice 14). Used by the lane.
- **The choice:** All three factions' rifle squads are one roster card and
  draw one soldier set. A soldier appearance in `assets/catalog.json` may name
  a look per faction (`"factions": {"eastern": "rifle_squad_eastern_active_a"}`).
  The renderer's `AppearanceCatalog` takes the session's side factions and
  draws the look of the faction each side fights for. A battle without
  factions (labs, the menu) draws the base look. A faction look loads with its
  base and is validated as a soldier on the same skeleton. The alternative,
  per-faction roster cards, would move the roster and digests.
- **The gap:** The plan assumed a card's look was per card.
- **The reach:** Presentation only. Any shared card can wear a look per
  faction.
- **Verdict:** sound.
- **Confidence:** high.

### Crew come from the faction's soldier, one tier coarser

- **When:** pipeline (slice 09). Faction looks added later.
- **The choice:** `vehicle_crew.CREW_SOLDIER` maps each faction to a rifle
  squad look. Vehicle tier t carries the soldier's tier t + 1, and tier 3
  carries no crew. The soldier's weapon is cut away: every piece skinned
  wholly to the `hand_r` joint goes, because the rifle is joined into the one
  skinned mesh. Crew vanish at the wreck. A family's crew pick up a new look on
  its next export.
- **The gap:** How to put soldiers on vehicles without a second soldier
  model.
- **The reach:** A soldier kit change re-exports every crewed vehicle. That
  happened at closeout: 22 vehicles changed bytes.
- **Verdict:** sound.
- **Confidence:** high.

### Exports are repeatable, and Blender runs single-threaded

- **When:** wheeled lane, closeout (slice 19).
- **The choice:** Every family and kit, live and wreck, exports to
  byte-identical files twice from the same code. Three causes were fixed in
  shared code. `wreckage.heat()` builds its noise from three scalar
  `noise.noise` samples, because `noise_vector` differs between Blender
  processes. `mesh_lods` orders edges and copies weights and colours from the
  nearest original vertex, because decimation's last-bit float noise flipped
  rounding. `asset blender` runs with `--threads 1`, because threaded tangents
  straddled the glTF exporter's rounding. The cost is time: an Abrams wreck
  takes about 50 s.
- **The gap:** None. Repeatability was a closeout item, and these were the
  causes.
- **The reach:** A re-export that changes bytes now means the code changed.
  The test art under `assets/source/test/` uses the same tiers but wasn't
  re-exported, so it will change bytes once on its next export.
- **Verdict:** sound.
- **Confidence:** high.

### Running gear is judged by material, not by node name

- **When:** closeout (slice 19).
- **The choice:** The validator (`articulatedFindings`) finds what touches the
  ground (tier 0, within `ground_m` of the lowest point). It requires every
  part whose material role rolls (`rubber`, `track`) to sit under a `wheel_*`
  or `track_*` node, which the renderer turns. Skids, legs, rails and a belly
  only rest, so they need no node. This replaced a rule that demanded a
  `wheel_*` or `skid_*` node, which six belly-landing drones failed. A wheeled
  type must still have `wheel_*` nodes. Nothing reads `skid_*` any more, though
  some airframes still name parts that way.
- **The gap:** A body that lands on its belly.
- **The reach:** One physical rule for every vehicle, aircraft and drone.
- **Verdict:** sound.
- **Confidence:** high.

### Budgets are about twice what was measured

- **When:** art rules (slice 12), pilot (slice 14).
- **The choice:** `packages/scene-assets/src/unitArt.ts` `UNIT_ART` holds one
  row per vehicle class and one for soldiers: triangles per tier, encoded
  bundle bytes, and distinct textures. Each row is about twice the largest
  measured unit of its class. Each tier draws at most 0.9 of the one before
  (`tier_ratio`, for every class). A mesh copied to every tier is refused.
  Aircraft have no class, so they were checked by hand against the
  tracked-heavy row as a ceiling. The pilot measured that a vehicle's far tier
  costs nothing noticeable on the GPU, so vehicles have no impostor.
- **The gap:** The numbers.
- **The reach:** Tripwires (the user's rule). Raise one when real art exceeds
  it and nothing suffers.
- **Verdict:** sound.
- **Confidence:** high.

### Download and texture limits

- **When:** transport (slice 10).
- **The choice:** The catalog load is held to 512 MiB
  (`CATALOG_LOAD_MAX_BYTES`), about four times the measured load when it was
  set. A map's on-request download is held to 256 MiB
  (`MAP_DOWNLOAD_MAX_BYTES`). Texture layers are held to 2048
  (`TEXTURE_ARRAY_LAYERS_FLOOR`), which is what the Mac mini's adapter
  reports. Each page's GPU device requests the adapter's limit through
  `renderer-core` `requestGpuDevice()`, because the default 256 layers would
  break as textures grow. The bake counts layers over every appearance, an
  upper bound on any page.
- **The gap:** The numbers.
- **The reach:** Tripwires. A page that grows past one is refused at bake or
  load.
- **Verdict:** sound.
- **Confidence:** high.

### One transport: shared texture files and gzipped bundles

- **When:** transport (slice 10).
- **The choice:** Every bundle (skeleton clips, units, scenery, kits) travels
  gzipped with its textures taken out. Each texture is its own
  content-addressed file, fetched once and shared. A bundle's travelling form
  has its own magic (`BGAT`), so it can never be read as a bundle. On every
  read the textures are joined back and the whole encoding is hashed, so the
  art's identity is the same hash as before. The catalog's `textures` table
  lists each bundle's texture ids, so a page's download can be counted from
  the catalog alone.
- **The gap:** How to split textures out without a second identity.
- **The reach:** Every bundle and texture hash stayed the same. A new bundle
  kind travels the same way.
- **Verdict:** sound.
- **Confidence:** high.

### A page loads only the art its units wear, and widens rather than reloads

- **When:** session catalog (slice 04), wrecks (slice 11).
- **The choice:** `schema.ts` `catalogLoadNames(catalog, wearing)` is the one
  selection of what a page fetches. It takes scenery, clips, the template
  library, and only the unit art the session's units wear (`UnitCatalog.appearances`),
  with each worn vehicle's wreck and each worn soldier's faction looks. A game
  page fetches no test art. When a second session on the same page needs more
  (a lab after the game), `AppearanceLibrary.withUnits` adds only what's
  missing. The alternatives were a library per set (everything fetched twice)
  and a reload per set.
- **The gap:** How a page with two sessions shares art.
- **The reach:** A worn appearance that isn't baked draws nothing, as before.
- **Verdict:** sound.
- **Confidence:** high.

### The battle session owns its catalog

- **When:** session catalog (slice 04), test units (slice 05).
- **The choice:** There is one resolver (`sim::fixtures::catalog_documents`)
  over three document sets: game (committed as `fixtures/catalog.json`),
  test (`+ fixtures/units/test/`) and menu (`+ units/test`, `units/menu`). The
  last two are resolved at run time. In the browser, only
  `web/src/battle/catalog/` imports `fixtures/catalog.json`. The router scopes
  `/battle` and `/replay` to the game set, every lab and workbench to the test
  set, and the menu backdrop to the menu set. UI reads `useSessionCatalog()`,
  and renderer and audio code take the catalog as an argument. A scenario
  naming a unit its set lacks throws by name instead of drawing nothing.
  Weapons are the game's in every set. The mechanics editor still edits every
  file under `fixtures/units/`, test units included, and lists them under
  "Test units".
- **The gap:** Where the set is chosen, and what happens on a mismatch.
- **The reach:** Every page and tool names its set. A test-set page always
  loads the WebAssembly resolver.
- **Verdict:** sound.
- **Confidence:** high.

### Test units are named, fenced and kept

- **When:** test units (slice 05).
- **The choice:** `fixtures/units/test/` holds `test_tank`, `test_jeep`,
  `test_supply`, `test_rifle`, `test_recon`, `test_at` and their soldier kinds,
  with art under `assets/source/test/`. A test
  (`catalog::the_game_catalog_holds_only_what_roster_cards_reach`) holds the
  game catalog to units an enabled card reaches. Native tests call
  `fixtures::test_game()`. Two exceptions are stated in the tests: deployment
  checks the roster Kornet, and parity tests whose browser half runs a lab use
  the test set. Parity records that hash type ids were regenerated, and no
  battle digest moved. Synthetic fakes inside tests keep their own names.
- **The gap:** Naming and the exceptions.
- **The reach:** A roster edit moves no test, scene or benchmark.
- **Verdict:** sound.
- **Confidence:** high.

### Roster soldiers stand on their own abstract base kinds

- **When:** roster stands alone (slice 02).
- **The choice:** `fixtures/units/roster/shared.json` holds `roster_rifleman`,
  `roster_grenadier`, `roster_scout` and `roster_at_rifleman`. The last three
  extend the first, as the generic kinds did, so mount order is unchanged.
  Base kinds carry no appearance. "No change" was proven by diffing the
  resolved cards, not by a parity file.
- **The gap:** Where the bases live and what they carry.
- **The reach:** A test unit edit no longer reaches a roster squad.
- **Verdict:** sound.
- **Confidence:** high.

### Vehicle presentation class is composed from physics

- **When:** presentation by property (slice 01).
- **The choice:** `packages/scene-assets/src/units.ts` `vehicleClass` returns
  `<tracked|wheeled>_<weight_class>`, plus `_logistics` for a hull with the
  `logistics` role (`tracked_heavy`, `wheeled_medium_logistics`). Sound loops
  and art budgets are keyed by it, and a class with no row takes `default`.
  Weapon sound and effect rows inherit along `extends` (`kindTable.inheritRows`),
  so derived roster weapons find their base's row. Per-unit sound overrides
  went, and with them the unit and mount identity in the effect publication,
  which only they read. The sound catalog refuses an unknown section.
- **The gap:** Names and the fallback.
- **The reach:** A new vehicle gets sounds and budgets from its physics with
  no table edit.
- **Verdict:** sound.
- **Confidence:** high.

### What a wreck is, and how it finds its art

- **When:** wrecks (slice 11). Finished in the review.
- **The choice:** A prop definition carries `wreck_of`, a unit type id. The
  published prop carries `wreckOf`, an index into `unitKinds` (-1 for none),
  and the world export carries the same. A prop is a wreck when its type is
  drawn by `wreck`. The world refuses a wreck without `wreck_of` and any other
  prop with one. The simulation reads `appearance.drawn_by` only for that
  admission check, which belongs to the presentation contract. A heavy wreck
  burnt down to remains keeps its unit. The renderer resolves `wreckOf` to
  the unit's appearance and then to that appearance's `wreck`. Nearest-size
  matching is gone. Cook-offs match a lost hull to its wreck by unit type, and
  throw a turret only when the unit's own wreck has hull and turret pieces.
  Every vehicle's wreck is modelled from its own build (`wreck` is required in
  `vehicle_export.run`), and `wreckage.burn()` is its last step. Test units keep
  their own wrecks, and map-placed wrecks in tests name the test unit.
- **The gap:** How the simulation and renderer agree on whose wreck it is.
- **The reach:** `wreck_of` derives from the dead unit's kind, which is
  already in the digest, so no digest moved. A leftover: `standsIn` treats
  every wreck kind as artless, so a page with stand-in styles also loads the
  tiny stand-in kit.
- **Verdict:** sound.
- **Confidence:** high.

### New art rules are errors from the start

- **When:** art rules (slice 12).
- **The choice:** Every new validator rule (`structure.tier_unsuffixed`,
  `structure.tier_ratio`, `fit.dressing`, `budget.*`, `material.role_*`) is an
  error, with no warning tier and no expected-failure list. For units and
  wrecks, a mesh not named for its tier is refused rather than copied to every
  tier. Scenery and kits keep that convention. Tier rules live in `unitArt.ts`,
  not in the build. An appearance drawn by types of several classes is held to
  each class's rules.
- **The gap:** How strict to be while lanes were still rebuilding.
- **The reach:** A bake can't let a regression through.
- **Verdict:** sound.
- **Confidence:** high.

### One export path and one parts library

- **When:** pipeline (slice 09), pilot (slice 14), lanes.
- **The choice:** Exporters read each variant's frame from the resolved
  fixture catalog (`blender/catalog_frames.py`), never from the archived
  manifests. Disabled cards state their own (see above). Every family is a
  `build`/`wreck` pair through `vehicle_export.run` or `run_disabled`.
  Reusable parts live in `vehicle_parts.py` (`tracked_running_gear`,
  `track_loop`, `road_wheel`, `tyre_wheel`, `bolted_panel`, `browning_m2`,
  `kord`, `hull_plan`, …) and `aircraft_parts.py`. A family derived from
  another imports it (`t90.py` from `t72.py`, `m1e3.py` from `abrams.py`,
  Skyranger on `boxer.drive`), and the imported script exports only when run
  directly. Every legacy family helper (`armor.py`, `light_armor.py`,
  `europe_carriers.py`, `eastern_armor.py`, `remaining_*`, `logistics.py`) is
  deleted. A receipt lists every script the export loaded.
- **The gap:** The structure of the scripts.
- **The reach:** A new family is one script.
- **Verdict:** sound.
- **Confidence:** high.

### Reference libraries are committed and licensable only

- **When:** pipeline (slice 09). Licences widened in materials (slice 13).
- **The choice:** Each family has `assets/references/<family>/` photos plus a
  `references.json` with `{entries, gaps}`. Licences are SPDX ids (CC0, CC BY,
  CC BY-SA, including country ports such as `CC-BY-SA-3.0-DE`) or
  `public-domain`, plus `ours` for generated views. A port of a disallowed
  licence (NC) stays refused. Images are 1600 px on the long edge. They are
  review inputs only (`asset sheet --references`), read by no runtime, bake or
  test, and `asset check` holds each library to its contract. A family without
  a library fails.
- **The gap:** Schema and licence ids.
- **The reach:** Every future family collects its own.
- **Verdict:** sound.
- **Confidence:** high.

### Disabled cards are held to their own model and library

- **When:** disabled cards (slice 18), closeout (slice 19).
- **The choice:** `fixtures/units/model-manifest.json` has a `model_statuses`
  registry, which today holds only `reference_built`. The disabled gate in
  `crates/sim/tests/catalog.rs` checks each card's status. It checks that its
  `source_path` is a real binary glTF (or an LFS pointer to one), that no
  other card shares the path or the bytes, and that its reference library
  exists in its source's folder. The placeholder generator and its status are
  deleted. A model never makes a card available. Only the resolved catalog
  decides that.
- **The gap:** How to stop a rebuilt card being overwritten or shared.
- **The reach:** A new disabled card needs its own model and library.
- **Verdict:** sound.
- **Confidence:** high.

### Icons are keyed by card and drawn from models

- **When:** disabled cards (slice 18).
- **The choice:** `icons.ts` `cardIcon(id)` names every card's icon. Each
  disabled card's silhouette is rendered from its model, and a team is one
  posed soldier at its skeleton's aim pose, because a disabled card has no
  slots to count. `asset check` fails a missing or stale icon. The purchase
  picker shows a family's icon (from its first available card, else its first
  card), and shows an all-unavailable family dashed and dimmed.
- **The gap:** Disabled cards had placeholder icons.
- **The reach:** Icons are derived. Re-derive them, never hand-edit.
- **Verdict:** sound.
- **Confidence:** high.

### The request contract and links refuse what they don't know

- **When:** request contract (slice 07), retiring the village (slice 06).
- **The choice:** `PrepareBattleRequest` is
  `{ map_source, factions: [Faction; 2], battle_seed }` with unknown fields
  refused. A saved-map source (`MapSource::Catalogue`) is gone. A `/battle`
  link refuses any parameter outside its list, a missing `faction`, and a
  profile other than `skirmish`. `enemy` stays optional and is resolved in
  `askedBattle`. Tools that judged the `assault` recipe name it themselves.
  Each fixed map's `meta.json` has `category` `test` or `menu`, and every
  reader takes only its category. Replays are one format, and the IndexedDB
  key became `last-replay` with no migration.
- **The gap:** The exact refusals.
- **The reach:** The engine id moved, and saved replays from before are
  refused (the user's no-compatibility rule).
- **Verdict:** sound.
- **Confidence:** high.

### Small calls with one obvious answer

- **When:** various.
- **The choice:** The encounter opponent and referee are
  `crates/sim/src/encounter/defender.rs` and `referee.rs`, moved unchanged. The
  benchmark defaults to `city-contact`, and its preset keeps its one-valued
  `variant`/`blue` fields so its fingerprint holds. The app journey buys a
  roster Abrams in a generated skirmish, and the army journey stays on the
  street test map with its digest pin. Live art lost its village names (`ruin`,
  `assets/source/props/`, city set `lab_boxes`). `street-watch` frames tick
  3600. The `unit-roster` scene's model page takes its device through
  `requestGpuDevice()`.
- **Verdict:** sound.
- **Confidence:** high.

## Sloped panels (2026-10-08)

The user's report: on the Dragoon and the T-15 "the side panels seem to not
be adapting to the sloped frame". Plates, tiles and skirts stood upright at a
fixed distance from the centre line while the faces they are bolted to lean.

### One helper places a part on a leaning side face

- **The choice:** `vehicle_parts.on_side(x, z, side, low, high, proud,
  standing, fall)` is the owner. A face is two (y, z) points on the left side,
  normally the corners of the rings the hull or turret lofts through, so the
  face a part sits on and the hull's own surface come from the same numbers
  (each family names them once, e.g. `UPPER_SIDE`). It returns the part's
  location and rotation: lying parts (`bolted_panel`, `armour_tiles`) get
  their back on the face, standing parts (`stowage_box`, jerrycans, slats)
  lean with it facing out, and `fall` turns a lying part within the face so
  its edges follow a falling line (the T-15's band beside the glacis). The
  per-family lean trig in the LAV, BTR, Fennek and VBL went into it.
  Families that don't call it were not re-exported: their receipts still
  name the `vehicle_parts.py` that built them, and the new function changes
  nothing they build.
- **Verdict:** sound. One rule, data per family.
- **Confidence:** high.

### Hull shapes changed inside the same frames

- **The choice:** No frame, mount or tolerance moved. The Stryker's upper
  sides lean 16° (`SIDE_LEAN`, was about 5°): the M1126 rear photo puts the
  roof at about 85% of the chine's width, and 12° did not read at the sheet's
  distance. The Dragoon's lower roof keeps the same lean. The commander's
  hatch moved 0.10 m inboard to stay on the narrower roof. Its armour tiles
  are hung only where the side under the glacis is tall enough for a whole
  tile, so the front tile no longer pokes above the glacis (it did before),
  and the exhaust moved into the gap between tiles and bins.
  The Armata's upper hull is lofted, not a box: skirts up to a shoulder, then
  a band leaning in to the deck's edge, and toward the nose the skirts' tops
  follow the glacis down. The T-14's band is a narrow 45° strip over tall
  skirts, which now carry the parade stripe and number they used to float
  above. The T-15's is deep (40°) and carries its slab modules as one
  continuous band falling with the glacis to the nose. Tall upright modules
  stand on its rear half, as the three-quarter-rear photo shows, and its rear
  stowage boxes moved inboard onto the narrower deck.
- **The gap:** The shoulder is a ring, so it is level along the hull. Where
  the T-15's band runs down to the nose, the slabs follow the glacis and the
  hull behind them stays at the shoulder. That is hidden from the sheet's
  views and is acceptable art.
- **Verdict:** sound.
- **Confidence:** medium. The leans are read from photos, not drawings.

### The audit: what was checked and what changed

- **Changed:** Stryker and its support cards (shared hull), T-14 and T-15,
  LAV, BTR, Fennek and VBL (helper in place of hand trig; the Fennek's
  window and the VBL's door and window leaned outward, against a body leaning
  in), Boxer, VBCI, ZBL-08 and ACV (upright plates on 4-14° sides, up to
  5 cm off at their tops), and the Type 15, M10, CV90120, M1E3 and KF51
  turrets (upright plates 6-11 cm off sides leaning 8-38°; the M1E3's rose
  above its roof, and the KF51's hatch straddled the chine).
- **Checked, unchanged:** every tracked hull (Abrams, Leopard, Challenger 2
  and 3, Leclerc, T-72/80/90, Type 99, Bradley, CV90, Puma, Ajax, BMP, BRM,
  M10, M1E3, KF51, Type 15, howitzers, rocket and air-defence carriers): the
  hull sides are upright prisms and the photos show upright skirts. The
  Centauro and Jaguar hang nothing on their sides. Turrets with upright or
  near-upright sides in model and photos (Leopard, Leclerc, Type 99, T-90,
  Challenger 2 and 3, Boxer RCT30, Bradley), and the T-72/T-80 domes, whose
  modules hang on the dome's near-upright foot. Trucks, cars and aircraft
  have no leaning armoured sides.
- **Left as is:** the CV90's and Ajax's turret add-ons are thick blocks
  across both the upright foot and the leaning wall of their turrets, so no
  single face fits them. They stand at most 5 cm and 2 cm off at the top, and
  the photos (the CV90's are camouflaged) do not settle their shape.
- **Verdict:** sound.
- **Confidence:** medium.

## Follow-up 02

Wreck debris scatters and fades (2026-10-08). The user's decision: "yes,
maybe debris that scattered can just disappear?" (see [the debris question](#may-a-wreck-scatter-debris-wider-than-its-hull)).
Spec: [slice 02](../../unit-models-followup/slices/02-wreck-debris.md).

### Debris is its own state of the wreck, not nodes inside the whole

- **The choice:** A wreck appearance may carry a `debris` state
  (`<appearance>_wreck_debris.glb`), every mesh of it under a `debris_*`
  node (`SCENERY_KINDS.wreck.debris`, `DEBRIS_NODE`). The whole (`default`)
  and the `hull`/`turret` pieces carry none, so the footprint check, which
  measures whole states, leaves debris out by construction. The debris state
  is held instead to its own allowance (`fit.debris`): no point more than
  `reach_m` 8 m across the ground from the hull box, none above `top_m` 1 m,
  and nothing that isn't under a `debris_*` node. Both numbers are tripwires
  picked against the scatter (it lands 0.5-4 m clear, under 0.6 m tall), not
  targets. The bundle keeps its kind's first state first, so a wreck's
  impostor and sheet still show `default` although `debris` sorts before it.
- **The gap:** The spec put `debris_*` nodes in the wreck and had the check
  leave them out. A static state is merged into one mesh per tier, so the
  renderer can't fade a node of it; only a state can be drawn apart.
- **The reach:** A `debris_*` node left in the whole is simply part of the
  whole: held to the footprint and never faded. That is safe (it can't read as
  cover the box lacks), so it is not refused. The disabled cards' committed
  wrecks still carry such in-footprint plates until their lanes re-export.
- **Verdict:** sound.
- **Confidence:** high.

### One helper throws every family's debris, from what the vehicle carries

- **The choice:** `wreckage.scatter(half, mats, seed)` throws pieces
  0.5-4 m clear of the hull box (`THROW_GAP_M`), flat on the ground
  (`LIFT_M` above it), never on each other, one per 3.5 m of the box's
  perimeter within 3-7 (`PIECE_EVERY_M`, `PIECES`). What it throws comes
  from the built scene: a run of track (as wide as the vehicle's own) off a
  tracked hull, a wheel or two (its own radius) off a wheeled one, a blown
  hatch or a door, armour packs, a stowage box, a jerrycan if it carries
  any, torn plate; only torn plate off anything without running gear (an
  airframe). Each piece hangs from its own root-level `debris_*` node, so a
  wreck settled askew on its suspension doesn't lift it off the ground.
  `vehicle_export` calls it for every family after the family's `wreck`
  and before `burn`, seeded by the variant id, so no family places debris
  and the disabled lanes get it by re-exporting. The test tank, jeep and
  truck call it themselves (their scripts don't use `vehicle_export`) and
  export the state with `--piece=debris`. `wreckage.cut_to(state)` is the
  one owner of cutting the whole into `default`, `hull`, `turret` and
  `debris`; `tank.py`'s own piece code went into it.
- **The gap:** Families' own `debris_<k>` plates were litter beside or on
  the hull, within the footprint ("pulled in" when tolerances couldn't
  widen). They are renamed `litter_<k>` and stay as the wreck's own, so the
  approved wrecks don't change; `debris_*` now means thrown. A plate left
  named `debris_*` on a roof would be refused by the debris allowance (too
  tall), which is how a disabled lane learns to rename it.
- **Verdict:** sound.
- **Confidence:** medium. The piece list is a first cut read from what a
  film shows, not from each family's photos.

### The cook-off owns the fade: debris lies, then sinks, as a corpse does

- **The choice:** `effects/cookOff.ts` `debrisSink` is the one owner: no
  debris before the ammunition goes, still through `hold_s` (20 s), then
  sinking, easing in, over `fade_s` (4 s) until its top is under the ground,
  then not drawn (`presentation.effects.cook_off.debris`). `cookOffs.ts`
  `debrisModel` draws the `debris` state where the wreck lies, its top
  sunk 0.15 m past the ground (`DEBRIS_BURIED_M`) so a slope still hides it;
  the battle session keeps each watched cook-off's debris drawn after its
  moving wreck hands over to the static wreck, until it is gone. The wreck
  itself never fades. Sinking is how corpses already leave
  (`corpses.sink_m`), so no translucent pass was added.
- **The gap:** None in the contract; the hold and fade lengths are picks.
- **The reach:** Only a death the side watched shows debris; a wreck found
  later is bare, which is what a viewer arriving later would see after it
  had "disappeared". A page restart or an earlier tick clears it.
- **Verdict:** sound.
- **Confidence:** medium. 20 s is long enough to read the scatter at the
  moment and short enough not to be mistaken for cover in a firefight.

### Merging with the other lanes

- **The choice:** Every roster family's wreck was re-exported (40 wrecks,
  byte-identical twice) with the test units' (also twice). The Stryker
  Dragoon's wreck is among them, and slice 01 rebuilds the Dragoon: after
  both merge, re-export `stryker.py --variant=us_stryker_m1296_dragoon
  --wreck` and rebake. The disabled lanes re-export their wrecks through
  `vehicle_export` and add each card's `debris` state to its catalog entry.
- **Verdict:** sound.
- **Confidence:** high.

## Follow-up 03

Disabled air and helicopter cards at the roster bar (2026-10-08), after the
user overruled [the lower detail](#disabled-cards-were-accepted-at-a-lower-detail-than-roster-units).
Spec: [slice 03](../../unit-models-followup/slices/03-disabled-air.md).

### The old airframes put their cockpits and gear too far forward

- **The choice:** Measuring each side photo (stations as a fraction of the
  stated length, heights against the stated height where the camera allows)
  showed the same fault in most jets: the canopy 0.5-2 m too far forward and
  often too short, the nose gear and mains a metre or more forward, and in
  several (F-35, A-10, Typhoon, Tornado, Gripen, Mirage) a body too shallow
  or sitting too low. Each was rebuilt to its photo, and every canopy now
  fairs into a dorsal spine instead of ending in a wall. Frames (length,
  span, height) didn't move; where a raised body pushed a fin above the
  stated height, the fin was shortened.
- **The gap:** Some photos are three-quarter or in flight (Su-27, Su-34,
  J-10, J-20, Su-25), so their stations are judged, not measured; their
  layouts were kept where nothing contradicted them.
- **The reach:** The disabled frames are art-only, so nothing in the
  simulation moved.
- **Verdict:** sound.
- **Confidence:** medium.

### Markings are paint pressed onto the skin, from the photo's air arm

- **The choice:** `aircraft_parts.insignia` and `lettering` lay a national
  insignia (layers of outlines, `INSIGNIA`) or letters on a plane and press
  each vertex onto the near-tier skin of the parts named by `onto` along a
  ray, standing 6 mm off it, so a decal follows a curved fuselage instead of
  floating as a flat card. Each card wears the markings of the air arm in its
  side photo (RAF Typhoon, Luftwaffe Tornado, Flygvapnet Gripen, Armée de
  l'air Rafale and Mirage, US low-visibility greys, VKS red stars, PLA stars
  and bars) with that photo's codes and numbers. The crash burns them off
  (`marking_*` go with the canopy glass). Colours are one marking paint per
  `MARK` key, role `marking`.
- **The gap:** The Swedish crowns are three gold spots and the A-10's shark
  mouth is left off: neither shape is a star-shaped outline the layers can
  draw. Cyrillic titles use Blender's built-in font.
- **Verdict:** sound.
- **Confidence:** medium.

### Shared detail lives in aircraft_parts, so every airframe gains it

- **The choice:** Framed canopies (windscreen arch and bows in the
  airframe's paint, sill frames), control-surface hinge lines (`hinges`),
  nav lights, gear drag braces, torque links, hub caps and bolts, taxi
  lights, pylon sway braces, missiles on launch rails with canards and live
  bands, bomb fuzes, and on every main rotor (over 2.5 m) a swashplate,
  pitch links and blade cuffs. Blade tips can take a paint (`tips`); no card
  uses it, as no photo settled it.
- **Verdict:** sound.
- **Confidence:** high.

### Helicopters got the shared detail and markings, not a re-measure

- **The choice:** The Apache was measured against its side photo and rebuilt
  (the tandem glass standing out of the body, half a metre aft; the engines
  higher; the mains under the gunner). The other fifteen rotorcraft cards took
  the shared rotor, gear and store detail and their air arm's markings, and
  were checked by eye against their photos, but their cabin cross-sections
  were not re-measured station by station.
- **The gap:** The slice asks for cabin depth and cross-sections from the
  references for every family; this pass ran out of room for fifteen of
  them.
- **The reach:** The "slim cabins" the closeout named may remain on the
  H-1s, Little Birds, Chinooks, Tigers, NH90, Merlin, Wildcat, UH-60, Z-20,
  Mi-8, Mi-35, Mi-28, Ka-52 and Z-10.
- **Verdict:** needs-user. Provisional: as shipped. To finish: measure each
  side photo as the jets were and rebuild the cabins.
- **Confidence:** medium.

### The NH90 stays 3.6 m wide

- **The choice:** A second source, [Army Technology](https://www.army-technology.com/projects/nh90-tactical-transport-helicopter/),
  gives the same 4.62 m width as Vertipedia beside a 19.56 m length "rotors
  turning" and a 5.2 m height that only the turning tail rotor reaches. The
  4.62 m is the overall figure, not the fixed airframe, so the card keeps
  3.6 m over its sponsons.
- **Verdict:** sound. It closes [the NH90's width](#the-nh90s-width) unless a
  drawing says otherwise.
- **Confidence:** medium.

### Disabled wrecks throw debris but have no catalog entry to name it

- **The choice:** Every air wreck was re-exported through `vehicle_export`
  after slice 02 merged, so each writes its `_wreck_debris.glb`, and the
  crash's own plates beside the fuselage are `litter_*`. Disabled cards have
  no catalog entry, so there is no `debris` state to add; a lone file can't
  be held to `fit.debris` or count its tiers with its whole, so those two
  checks were read past when validating the debris files.
- **Verdict:** sound.
- **Confidence:** medium.

## Follow-up 01

The Dragoon's sides, the M10's turret and the Abrams' front and skirts
(2026-10-08). Spec: [slice 01](../../unit-models-followup/slices/01-dragoon-m10.md).
No frame, mount or tolerance moved, and every export repeats byte for byte.

### The Stryker's chine is a low sharp knuckle under a tall leaning side

- **The choice:** The hull is three rings (`BELLY`, `KNUCKLE`, roof). The
  lower hull flares from the belly (0.52 m, 0.84 m half width) out to a sharp
  knuckle at 1.16 m, just above the 1.12 m tyre tops and flush with their
  outer faces (1.34 m). The 0.12 m upright chine band is gone. The upper side
  leans in 14 degrees (`SIDE_LEAN`, was 16) from the knuckle to the roof, so
  the leaning face is 1.14 m tall on the M1126 and 0.79 m on the Dragoon (it
  was 0.86 m and 0.51 m), and from the front and rear the knuckle is the
  widest line of the hull, as in the photos. The lean dropped two degrees so
  that the roof, now further from the knuckle, keeps room for the M1134's
  launcher base and the commander's hatch (moved 7 cm inboard). The nose
  comes to a point at the knuckle, and the upper glacis keeps one slope
  (`GLACIS`, 25 degrees) on either roof. The ramp, mudflaps, tow points,
  antennas and the M1129's bay doors are placed from the same numbers.
- **Tiles and plates:** Armour tiles lie on the leaning side through
  `on_side`, in as many rows as the side is tall for (two on the 2.30 m roof,
  one on the Dragoon's). Under the glacis' chamfered edge (`side_top`, the
  rings' own corner) a tile is cut down to what fits, and it is left off below
  0.24 m. That gives the stepped tiles the M1126 photo shows. The nose's
  bolted plates, two rows of two, lie on the lower nose face. `on_side` only
  knows side faces, so they take their position and tilt from the same ring
  numbers (`nose_x`).
- **The gap:** The knuckle sits a little lower than the photos put it (they
  show about 1.25-1.3 m). The slice asked for a lower and stronger knuckle so
  that the lean reads at sheet distance, and the Dragoon's roof is pinned low
  by its gun axis. The rear half above the bins is a bare leaning face, where
  the real vehicle carries racks and bags.
- **Verdict:** sound.
- **Confidence:** medium. Measured from photos, not drawings.

### The M10's turret is tall and blunt, not a small Abrams

- **The choice:** The turret's plan is a broad front of two flat cheeks
  either side of the mantlet's slot, with chamfered outer corners and upright
  slab sides 1.48 m out. The old plan was a wedge whose cheeks swept back 55
  degrees, on sides leaning in 12 cm. In profile the cheeks come to a prow
  just above the gun axis (`PROW`). Under it the front falls back to the
  ring, and over it a long chamfer runs back to a flat roof. The roof is 6 cm
  higher (`ROOF` 0.78 above the pivot, 2.56 m: the side photo gives about
  2.53 m) and the sight dome's collar 2 cm shorter, so the dome's top stays at
  the stated frame height (built 2.94 m against 2.90 m). The smoke clusters
  sit on the cheeks firing forward, the cheek plates lie on the flank through
  `on_side`, and the gunner's sight moved behind the chamfer. The mounts are
  unchanged.
- **Verdict:** sound.
- **Confidence:** medium. Only side and front-quarter photos, no top view.

### The Abrams' cheeks are broad and blunt, and its skirts sit flush

- **The choice:** The turret's cheeks (`CHEEK`) now run from the gun's slot
  at x 1.95 back 15 degrees to a cut corner. Before, they ran from a point
  at 2.18 back 35 degrees, plus 0.30 m more at the roof, which made an
  arrowhead in plan. The roof edge sits 0.22 m back of the face
  (`CHEEK_CHAMFER`), the short top chamfer the photos show. The cheek's side
  plate lies on the turret's leaning flank through `on_side`; it had stood
  upright 5 cm off it. The skirts' outer faces did not move (1.80 m). The
  hull's upper half widened from 3.40 m to 3.46 m (`SPONSON`), so the
  sponson side is flush with the skirts' inner face and the thick front
  panels sink into it. Before, there was a 3 cm gap and the deck stopped
  short of the skirts. The Abrams' hull sides and skirts are upright, so the
  skirts need no `on_side`.
- **The reach:** The M1E3 builds on `abrams.hull_body`, so its hull widened
  too and it was re-exported. Its own skirts already overlapped the wider
  hull, and its turret is its own. The menu's tank keeps the test hull's
  physics, and `menu_reel::` passes unchanged.
- **Verdict:** sound.
- **Confidence:** medium.

## Follow-up 04: disabled ground cards at the roster bar (2026-10-08)

The user overruled the lower detail of disabled cards. These are the calls
made while raising the ground, support, drone and infantry cards, family by
family. No roster frame, mount or tolerance moved.

### A disabled card stands on the roster vehicle it is built on

- **The choice:** Where a card's chassis is a roster vehicle, it uses that
  vehicle's script instead of a look-alike. The BM-21 stands on the roster
  Ural's `chassis` and `cab` (the Ural-375D and 4320 share the cab and frame),
  so `ural.py` was split into `chassis`, `cab` and `body` and its export put
  under `__main__`; the Ural's GLB re-exported byte-identical. The truck
  chassis's `bonnet` cab, which only the BM-21 used, is gone. The armoured
  cab-over (`truck_chassis.armoured_cab`, under the HIMARS, CAESAR and
  Pantsir) was rebuilt to the bar: waist-high upright sides with the upper
  sides leaning in, framed windscreens with wipers and a mullion, a bolted
  door with its small armoured window placed with `on_side`, hinges, handle,
  grab rail and two steps, the bolted front plate, grille, guarded lights and
  shackles, roof hatch and marker lamps, and the dash and seats behind the
  glass. It now spans the card's width, as the photos show, not its wheels'.
- **Verdict:** sound. One owner per cab.
- **Confidence:** medium.

### The BM-21's width is the Ural-375D's

- **The choice:** The card stated 2.40 m wide, the figure usually quoted for
  the BM-21, but that is narrower than the truck's own axle hubs. The card now
  states the Ural-375D's published 2.69 m; length and height stay the BM-21's.
  The library's `gaps` records it.
- **Verdict:** sound. A disabled card's frame is its script's statement from
  cited sources, and this one was wrong.
- **Confidence:** medium.

### Rocket launchers travel as the photos show them

- **The choice:** The M270's and HIMARS's pods stow with their capped faces
  to the rear, inside a ribbed launcher-loader box open at the back
  (`launcher_box`, `REAR_LIP`), with the loader boom on top and a cable run
  down the side. They had faced forward, inside a closed box. The BM-21's
  forty tubes also travel muzzles to the rear, rising slightly toward them, on
  a cradle with sector gears, as all three of its photos show. The Tornado-S
  keeps its muzzles forward (it elevates about a rear pivot) and gained rear
  stabiliser jacks, stowage, a ladder, tube rings and the firing cable.
- **Verdict:** sound.
- **Confidence:** medium.
### Disabled wrecks throw through `wreckage.scatter`, and lose their own throws

- **The choice:** After slice 02 merged, these families' wrecks were
  re-exported through `vehicle_export`, so each writes its
  `_wreck_debris.glb` state from the shared `scatter`. The families' own
  pieces beside the hull (torn plates, fallen rounds, the air defence
  family's fallen canister) were deleted rather than renamed `litter_`,
  because they lay outside the hull, which is exactly what `scatter` now
  throws. A disabled card has no catalog wreck entry, so there is no
  `debris` state to list; the debris GLB is written for when the card
  becomes playable. `asset validate` on a debris GLB alone refuses it (a
  state of several root `debris_*` nodes is only judged inside its catalog
  entry), so those files are judged only by `scatter`'s own tests until then.
- **Verdict:** sound.
- **Confidence:** medium.

### Air defence and the CAESAR: what changed

- **The choice:** The tracked air defence hulls (`tracked_hull`, Gepard and
  Buk) carry a bolted glacis plate, tow cables, hull bins, tail lights and
  exhausts. The Gepard gained its gun yokes, ammunition feeds, muzzle
  velocity radars, the tracking radar's feed and yoke, the search radar's
  face, a second hatch, periscopes, rangefinder and rear basket. The Buk's
  cab has shutters raised over its windscreens, hatches, periscopes and doors;
  its radar a flat face and ribs, its canisters ribs. The Pantsir-SM stands on
  a new `cabover` cab, the armoured cab's shape unarmoured, because its
  photos show the KamAZ-6560's ordinary cab with a tall windscreen, not an
  armoured one; it carries six missiles a side in two columns of three (it
  had three), drawn narrower so the module stays within 6% of the card's
  2.55 m. The Skyranger's turret has radar panel frames, a mantlet, sleeve,
  a four-cell launcher on its arm, hatch, smoke dischargers. NASAMS's
  trailer has bar-tread tyres, one mudguard per side, an A-frame drawbar,
  a jockey wheel (under a `wheel_` node, since it is rubber), ribbed
  canisters and a cable reel. The CAESAR's crew cab has two doors a side
  (`chassis(doors=2)`), its gun a trunnioned cradle with equilibrators and
  elevating sectors, a breech ring, collar and clamp on the rest, walkways,
  rear jacks and the spade's rams.
- **The gap:** Judged at the sheet's distance against one or two photos
  each. The Gepard's search radar is drawn folded (its photos show it up)
  and its turret is lower than the photos'; the Pantsir's module is still a
  box where the photos show a busier one.
- **Verdict:** sound for the silhouette and detail; the two gaps are taste.
- **Confidence:** medium.

### Drones: what the closer look adds

- **The choice:** Every propeller is a hub and tapering two-piece blades
  (and a spinner on a tractor or pusher); an FPV's props are three-bladed,
  shortened to 0.81 of a two-blade one so their tips stay where the frame's
  stated length put them. Quadcopters carry the battery seam and latch, GPS
  puck, obstacle cameras, a status light, arm hinges and arm-tip lights, motor
  bells and skids; the FPV its video antenna and receiver whiskers and the
  PG-7's band and tail. Loitering munitions have a seeker bezel, warhead seam,
  arming plug, wing hinge blocks, datalink antennas and ailerons; the Orlan-10
  a wing pylon, flaps, tip lights, cylinder head and exhaust, parachute lid,
  pitot and launch lug. The FPV's PG-7 fuse was left off: it would reach past
  the card's stated 0.45 m.
- **The gap:** `asset validate` on a drone GLB alone reports
  `structure.scenery_kind` (no wheel, track or mount nodes, so the loose
  validator takes it for scenery). That was so before this pass; the drones
  only meet the strict rules when their cards get unit types.
- **Verdict:** sound.
- **Confidence:** medium.

### The M1E3's remote station is drawn at a CROWS's size

- **The choice:** The station on its pedestal now carries what a CROWS
  carries: the flange and its bolts, a cable harness down the column, the
  traverse and elevation drives, a three-window sensor block (thermal, day,
  rangefinder) under its hood, the ammunition can with lid and latches, the
  feed tray and charger. The block and can are drawn larger than the M2
  alone suggests, because the one photo shows the station as a big block
  beside the gun. The turret's cheeks carry sensor windows under hoods, the
  gun shield its bolt rows, the rear corners cameras, and the independent
  sight box a second window. The mounts did not move: the station's pivot is
  still the photo's, and the frame check leaves the station out as before.
- **The gap:** One photo, a left quarter. The station's right side and the
  roof between the hatches are unseen and drawn from the CROWS.
- **Verdict:** sound.
- **Confidence:** medium.

### Families not yet raised still throw through `scatter`

- **The choice:** The SP howitzers, BRM-3K, T-14 and T-15, Jaguar,
  Challenger 3, Type 15, Centauro II and CV90120 were not raised to the bar
  in this pass. Their wrecks were re-exported through `vehicle_export` so
  each has its `debris` state, and their three ground plates beside the hull
  went, as above. Their live GLBs re-exported byte-identical to the
  committed ones. The M-SHORAD and M1129 (`stryker_support.py`) were left
  alone: they stand on the Stryker hull slice 01 is rebuilding, so their
  wrecks are re-exported after that lands. A wreck's thrown turret piece
  alone fails `asset validate` (`basis.ground`, `structure.scenery_kind`):
  it is judged only as a state of its catalog wreck, as the M1E3's was before.
- **Verdict:** sound.
- **Confidence:** high.

### The SP howitzers share one set of turret fittings

- **The choice:** The four turrets (M109A7, PzH 2000, 2S19M2, PLZ-05) differ
  in shape and gun, so their shared fittings went into one
  `turret_fittings`: the mantlet's bolt rows, recoil cylinders over and under
  the barrel's root, roof ventilator domes and an access plate, a side
  periscope and an ammunition loading hatch on each flank, roof grab rails,
  the rear door's hinges and handle, and rungs beside it. Hulls were already
  at the bar (skirts, cables, travel lock, lights) and were left alone.
- **The gap:** Where a type differs (the 2S19's ventilators sit further
  forward), the shared positions are an approximation.
- **Verdict:** sound.
- **Confidence:** medium.

### Turret roofs share one set of fittings

- **The choice:** `vehicle_parts.roof_fittings` is the owner of what any
  turret roof carries: a lifting eye at each corner of the crown ring, a grab
  rail along each side's rear half, and vision blocks where a family names
  them. The BRM-3K, Jaguar, Centauro II, CV90120, Type 15 and Challenger 3
  call it with their crown ring (and the T-14 and T-15, which also gained
  tail lights), so the fittings follow each turret's own
  plan. Their hulls gained what was missing at the rear: tail lights in
  guards (BRM-3K, Type 15, Challenger 3) and tow hooks (BRM-3K, Jaguar).
  Each family already carried its own sights, hatches, smoke and stowage
  from slices 18 and 20, and sloped-panel fixes from the sloped-panel pass.
- **The gap:** The eyes and rails sit at proportions of the crown, not at
  places measured off each photo.
- **Verdict:** sound.
- **Confidence:** medium.

### The five disabled infantry teams: their launchers at the bar

- **The choice:** The soldiers already wore the roster infantry's uniforms,
  carriers and rigs (slice 17), so only `shoulder_tube_equipment` changed,
  and only these five cards use it. Every tube has its sling swivels and the
  bands where its halves join. The Javelin's command launch unit has its two
  handles, eyecup, day and thermal windows and switches, and the battery
  coolant unit hangs under the tube. The Akeron's sight has a hood, a thermal
  window, an eyecup and a shoulder rest. The MANPADS have a battery coolant
  unit, front and rear sight frames and a sight arm; the Stinger has its IFF
  grid's ribs, and the Igla and FN-6 the interrogator box on the gripstock.
  The kits re-exported byte-identical twice. The Javelin's carried kit did
  not change.
- **The gap:** `asset sheet` can't draw a kit that isn't in the catalog, so
  the kits were judged on the exporter's own Blender preview, not beside the
  references. `asset validate` passes them only when given the launcher clips
  and the skeleton's 90° basis.
- **Verdict:** sound.
- **Confidence:** medium.
## Names and weapon icons (2026-10-08)

The unit info-card workbench showed catalog identifiers on cards: unit
names composed from family and variant ("BMP IFV Family BMP-3", "Tigr
Tigr-M"), mount ids as labels (`MAIN_GUN`, `HEAVY_SNIPER_1`), terse or long
weapon names ("AP", "Spike (direct guidance)") and blank icon slots. The
user's request: every unit and weapon gets a short human-readable label from
the catalog, display names separate from ids, every weapon a generated icon,
and missing names or icons fail validation; no UI truncation, no automatic
underscore replacement, no generic fallback icon.

### A label is concise by one testable rule, checked where the catalog loads

- **The choice:** `contract::labels::check_label` owns "concise": at most 18
  characters, single-spaced, opening with a capital or a digit, only letters,
  digits, spaces and `- / . ×`, and no word written twice (split at spaces,
  hyphens and slashes, so a family repeated before its variant is refused).
  `check_icon` holds an icon to a lowercase id. Catalog resolution applies
  them to a unit's `name`, its `roster.family_name`, each mount's `name`, an
  active protection's `name`/`icon` and a planned unit's weapons; weapon-row
  resolution (`resolve_weapons`) to each row's `name`/`icon`. A refusal names
  the entry and the field. Test and menu units are held to it too.
- **The gap:** The user named examples, not a rule.
- **The reach:** 18 is a tripwire picked against the longest label a card
  needs ("M1A2 SEP v3 Trophy", "F-15E Strike Eagle"), not a target. The rule
  cannot judge terseness ("AP" passes); the authored labels carry that.
  `roster.variant` (the picker's variant buttons) is not held to it: several
  read as short descriptions ("Close-quarters infantry"). Soldier kinds and
  parts are not shown on a card and are not checked.
- **Verdict:** sound.
- **Confidence:** medium. The repeated-word clause is the one most likely to
  refuse a real name some day; rename it rather than loosen the rule.

### A mount's id moved to `id`; `name` is its label

- **The choice:** `MountDefinition` gains `id` (what `sensors.on`, a mount's
  `on`, a model's rig declarations and variant merges name) and keeps `name`
  as the player label. Lists of objects now merge by `id`. The mechanics
  editor addresses mounts by id. Old documents (and replays carrying them)
  are refused, with no compatibility path; the user accepted that.
- **The gap:** The spec has no mount label; mount names were slugs used both
  ways.
- **The reach:** A variant that changes a mount's weapons inherits its
  parent's label; every shipped label matches its weapons, but nothing
  checks that a label agrees with what it fires.
- **Verdict:** sound.
- **Confidence:** high.

### Active protection and planned weapons carry a label and an icon

- **The choice:** `ActiveProtection` gains `name`/`icon` (the Trophy part says
  "Trophy", `trophy`), replacing the panel's hard-coded `TROPHY` row, whose
  icon file was never generated. A planned unit's `planned.weapons` become
  `{ name, icon }` objects, and each card publishes them as
  `planned_weapons` (empty for a unit type, whose weapons are its mounts), so
  a disabled card can show its weapons like any other. `ActiveProtection` is
  no longer `Copy`; the simulation reads it by reference, and a supply need
  carries the three numbers it uses.
- **Verdict:** sound.
- **Confidence:** high.

### Eight new generated weapon icons

- **The choice:** `WEAPON_ICONS` draws `sniper` (marksman and heavy sniper
  rows), `autocannon`, `rpg` (RPG-7/29), and for planned weapons
  `aa_missile`, `bomb`, `howitzer`, `rockets` and `mortar`; `trophy` was
  already drawn. `iconFiles` now writes every weapon row's, every active
  protection's and every planned weapon's icon, so `asset check` and the icon
  test refuse any of them without a drawing. Drone warheads reuse
  `ap_shell`/`he_shell`, aircraft and helicopter guns `autocannon`, tank
  guns on planned cards `ap_shell`.
- **Verdict:** sound.
- **Confidence:** medium. Judged on a contact sheet at 64 and 16 px; no
  unprimed critique was run (no helper agents in this lane).

### Moved records

- **The choice:** The frozen generation corpus's recorded catalog
  (`fixtures/parity/map-layout/physical-rules.json`) was rewritten to the new
  contract (mount ids and labels, test unit labels), since the old form no
  longer loads. Its generated layouts are byte-identical (checked against the
  base build); only each result's `identity.config_hash`, which hashes the
  physical rules including that catalog, moved, so `paired-records.json` was
  re-blessed (`BLESS_PARITY=1` on `layout_cli`). `fixtures/catalog.json` was
  re-blessed (`BLESS_CATALOG=1`).
- **Verdict:** sound.
- **Confidence:** high.

## Not fixed here: failures that also happen on main

These checks fail on main as well as on this branch. This spec did not cause
them and did not fix them.

- **Web test `battleFailure`** ("a rejected replay shows its refusal"). A
  replay the simulation refuses shows its message in the battle's top bar,
  not on the loading screen's Aborted page the test expects.
- **Scene `foundation`** times out.
- **Scene `consequences`** fails its soldier-death check.
- **Scene `panels`** fails all six of its checks.
- The `unit-roster` scene also reports one 404 page error, seen since the
  session catalog slice. Its checks pass.
