# Armory: choices

The decisions the implementing agents made where the plan was silent, as the
work finally shipped (`6858248a9`). Each entry was re-audited against that
code. Provisional entries are written at their end state, entries a later pass
replaced are gone, and duplicates are merged. Decisions the user made, and the
plan's own decisions, live in [`decisions.md`](decisions.md) and the
[unknowns map](unknowns-map.html). This file holds only the choices made on
the user's behalf.

Each entry gives the choice as a headline, then the scenario: what happens,
what the code does, and what the alternative would have done. After that come
**When** (the slice it landed in), **Gap** (what the plan left open),
**Reach** (what later work it binds), **Verdict** and **Confidence** (how sure
the audit is that the user would have made the same call). Groups are ordered
by verdict, and each group runs least confident first.

Counts: 6 user-owned, 1 unsound, 72 sound (13 low, 24 medium, 35 high), and
24 trivial calls folded into one line at the end.

## Review these first

1. **Art quality** (user-owned): six of the seven military props ship below
   the bar the repo sets for models.
2. **Prop boxes have two homes** (unsound): depots read the presets' one
   table of boxes, but homes and field cover still carry their own copies of
   the same boxes.
3. **The hangar ruin lets infantry walk through it** (sound, medium): the
   earlier ledger said it blocked soldiers. The shipped row does not.

---

## User-owned: open calls

These calls are the user's to make: taste, product direction, or a contract
change. Each one ships with a provisional answer you can reverse.

### Art quality: six of the seven military props score 4–6 out of 10

When a player opens the Armory on a helicopter, it stands on the apron in
front of three hangars. Each hangar is the same model, so all three carry the
same stencilled "3" on the door. At rest, the Apache's aft rotor blade crosses
its tail fin. The model's rest angle puts it there. An unprimed critique
scored each prop against its real photos, out of 10, on silhouette, surface,
fittings, materials and cohesion. The repo's model bar is none under 6 and an
average of 7. Only the hangar meets it, at 8/7/6/6/7. The ruin scores 5/6/5/5/5
and reads as "a rug over a bed" at a distance. The revetment scores 7/5/5/5/6
and can read as a shipping container. The HESCO scores 7/6/5/6/6 and is pale.
The tent scores 6/5/6/5/6. The flagpole scores 5/5/4/5/5: its 7 cm pole thins
to nothing at battle distance and leaves the flag hanging in the air. The net
scores 4/6/3/5/4 and still reads as a mound. One hangar's roof also reads teal
under the yard's light. "Lived-in" would also want people and parked
vehicles, which no prop provides.

- **When:** slices 15 and 16.
- **Gap:** the plan set the bar ("looks like a shipped game's armory") but
  gave the art no time box. The art ran under tight disk.
- **Reach:** every depot on every generated map, and the Armory's yard. A
  better prop is a re-export from its Blender script, not code: the bodies,
  boxes and digests stay put.
- **Verdict:** needs-user. **Provisional answer:** ship the current art and
  improve it as a later art pass, one prop at a time, starting with the net
  and the flagpole. The Apache's rest angle belongs to that model's next
  pass. To give each hangar its own number, the yard would need a per-instance
  decal, which is new art machinery.
- **Confidence:** low.

### Depot density: `open_country.depots.per_road_km` is 0.3

A player starts a Medium skirmish. Somewhere on the map, beside a country
road, stands one military depot: a hangar with a revetment and supplies, or a
camp of tents. The density is one number in the map presets, depots per
kilometre of country road. The lobby's maps are short-roaded (1.8–3.6 km
across), and at 0.08 only 1 map in 36 held a depot. At 0.3, Small maps hold 0
or 1, Medium 1, Large 1 or 2, and XL 0 to 2. The larger maps that only
developer tools and tests use hold 1 to 9, kept 1.5 km apart.

- **When:** slice 17.
- **Gap:** the plan said depots are "rare" and left the number open.
- **Reach:** every generated map's identity. Changing the number moves
  generated maps and the frozen map records, a named move.
- **Verdict:** needs-user. **Provisional answer:** 0.3, about one depot per
  battle. "Too many" or "too few" is that one value.
- **Confidence:** low.

### The lobby's map picture draws no props, so players never see depots there

When a player sets up a skirmish, the lobby shows a picture of the map:
buildings, roads, woods and water. It draws no props, so a depot (made
entirely of props) is missing there, as field cover already is. The
developer's plan picture does show depots: each one in fuchsia, ringed on a
whole-map view, with a legend entry. Drawing big bodies in the lobby would be
a player-facing look of its own. It would also change every lobby picture
that holds a court or a garage row.

- **When:** slice 17.
- **Gap:** the plan never asked whether players should find depots before a
  battle.
- **Reach:** the lobby picture's look, and its baselines.
- **Verdict:** needs-user. **Provisional answer:** no props in the lobby
  picture. Players find depots in battle.
- **Confidence:** medium.

### The battle-wide rotor blur and the squad's combat-ready idle stance

Two of the battle's own looks show through the Armory. A helicopter in flight
draws the battle's rotor blur, and a fresh eye reads it as "frozen blades
over a glass plate", with the tail rotor as wedge sectors. In the Armory a
parked helicopter's rotors are stopped (the user's call), so the blur now
shows only in battle. An infantry squad idles crouched with weapons up, which
reads as combat-ready rather than at rest in a camp. The Armory reuses the
battle's idle clip and its watchers. Both looks are owned by
`fixtures/presentation.json` (`rotor_blur`, `rotor.tip_mps`, `pose.rest`), so
changing either changes every battle.

- **When:** slice 04, which raised both.
- **Gap:** the plan reused the battle's driver so the Armory could never
  drift from the battle, but it never asked whether the battle's looks were
  the ones wanted.
- **Reach:** every battle frame. A change here moves battle screenshots, not
  just the Armory.
- **Verdict:** needs-user. **Provisional answer:** both unchanged, one owner,
  the battle's data.
- **Confidence:** medium.

### A jet's sheet reads "Sight · ground 1 m"

The air-war profile makes a jet blind to the ground. The catalog contract
requires every unit's `sensors.ground_m` to be positive (the sensor check in
`contract/src/catalog.rs`), so the profile writes 1 m. The Armory sheet
prints the catalog's true number, so a player sees "Sight · ground 1 m" on
every jet. Allowing 0 would mean a changed contract and catalog check, plus a
sheet that words 0 as "none".

- **When:** slice 11 found it; the commit `8b58c5a6d` recorded why.
- **Gap:** the sheet shows catalog figures as they are, and the plan never
  met a figure that stands in for "none".
- **Reach:** the catalog contract, which every roster row and the mechanics
  editor share.
- **Verdict:** needs-user. **Provisional answer:** the sheet shows the true
  1 m until the user allows 0.
- **Confidence:** medium.

### Sheet bars are magnitudes, so a slow reload has the longest bar

On the sheet, each figure has a bar: its value over the largest value of that
figure among units of the same body (the plan's U9). That rule applies to
times too. A 5 s reload draws a longer bar than a 1 s one, even though
shorter is better. The number beside the bar says what it means. The
alternative flips time bars so a full bar always means "better". That reads
as a score, but then a 5 s bar is shorter than a 1 s one. A family's tile in
the list shows its cheapest variant's price.

- **When:** slice 08. `decisions.md` left it to the user at that checkpoint.
- **Gap:** U9 fixed the scale per figure but not the direction.
- **Reach:** the sheet model (`unitSheet.ts`), the one reader.
- **Verdict:** needs-user. **Provisional answer:** magnitudes everywhere.
  Flipping times would be a direction flag per stat in `SHEET_STATS`.
- **Confidence:** medium.

---

## Unsound

### A prop kind's box has two homes in the presets

A depot names its pieces by kind only (`hangar`, `pallet_stack`, …). Their
boxes come from `street_props.bodies`, the presets' one table of placed
boxes. But the homes' `bodies` and the open country's `field_cover` rows
still carry their own `half_extents_m`, written by hand. One of those copies
covers a kind the table already sizes: `pallet_stack`, [0.6, 0.5, 0.6] in all
three places. Retune a pallet stack in the table and a home's or the field's
stack keeps the old box, so the same art stands on two different simulation
boxes.

- **When:** slice 17. It left the older copies alone because they predate it.
- **Gap:** the plan named `street_props.bodies` for depots only.
- **Reach:** every generated map. Folding the copies is a named move only if
  a copy differs from the table. Today the shared kinds match.
- **Verdict:** unsound. **Corrected decision:** a placed prop's box has one
  home, `street_props.bodies`. Homes and field cover name kinds and read
  extents from it, as depot pieces do, and admission refuses a kind the table
  lacks.
- **Confidence:** medium.

---

## Sound

### Low confidence: data and taste the user may want to retune

**The military yard's composition.** A player opens the Armory on a tank. It
stands on a hardstand of 3 × 3 concrete slabs, in a bay between two steel-bin
revetments that opens toward the camera. A netted supply stack and crates
stand at the back of the bay, so a tan hull stands against dark green and
never against a pale wall. A helicopter stands on a 7 × 6 slab apron with
yellow taxi lines, a line of three hangars behind it, a site cabin and
jersey barriers. A squad stands on gravel before three tents, with the
flagpole at the camp's centre, sandbags to one side and HESCO to the other. A
gravel lane joins the hardstand to the camp, and a HESCO perimeter runs along
the north and west, the sides every opening camera faces. The heras fence,
dragon's teeth and hangar ruin left the yard: the first two read as a
building site, and the ruin read as a derelict airfield. No prop stands in a
spot's clear disc. A scratch generator laid it out in each spot's own frame,
and `fixtures/armory/yard.json` stores the world coordinates.
- **When:** 16. **Gap:** the plan said "a lived-in military yard". **Reach:**
  data only; the human can move any prop.
- **Verdict:** sound, judged against the references and six unprimed
  critiques. **Confidence:** low.

**The hills beyond the yard.** From every opening camera, past the treelines,
the land rolls: hills 20–40 m high and 280–420 m across, 600–1400 m out,
mostly to the north-west, and a ridge line 160–270 m high about 3 km out. The
first ridge, 95–160 m high, hid behind the nearer hills and was raised. The
relief is sampled every 32 m, about 130,000 vertices (5 MB) per stage. Finer
cells would smooth the slight kinks in hilltop silhouettes, at a measured
cost of 366,000 vertices for 24 m cells.
- **When:** relief pass. **Gap:** the user asked for "more exciting terrain".
  **Reach:** `yard.json` `ground.relief`, data only.
- **Verdict:** sound. **Confidence:** low.

**The Armory camera's feel.** A player drags on the stage: a viewport height
of drag turns 2.5 rad, as the battle's middle-drag does. The wheel zooms
0.0012 per pixel. Pitch runs from −0.15 rad (just under the horizon) to
1.1 rad. A critique read the first ceiling, 1.25, as a flat map view.
Framing opens at 0.85 of the centre cell, zooms in to 1.0 (the unit's box
fills the cell, nothing clipped) and out to 0.4. The near plane is 0.2 m and
the clearance margin 0.3 m. There is no inertia, because the clearance spring
already eases the eye. All of it is `camera_armory` in
`fixtures/presentation.json`.
- **When:** 12. **Gap:** the plan named orbit and zoom, not their numbers.
  **Reach:** data; the user tunes it without code.
- **Verdict:** sound. **Confidence:** low.

**The sheet's words.** The sheet writes capabilities as short phrases
("Carries 8 men", "Deploys in 10 s", "Supply 500", "Trophy: 4 charges",
"Flares ×6", "Radar", "Finds radars within N m", "Afterburner"). The catalog's
`concealment` multiplies how far a unit is seen (infantry 0.5, a helicopter
2.8), so the sheet labels it "Visibility ×n". "Concealment 2.8" would read
backwards. A typed card that isn't sold reads "Not yet on sale". A planned
card reads "Planned", with "Not yet in battle: these are its planned
weapons." The card's `disabled_reason` is developer text and is never shown.
When the stage fails, it says "This unit can't be shown. Its sheet is still
here. Pick another unit, or reload to try again." The raw error sits under a
"Details" disclosure.
- **When:** 08, 11, closeout. **Gap:** the plan gave no copy. **Reach:**
  `unitSheet.ts` and `ArmoryStage.tsx`, text only.
- **Verdict:** sound. **Confidence:** low.

**Each military prop is a particular real type.** The hangar is an olive arch
(Quonset) hangar, and its fine 75 mm corrugation lives in the texture: two
critiques read geometric ribs as "a ribbed barrel". The ruin keeps the vault's
feet on cracked curbs and bent rib stubs, so it reads as that hangar brought
down. The revetment is a galvanised steel bin filled with earth, not a bare
earth berm: bare earth can't stand 3.2 m tall on a 3 m foot, and every
reference is a bin. The row's mechanics (immovable, no `hp`) still describe
it. HESCO is one six-cell module, the battle's repeat unit, and two modules
meet 1 cm apart, never in one plane. The tent is a ridge-roofed wall tent
with its door tied open. The reference camp's tents are square with pyramid
roofs, but a ridge tent is as military and fits the 6 × 5 m body. The net is
woodland camouflage lifted on poles so the stacks show under its edges. It
never touches the stacks, because the art can't know whether two or three
stand under it. All props are generic, with no nation, unit or brand.
- **When:** 15. **Gap:** the plan named the prop kinds, not their real-world
  type. **Reach:** art only; the bodies stay put.
- **Verdict:** sound. **Confidence:** low.

**No cloth motion.** The flag and the tents stand still. The flag is caught
mid-ripple. A static scenery bundle carries no motion, and waving cloth would
be a new renderer feature for every battle.
- **When:** 15. **Gap:** unasked. **Reach:** a renderer feature if wanted.
- **Verdict:** sound. **Confidence:** low.

**A depot's groups and yards.** A depot is one of three groups by weight: a
hangar, a revetment and supplies (2); two tents, a flagpole and supplies (2);
or HESCO, a tent, a revetment and supplies (1). The hangar group carries
extra weight because its big yard fails to fit more often. On 39 skirmish
depots the three stood 10, 16 and 13 times. A yard is 3 m deep front and
back and 2 m at the sides, and yards stand at most 0.5 m apart so their slabs
join into one pad. 40% of depots stand at the end of a 90–150 m lane.
- **When:** 17, delegated. **Gap:** the plan named depots, not their mix.
  **Reach:** `fixtures/map-presets.json` `open_country.depots`, data.
- **Verdict:** sound. **Confidence:** low.

**The hangar's body is 24 × 16 × 8 m.** That's Quonset size, in
`street_props.bodies`. A 30 × 20 m hangar rarely found a straight enough
stretch of road for its yard. The art is fitted to this box.
- **When:** 17. **Gap:** no size given. **Reach:** generated maps and the
  hangar art.
- **Verdict:** sound. **Confidence:** low.

**A unit switch holds the last frame and blurs it.** Every pick rebuilds the
stage's scene (see "each show stands on a presentation of its own"), and a
freed renderer leaves the canvas clear, so the menu's amber backdrop flashed
through on every pick. A 2D canvas behind the live one now holds the last
drawn frame. A `data-switching` mark on the stage blurs, dims and slightly
scales both canvases until the next unit stands. The pause reads as a change,
not as the unit just leaving. The alternative is an in-place row swap inside
`BrowserScene`, a renderer change for every caller.
- **When:** 11. **Gap:** the plan didn't foresee the flash. **Reach:** CSS
  and the stage component.
- **Verdict:** sound. **Confidence:** low.

**The stage draws the whole screen behind the glass panels.** The unit is
framed in the centre cell (`armoryCamera.ts` slides the orbit's target into
that cell). The chosen mock A has the yard filling the screen behind the
panels. A canvas confined to the cell read as "two scenes stuck side by
side": a midday yard beside the amber backdrop. The backdrop now shows only
where no stage is drawn.
- **When:** 11. **Gap:** the plan didn't say whether the stage owns the
  screen or the cell. **Reach:** the page layout and camera.
- **Verdict:** sound. **Confidence:** low.

**The show-loop fade is 0.5 s, in and out.** Switch from a tank to a truck
and the tank's engine fades over 0.5 s while the truck's ramps in over the
same time. That is a quick crossfade, well under the menu bed's 1.2 s
screen-change fade, which would drag on a fast sweep down the list.
- **When:** 07. **Gap:** no fade given. **Reach:** `appAudio.ts`
  `SHOW_FADE_S`.
- **Verdict:** sound. **Confidence:** low.

**The distant-impostors receipt is not re-recorded.** Its digest
(`specs/distant-impostors/baseline.json`) is the benchmark's Metro XL seed 4
battle at tick 90, recorded in the browser on 2026-10-21. A native run of the
same recipe gave a different digest even before depots. So either the native
path differs from the browser's, or the receipt was already stale. With
depots on standard-profile maps it moves again. An unproven digest is worse
than a stale one, so it was left as it is. It is still stale at the close.
- **When:** 17. **Gap:** the receipt belongs to another spec. **Reach:** the
  next person to trust that baseline.
- **Verdict:** sound, with the re-record still open: run the browser
  benchmark and record it. **Confidence:** low.

**The bare flat-map literal stays in most scenario tests.** `MapDefinition::flat`
now owns a flat map. The two test helpers, the movement scenarios and
`shelter.rs` call it. About 58 other test files still write `fog_cell_m: 8,
height_grid_m: 4, slope_cutoff_deg: 35` inside whole scenario JSON, among
roads, props and encounters. Folding those in is a test-wide refactor.
- **When:** 02, extended at the close. **Gap:** the plan named two copies.
  **Reach:** a later test-cleanup pass.
- **Verdict:** sound for this spec's scope. **Confidence:** low.

### Medium confidence

**The hangar ruin is a ground body for infantry and a wall for vehicles.**
HE brings a hangar down into `hangar_ruin`, 1.5 m high. Soldiers can climb
into the twisted steel. They take light cover from it whichever way the fire
comes, as rubble or a crater covers whoever stands in it (`cover.rs`: a body
that blocks no infantry covers who stands in it). No vehicle can cross it. It
stops no rounds and hides nothing, and it is terminal. A war-film viewer sees
infantry scrambling through wreckage and a jeep that can't. This is the row
as shipped since slice 14. The earlier ledger described a ruin that also
blocked soldiers, which the row never was.
- **When:** 14. **Gap:** the plan named the prop, not its physics. **Reach:**
  `fixtures/props/military/works.json`, data.
- **Verdict:** sound. **Confidence:** medium.

**The military rows' physics.** Each row is ordinary, with no garrison:
- **hangar:** immovable, blocks everyone, stops rounds, hides what's behind
  it, heavy cover, metal. It has `hp` 800 at armour 0.5, so a tank's HE takes
  about sixteen direct hits, a barrage brings it down, and machine guns barely
  mark it.
- **revetment:** immovable, blocks, stops rounds, hides, heavy cover, and no
  `hp`. Earthworks soak shelling, like dragon's teeth.
- **HESCO:** heavy (no tank shoves it), blocks, stops rounds, heavy cover,
  and soldiers fire over it, so it hides nothing. It has `hp` 300 and bursts
  into rubble 0.4 m high.
- **tent:** light, a vehicle flattens it, canvas hides who's inside, and it
  stops no round and gives no cover. It has `hp` 30.
- **flagpole:** light, topples. It has `hp` 20.
- **supply net:** blocks, stops, hides and conceals nothing (L3). It has `hp`
  30, so a burst on the stack it covers tears it away rather than leaving a
  net over nothing.

No rule names any of them. The bodies decide everything.
- **When:** 14 (`tweak-mechanics`). **Gap:** the plan gave the kinds, not
  their columns. **Reach:** every battle on a map with a depot.
- **Verdict:** sound. **Confidence:** medium.

**Depots ship with no balance retune.** The tournament corpus and the battle
sweep ran paired, once with depots and once with the density set to 0.
Depots moved the maps' digests, and no battle reached a different result.
Depots change the ground, not who wins.
- **When:** closeout. **Gap:** Q3 asked for a balance report, not a target.
  **Reach:** depot density stays a free choice (see the user-owned group).
- **Verdict:** sound. **Confidence:** medium.

**A depot stands as one whole compound.** A home's yard keeps `clear.yard_m`
from every other yard, its own group's included. A depot's pieces' yards may
touch each other and keep that distance only from everything else.
`Ask.own_from` marks which yards are the asker's own, and homes pass the
current count, so they're unchanged. All of a depot's pieces take the same
side of the road: a hangar on one side and its berm on the other read as
things dropped along a road. If any piece finds no room, the whole depot
rolls back (lots, yards, bodies, tally). A home group keeps as many homes as
fit, but "three or four props" is what makes a depot. A piece that finds no
room tries again further along, up to four times. Without that, no hangar
group ever stood whole. Its yards are paved (`SurfaceKind::Paving`): on bare
ground the bodies read as boxes in a crop field.
- **When:** 17. **Gap:** the plan reused the homestead walk (L4) without
  knowing a depot needs a compound. **Reach:** `open_country/steads.rs`.
- **Verdict:** sound. **Confidence:** medium.

**Depots are placed after homesteads, on their own random stream.** Placing
depots last on `country/depots` leaves homes unchanged, so a map with no
depot station is byte-identical to before. Depot stations keep 1.5 km from
other depots, not from homes, though their yards still keep apart. The
map's even-halves rule counts a depot by the middle of its first lot.
`fairness.depots` is 1.
- **When:** 17. **Gap:** ordering unasked. **Reach:** generated-map
  identity.
- **Verdict:** sound. **Confidence:** medium.

**Each show stands on a presentation of its own.** Every pick builds a fresh
`ScenePresentation` and scene, and frees the old pair once the new one
stands. A superseded or failed show frees its own. With the battle's scenery
placed, a second show on the same presentation drew instances the
presentation thought unchanged, and the GPU refused the frame: a cache that
outlived its scene. A battle never rebuilds a scene on one presentation, and
now the stage doesn't either. The cost is the ground and scenery placed
again on every switch: 0.4–1.4 s after the first, and a first open of about
21 s while the menu's prefetch downloads. The alternative was finding and
resetting the stale cache in Rust.
- **When:** 16 (replacing slice 04's in-place rebuild). **Gap:**
  unforeseen. **Reach:** switch time. A Rust cache reset could win it back.
- **Verdict:** sound. **Confidence:** medium.

**A parked aircraft is "grounded", one feed field.** The showcase sets
`FeedUnit::grounded` on every unit. The driver flies a unit only when its
type flies and it isn't grounded (`airborne = ty.airborne &&
!unit.grounded`). A parked helicopter therefore poses as a ground vehicle
does: no rotor travel, blur, disc or flight lean, its blades drawn at rest.
The showcase reads no type to set it. Battles never set it, so their frames
are byte-identical. The alternative, a `rotors_stopped` flag, would have
kept the flight lean on a unit standing still.
- **When:** parked-aircraft pass (the user's "parked rotors are stopped").
  **Gap:** how to say "on the ground". **Reach:** the driver's feed.
- **Verdict:** sound. **Confidence:** medium.

**A type that flies is silent on the ground; a ground vehicle idles.**
`idle_loops(tables, vehicle_class, flies)` returns nothing when `flies`.
`idle_loops_of` reads `flies` from the row's mobility through
`fielded::airborne`, the same reading the pose uses. A test gives one sound
class to a wheeled and a flying type, and only the wheeled one idles. A tank
at rest rumbles. An aircraft on an apron sits cold, because a running engine
would read as about to take off.
- **When:** parked-aircraft pass. **Gap:** the idle sound of a parked
  flier. **Reach:** `presentation::sound::idle`.
- **Verdict:** sound. **Confidence:** medium.

**The backdrop's relief is the one height owner past the yard.** Hills past
the yard are `BackdropRelief`
(`crates/presentation/src/static_world/ground/relief.rs`): the battle's own
relief features (`contract::map::Relief`), sampled on cells and split by the
simulation's own triangle rule (`tables::cell_height`). Four readers share
it: the backdrop ground, its trees and hedgerows, the Armory camera's
clearance (`Framing.ground`), and the camera tour's ground check. A battle
draws relief only inside its map, so relief past the yard is relief past a
map's edge. The relief must lie wholly past the map and the cells beside it,
so the yard and every spot stand at exactly 0. The rejected alternative made
the yard a real height grid. It needed a 3–4 km map that pushes every
treeline past the hills and compiles a million-sample world on each open.
Battles pass no relief, so their backdrop is byte-identical.
- **When:** relief pass. **Gap:** the user asked for terrain. **Reach:**
  `Presentation.set_ground(…, relief?)`, the wasm `BackdropRelief`, and
  `Relief::footprint()` as the one owner of a feature's box.
- **Verdict:** sound. **Confidence:** medium.

**The yard's ground stops at 180 m, and the battle's backdrop runs on.** Past
the yard's own ground, the presentation draws the battle's backdrop: the
biome's field patchwork, hedgerows, trees and copses, as past a battle map's
edge. The yard's ground is the battle's ground material, so it meets the
backdrop with no seam. 180 m holds the camera's farthest eye, so no tree
stands in its orbit. The yard authors no tree and keeps no tree list. The
alternative, a `trees` list in `yard.json`, would have placed the battle's
trees a second time.
- **When:** 16 (replacing slice 11's 2 km plain ground). **Gap:** the user
  asked for trees. **Reach:** `yard.json` `ground.half_m`.
- **Verdict:** sound. **Confidence:** medium.

**A pad is a tinted quad at its own level.** The yard's ground gains `pads`
(`{ at, half_m, yaw, tint }`), paved rectangles drawn as tinted quads just
above the ground. Without them every unit stood on one khaki plane. Each pad
stands 5 mm above the ground or the pads it overlaps (`padLevels`, oriented
boxes from `math`), so pads turn freely and overlap (slabs over joints, taxi
lines over slabs), and the yard's pads stack at most three high (1.5 cm). A
test keeps every pad under the 2 cm a unit may sit off its ground. The
ground's height query stays 0.
- **When:** 16. **Gap:** the yard had one ground tint. **Reach:** the yard
  schema; `plainWorld.ts`.
- **Verdict:** sound. **Confidence:** medium.

**A top just under an eye takes the fog's roof rule.** A 1.5 m hangar-ruin
top, seen by 1.6 m infantry eyes, drew as unseen in plain sight.
`fogSeenSurface` (`fog.wgsl`) tests whether a face turns to the eye from a
probe point 0.1 m above it, then lets the roof rule rescue an up face, but
only for a surface above the eye. The top was neither. The rule now runs for
every up face whose probe point stands level with or above the eye. Those
are exactly the faces the face test turns away, so the two split up faces at
one height with no gap. Geometry decides, with no height constant or named
prop. A GPU probe test (`renderer-core/tests/probes.rs`) fails on the old
condition.
- **When:** closeout. **Gap:** found by slice 14's gate. **Reach:** every
  battle's fog on low tops.
- **Verdict:** sound. **Confidence:** medium.

**Browser sessions publish `derived` through a wasm export.** Before this,
the browser's catalog came from the resolved rules, which refuse `derived`
as an authored field, so the Armory's sheet would have read nothing. A new
export, `publish_weapons`, gives the browser the same published rows as the
committed catalog. `composeCatalog` takes one `CatalogResolver`. The
TypeScript types split to match: `WeaponDefinition` is the rules row, and
`WeaponRow` adds `derived` (`SessionCatalog.weapons`). Every sound setup takes
`catalog.rules.weapons`, the rules rows, because sound admission refuses
`derived`. The alternative was taking the committed game view's rows. That
is right for the three catalog sets but wrong for uploaded scenarios, which
carry their own rules.
- **When:** 05. It goes past K5's text, which names only the catalog view.
  **Gap:** K5 didn't say how the browser gets the facts. **Reach:** every
  catalog composition.
- **Verdict:** sound. **Confidence:** medium.

**The mechanics editor shows the saved row's derived facts.** An author edits
a weapon and the read-only "derived" panel (`DerivedFacts.tsx`) keeps showing
the saved row's facts until they save, and says so. Reading the preview's
catalog after Preview would be a second data path for a read-only panel.
- **When:** 05. **Gap:** unasked. **Reach:** editor only.
- **Verdict:** sound. **Confidence:** medium.

**Framing is fitted per unit and per view angle.** At each orbit angle, a
framing is the distance at which the unit's rest-pose box spans that share of
the centre cell across and up, every corner at its own depth. Turning keeps
the zoom, and a turn that would clip the box pushes the eye back. A bounding
sphere (the first try) framed a helicopter loosely, because the rotor sweep's
diagonal sized it. The box is still square around the rotor disc, so a
helicopter's nearest zoom is less tight than a tank's.
- **When:** 12. **Gap:** the plan expected fixed spot cameras. **Reach:**
  `armoryCamera.ts`.
- **Verdict:** sound. **Confidence:** medium.

**The load line counts what the Armory holds, not what the network moved.**
A selection's download is `ArmoryAppearances.held()`: the files the newest
selection read, with the yard and the catalog, 7–18 MiB. Resource timing also
counts the menu's background prefetch of every runtime file, which showed
200–300 MB per switch that the Armory never asked for.
- **When:** 11. **Gap:** S5 didn't say how to measure. **Reach:** the
  `armory` scene's readout.
- **Verdict:** sound. **Confidence:** medium.

**Failures stay on the stage, behind one boundary.** Art, fielding or scene
admission failing throws into `ArmoryStageBoundary`, a React error boundary
that draws the error in the cell and clears on the next pick, which builds
the stage again. The shell's cover lifts on a failure, and the list, sheet and
menu stay usable. Failures never reach the shared resources.
- **When:** 11. **Gap:** unasked. **Reach:** `ArmoryStage.tsx`.
- **Verdict:** sound. **Confidence:** medium.

**A planned card shows the yard empty.** A planned card has no unit to field,
so the stage shows the yard alone: from the art already on show, or, on a
first open, the yard's own art from the motor pool's camera. Idle loops stop.
- **When:** 11, corrected at the close (it had drawn an unpainted stage).
  **Gap:** unasked. **Reach:** the stage.
- **Verdict:** sound. **Confidence:** medium.

**The Armory round trip counts the stage's own allocations.** The app
journey visits the Armory and leaves. Each GPU allocation records the stack
that made it, and the check holds that nothing the Armory's stage made
survives leaving. Comparing with what the menu's backdrop churned meanwhile
gave two noisy samples.
- **When:** 11, sharpened at the close. **Gap:** unasked. **Reach:** the
  street tour.
- **Verdict:** sound. **Confidence:** medium.

**Grounded is judged from each model's placement plus its far-pose bounds.**
The showcase lab and the Armory judge "on the ground" from each drawn model's
placement plus `farPoseBounds`, not from an idle clip's frame. An aircraft's
gear is its `wheel_*` or `track_*` subtree, the validator's own rule. A skid
or belly rests on the node that carries it, so four skid helicopters rest on
`hull`. The world's trees are left out, because the presentation sinks them
5 cm (`SINK_M`) so no trunk floats.
- **When:** 04, 16. **Gap:** how to judge grounding. **Reach:**
  `stageProbes.ts`.
- **Verdict:** sound. **Confidence:** medium.

**One rule decides which tab a family's tile sits under.** A family sits
under the group of its first typed variant, and a family with no typed
variant takes its category's first group. One owner, `familyGroups` in
`purchaseFilter.ts`, serves the picker and the Armory, so a tile never
splits across two tabs. No shipped family differs between per-card and
per-family grouping today.
- **When:** 06 grouped per card, 08 per family, and the close made one owner.
  **Gap:** the plan's seam was per card. **Reach:** both browsers.
- **Verdict:** sound. **Confidence:** medium.

**The gallery's planned card borrows the first planned card's id.** A
silhouette is keyed by card id and no test unit is planned, so the UI
gallery's planned tile borrows a real planned card's id for its silhouette,
with a fabricated name and weapons. A new first planned card changes that
picture.
- **When:** 08. **Gap:** none in the test units. **Reach:** one baseline.
- **Verdict:** sound. **Confidence:** medium.

**The picker widens to 840 px and its tabs sit 6 px apart.** The nine tabs
and search needed about 742 px against the old 694 px box. At 840 px, 58 px
are spare at 1280. The diamonds and type are unchanged, so tabs keep every
menu choice's look. The search stays a field at every width.
- **When:** 06. **Gap:** the plan offered shortening labels or collapsing
  search. **Reach:** `hud.css`.
- **Verdict:** sound. **Confidence:** medium.

**`ui` baselines pin the Armory page without a stage.** The live stage
animates, so it can't be a pixel baseline. The gallery's `armory` (1280 ×
800) and `armory-1920` pin the page and sheet, the `armory` scene's evidence
carries the stage, and `ArmoryVisit` (the real growing roster) is exempt in
`uiCoverage`.
- **When:** 08, 11. **Gap:** unasked. **Reach:** visual tests.
- **Verdict:** sound. **Confidence:** medium.

### High confidence

**Menu pages are plain data; each page's contents are a record.** `MENU_PAGES`
rows (`id`, `path`, `title`, `tile`) are plain data in `menu/pages.ts`.
`menuHref`, the router and the browser scenes (node `.mjs`) read the same
paths, and a component in a row would make an import cycle. Contents live in
`menu/MenuVisit.tsx` `CONTENTS: Record<MenuPageId, ComponentType>`, which is
exhaustive, so a row with no contents fails to type-check. Four pages are
eager. The Armory is `lazy`, so its stage, yard document and art library never
reach the menu until a visit, and a fault in them reaches only its own page.
A page whose code is still loading holds the shell's cover, so the cover
lifts once.
- **When:** 01, closeout. **Gap:** the plan put a lazy component in the row.
  **Reach:** every menu page.
- **Verdict:** sound. **Confidence:** high.

**`menuPageOf` lives in `router.tsx`.** Its `null` answer (a lab or battle
route) needs the lab route table, so `pages.ts` would need a cycle.
- **When:** 01. **Verdict:** sound. **Confidence:** high.

**The shell derives focus, back-state and readiness from the address.** When
the address goes from a page to home, the page left takes focus, whether the
player used Back, Escape or the browser's Back, with no flag at any call
site. `fromMenu` is router state in `history.state`, so after a refresh Back
still steps to the home entry. `PageVisit` keys on `location.key`, so the
Armory's raw `history.replaceState` on a pick never remounts the page.
`detected` means the graphics tier is settled, the same condition that gates
the runtime prefetch.
- **When:** 01. **Verdict:** sound. **Confidence:** high.

**`MenuShell`'s loading tasks wrap the page too.** The stage reports its
`renderer` task, which holds the first cover until the unit draws. No other
page reports a task.
- **When:** 11. **Verdict:** sound. **Confidence:** high.

**`menuHref(null)` is `/`.** A replay's Cancel lands on home, because a
replay names no map to set up.
- **When:** 01. **Verdict:** sound. **Confidence:** high.

**Unit groups are a table with `null` for "the rest".** `UNIT_GROUPS`
(`purchaseFilter.ts`) gives each group its category and the roles it claims,
with `null` for the rest of the category. `unitGroup` takes the first of a
card's category's groups that claims one of its roles, else the category's
first group. IFV·APC is the vehicles' catch-all, and a support role no group
claims lands in ARTY. A label, order or split change is a table edit.
`unitGroup` takes only `roles`, so tests fabricate one without a type.
- **When:** 06. **Verdict:** sound. **Confidence:** high.

**Weapon facts are published under `derived`, from the contract.**
`contract::weapons::published` is a contract function, not code in the
native-only `sim::fixtures`, because the browser needs the same rows.
`WeaponDefinition::derived()` gives `fires_regardless` and `reach`. A `Reach`
row is `{ layer, range_m, only?, intercepts }`. `only` is absent when the row
takes everything its layer holds. Otherwise it lists the kinds it does take,
in `TargetClass` order, never `missile`. A layer listing only missiles
publishes `only: []` with `intercepts: true`. `reach` reads through `against`
and `engages`, so the class match keeps one owner. An authored `derived` is
refused by name.
- **When:** 05. **Gap:** K5 named the facts, not the shape. **Reach:** the
  catalog view's rows.
- **Verdict:** sound. **Confidence:** high.

**`MapDefinition::flat` owns a flat map; `WorldGeometry::flat` builds on
it.** Both test helpers took extra map fields, so what they duplicated was
the map, not the geometry. `Showground` uses `WorldGeometry::flat`.
- **When:** 02. **Verdict:** sound. **Confidence:** high.

**`original_carriers` is the one owner of a squad's first claim on its
guns.** It lives in `sim/weapons/operators.rs` beside `assign_operators`. It
takes each mount's spec (`None` for a gun not wanted) and each member's slot
(`None` once fallen), and returns member indices. Spawn, `assign_operators`
and `Showground` all read it. Spawn had its own non-exclusive search until the
close. `Showground`'s test compares a squad at rest with a real battle's
spawn.
- **When:** 02, closeout. **Verdict:** sound. **Confidence:** high.

**The showground is a 400 m flat world, and the squad reports offsets.** The
squad stands near its middle and is reported as offsets, so it sits about
(0, 0). Soldier ids are `slot + 1`, and it scatters as battle unit 0. Flat
ground scatters the same anywhere. `Showground` keeps Rust-typed `admit` and
`at_rest` beside the wasm `new` and `squad`, because `JsError` can't be
built natively.
- **When:** 02. **Verdict:** sound. **Confidence:** high.

**A presentation shows a battle or a showcase, never both.** `set_showcase`
errs while a battle is loaded, and the reverse also errs. Each would be the
frame's one effect batch. A world of statics alone loads beside a showcase.
Showing the next unit replaces the last: there is no `clear_showcase`.
- **When:** 03, closeout. **Verdict:** sound. **Confidence:** high.

**The shown unit's rows are the presentation's own.** They sit after the
world's rows and before the lab's suffix, so the browser loads their art as
it loads a battle's, and a row change rebuilds the scene by the rule a
battle's reload follows. The wasm `set_showcase(fielded, input)` returns the
world header's address, as `install_lab_assets` does, and reads the
`{units, effects}` that fielding returned, with clip tables staged as
`load_world` stages them.
- **When:** 03. **Verdict:** sound. **Confidence:** high.

**Posing has one owner, `presentation/posing.rs`.** The battle's frame and
the showcase draw soldiers and vehicles through `UnitArt`, share a scratch of
node transforms, and build models with `Model::unit`. That removed the
battle's nine-argument model closure without lint exemptions.
- **When:** 03. **Verdict:** sound. **Confidence:** high.

**Idle sound reads the whole sound setup.** `idle_loops_of(setup, kind)`
parses the battle's one `SoundFrame` setup through the same admission
`Sound::from_json` uses, and finds the kind's class there. A smaller
idle-only setup would be a second parser.
- **When:** 03. **Verdict:** sound. **Confidence:** high.

**`ShowcaseStage.show(unit, seed, art)` is async and takes a `StageArt`.**
`StageArt` is `{ appearances, runtime }`, the one type `ArmoryAppearances.show`
returns. A unit is fielded from the runtime files of the library that
installed its art, never the shared `gameRuntime`. `create` is synchronous
and builds no scene until the first show. `models` and `scenery` are
functions of each show's appearances. A show another supersedes resolves with
none, and is dropped by its number, so two rebuilds finishing out of order
never keep the older scene.
- **When:** 04, 09, 11. **Verdict:** sound. **Confidence:** high.

**`ArmoryAppearances` keeps its own art library.** `new ArmoryAppearances(units,
fetcher?)` takes the session's unit catalog once, because a kind's art comes
from `appearancesFor`. Each `show` makes a fresh `AppearanceLibrary`, so the
loader stays the one owner of loading and a dropped selection's art goes with
its library. A memo keeps the newest answered selection's files as bytes, so
the yard, catalog and templates download once while the last unit's bytes
leave on the next switch. A failed or superseded call never replaces the
memo. A superseded call's next fetch rejects with `AbortError`, so a fast
sweep down the list stops downloading. `catalog()` shares that fetch, so the
art load reuses the bytes. The default fetcher is `countedFetch`, so the
loading counter moves as a battle's does. The yard is the loader's own
`scenery` filter.
- **When:** 09, 11. **Verdict:** sound. **Confidence:** high.

**The showcase lab runs on the Armory's parts.** `/labs/showcase` shows one
`ShowcaseStage` per unit, three panels side by side, each with its own
orbit, on one `ArmoryAppearances` per panel. The parked-aircraft probe keeps
the shared library, because it loads every aircraft at once. Both pages
build a stage from `armory/stageInputs.ts` and judge it with
`armory/stageProbes.ts`. A `flight(kind)` probe finds rotorcraft by their
mobility, not by rotor discs.
- **When:** 04, 11, parked pass. **Verdict:** sound. **Confidence:** high.

**`plainWorld` is its own module.** `apps/battle-lab/src/plainWorld.ts` is
the one plain flat world, used by the facade lab, the showcase and the yard.
It has no top-level await. Importing it from `benchWorld` would have made the
menu's module graph wait on the presentation load.
- **When:** 10, 11. **Verdict:** sound. **Confidence:** high.

**The yard names prop kinds, and the art catalog names their art.**
`yard.json` names kinds (`crate`, `tooth`). `yardScenery` asks the runtime
catalog which scenery appearances draw them (`propsDrawnBy`), because only
the catalog knows `crate` is `crate_stack`. `yardModels` draws each prop by
the first installed appearance of its kind, at its authored size, one module
per entry for modular props. `Spot` is a place plus its id, and a camera pose
is an orbit with the game camera's lens. `admitYard` checks shape and that
every body names a spot. Layout rules are the tests'.
- **When:** 10. **Verdict:** sound. **Confidence:** high.

**The scenery's art has one owner.** `worldSceneryNames(biome)` and
`worldScenery(biome, installed)` (`worldMesh.ts`) serve the battle's world
layers and the yard. `ShowcaseStage` takes `scenery` beside `models`, and
turns presentation scenery on when it returns some.
- **When:** 16. **Verdict:** sound. **Confidence:** high.

**The Armory camera's settings live in `presentation.json`, admitted in
Rust.** `camera_armory` is admitted by `ArmoryCamera`
(`crates/presentation/src/settings/ui.rs`). The checks cover the lens, a
pitch range within (−π/2, π/2), `0 < far < open <= near <= 1`, positive
speeds, and the battle camera's clearance checks, now one shared function.
`CameraPresentation` carries `pitch_limits`, so one controller serves both
cameras. The battle keeps its values in `gameCamera.ts`.
- **When:** 12. **Verdict:** sound. **Confidence:** high.

**The camera is a class plus a hook.** `ArmoryCamera` holds orbit, framing and
clearance and gives the camera for a frame. `useArmoryCamera` wires
left-drag and the wheel on the canvas alone. The stage draws outside React,
and a hook returning a camera would re-render React every frame. A unit's
bounds come from data: a hull's art at rest, and a squad's scatter with each
soldier `soldier_height_m` tall. A drawn frame from the last spot's camera
may have culled the new unit. The eye keeps clear through renderer-core's
clearance over the unit's box, each prop's box and the relief's ground. The
camera opens at the spot's yaw and pitch, aimed at the unit's centre. The
camera tour drives real drags and wheels through 8 yaws × 3 pitches × both
zoom ends on each spot.
- **When:** 12, relief pass. **Verdict:** sound. **Confidence:** high.

**Show loops ride the one audio context.** `AppAudio.showLoops(loops)` keys
loops by sound name. A repeated name's last entry wins. Voices take ids from
16 up, open-ended, above the bed (1) and the cue (3). The bed's restart
clears every voice but show voices, through an id filter on
`WebAudioSink.clear`, because a fading bed has already left the id map.
Loops start only once `SoundBank.prepared` says so, and the bank already
joins duplicate preparations. They ride the `units` bus, heard at the
camera, with no low-pass, reverb or position. Leaving the menu screen
forgets them. Mute keeps them, because mute is a setting. `stats()` reports
`showVoices`.
- **When:** 07, 11. **Verdict:** sound. **Confidence:** high.

**The sheet model is one table and pure data.** Stat names and units are
`SHEET_STATS` (`unitSheet.ts`). Bars go on road and off-road speed (cruise
for a flier), sight per band, visibility, and each ammunition kind's damage,
penetration, aim and reload. Hull HP and armour are plain numbers. Reach is
per band under each kind, so a range bar would say it twice. A mount carries
its first kind's icon, and a planned card's planned weapons are mounts with
no kinds, named as `equipmentName` names real ones. `equipmentName` takes
`Pick<PanelRules, "weapons">`. A card's description shows only when it adds
to the head: 119 of 152 cards repeat their variant name.
- **When:** 08. **Verdict:** sound. **Confidence:** high.

**The page reuses the picker's look.** A tile is the picker's family card
(`UnitFace`), wider, with the name above the price. A family with no typed
variant is the picker's greyed, dashed card. Prices are credits gold on typed
cards and plain on planned ones. Tabs are the picker's `hud-menu-choice`, with
counts in even columns. Empty groups and factions are disabled, not hidden,
so tabs never move. Faction tabs are boxed and larger, so the two levels read
apart. Variant chips are the menu's variant choice. The fires-regardless mark
is a generated HUD icon (`fires_regardless`), with U7's sentence as a
focusable tooltip on the mount's or each kind's head. The Armory tile stands
third, so Settings stays last.
- **When:** 08. **Verdict:** sound. **Confidence:** high.

**The page is the whole plate, with panels in columns.** The list (with the
page head on it, a subgrid) is the left column, the sheet the right, the stage
between. Columns are `clamp(300px, 22vw, 400px)` and `clamp(360px, 26vw,
480px)`, and the sheet scrolls. The head stays the shell's `PageHead`,
exported so the gallery draws it, because a fixed panel can't escape the
plate's `backdrop-filter`. `ArmoryPage({ cards, stage })` stays free of 3D
imports, and `useArmorySelection(roster)` keeps a faction switch on the open
card where that faction fields it.
- **When:** 08. **Verdict:** sound. **Confidence:** high.

**The key art comes from the Duet CLI.** The model is
`openai/gpt-image-2.5-flare`, at 1536 × 512, processed as the other tiles
are. A tight, low three-quarter view replaced the first wide shot, which
stood out of the set.
- **When:** 08. **Verdict:** sound. **Confidence:** high.

**The `armory` scene drives the real menu.** It covers the tile, picks,
refresh, an unknown unit and Escape, the stage on each spot, and the camera
tour. Its probe is `window.__armory` (`armoryProbe.ts`), following the
battle's `window.__lab`. Voices and switches outlive the stage, so they can
be read after Back. Unit tests drive the real menu under `AppShell`
(`visitShell`), with the 3D parts stood in.
- **When:** 08, 11. **Verdict:** sound. **Confidence:** high.

**`Steads<P>` is typed by its pieces.** `homesteads: Steads<Homes>` and
`depots: Steads<Vec<BodyGroup>>` share the station walk through `Pieced`, so
no reader pulls a variant back out under `unreachable!`. `spread_m` belongs
to `Steading`, because depots space their yards too. The preset rows stay two
private serde structs (`HomesteadRow`, `DepotRow`), because
`deny_unknown_fields` can't flatten a shared row. Which templates a home may
build is `Homes::choices`, and `furnish_yard` takes the yard and the homes.
The types and `Pieces::depot_of`, the one parser of a depot's lot id, are in
`open_country/rules.rs`. The station walk is `open_country/steads.rs`.
- **When:** 13, 17, closeout. **Verdict:** sound. **Confidence:** high.

**A depot piece is `count` bodies under an optional `net`.** The supply piece
is two or three pallet stacks under a `supply_net`, so a net only ever
covers supplies (L3), because only that piece names one. No rule reads a
prop's id. Admission refuses a row wider, deeper or taller than its net.
Pieces name kinds only, and their boxes come from `street_props.bodies`.
- **When:** 17. **Verdict:** sound. **Confidence:** high.

**A depot holds no new rule.** A depot by a road is a strongpoint for
whoever reaches it first. Even halves keep it fair. Its yards keep off kept
approach corridors (`yard_fits`) and off the carriageway, so it never closes
the main approach or blocks the road. The rows' bodies decide every effect.
- **When:** 17 (`tweak-mechanics`). **Verdict:** sound. **Confidence:**
  high.

**The map tools show depots.** The plan picture draws depot yards in
fuchsia, a hue no other layer uses, and rings each on a whole-map view as it
rings a bridge, with a legend entry. Each depot is a workbench feature
(`depot-<n>`, rule group `open_country.depots`) with its own crop. Field
help maps each flat steading key to the type that documents it.
- **When:** 17. **Verdict:** sound. **Confidence:** high.

**The military exporter is one entry with family modules.**
`packages/scene-assets/blender/military.py` is the exporter, with arguments,
lint and export. The kinds live in `military_hangar`, `military_works` and
`military_camp`, on `military_parts`' helpers, so no file nears the size
budget. New texture recipes (`hesco_liner`, `camo_net`) live in
`textures_military.py`, because `textures.py` may not grow. Each prop lints
itself (`--lint`) with `vehicle_lint`'s floating, ground and body-overlap
checks, not a second lint.
- **When:** 15. **Verdict:** sound. **Confidence:** high.

**Art may reach past its body where nothing is blocked.** The flag flies
1.5 m out from its 16 cm pole at 7 m up, over everyone's heads, through the
flagpole appearance's footprint tolerance (1.6 m). The ruin's kind takes a
1.5 m rubble allowance (`hangar_ruin` in `scenery.ts`), so sheets that slid
off lie flat round its plan, as a wreck's thrown plate does. A wider body
would move generated maps for a cloth.
- **When:** 15. **Verdict:** sound. **Confidence:** high.

**Tests carry their own rows and prove effects through battles.**
`crates/sim/tests/shelter.rs` pushes its own `test_*` props, so no test reads
a military row and no test row ships. A body with no cover and no stop
"never shields" is proved by whole firefights: the blue soldiers' health,
tick by tick, is the same with the screen as without it.
- **When:** 14. **Verdict:** sound. **Confidence:** high.

### Trivial calls (24)

Splitting files for the size ratchet: `weapons/operators.rs`,
`open_country/steads.rs`, `measure.rs` and `bodies.rs`, the editor's `api.ts`,
the catalog-view tests, the open-country test submodules, and
`useBattleSession`'s namespace import. `MainMenu.tsx` renamed to `menu/Home.tsx`.
The developer list's unused disabled-entry branch dropped. Shared
`AltitudeLayer` and `TargetClass` types. A hull's lab view standing back 6.5×
its half extent. The `lobby` scene opening `/skirmish` directly. One "tile
opens its page" check per page. The tutorial's buy still cropping the tab row
in. The gallery's picker cards reshuffled. `unit-roster` and the app journey
left untouched. The W09 amendment dated and credited. The showcase's replace
and exclusion tests kept apart. The idle test letting a watcher stand still. The AH-64D
reference photo (`assets/references/ah_64_apache/`). References kept to real
photos, with the chosen mock standing in for the genre's framing. Slice 08 in
two commits. 15a and 15b in one commit.
