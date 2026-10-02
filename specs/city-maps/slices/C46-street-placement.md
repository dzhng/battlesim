# C46: street placement

**Depends on:** C44, C53. **Kind:** slice.

## Question
Does street placement keep playable space?

## Contract it unlocks
`place_street_props(MapPlan, catalog, seed) → props`, inside `crates/mapgen`: curb setbacks; no overlap with buildings, entrances or the reserved plain's open corridor; infantry and vehicle routes preserved; bounded rejection attempts. Street trees use C74's one tree generator and a named random stream.

## API seam
`crates/mapgen`.

## What the human can run or see
A block traversal GIF and a top-down density PNG.

## Verification
- Deterministic placement.
- Route-preservation test.
- City digests change (named).
- Frame-cost row.

## Delegated to the implementer
Density and spacing data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every placement resolves to a body and an appearance.

## Feedback that would change this slice
Furniture that blocks required approaches changes placement constraints before visual density is accepted.

## Outcome — placement pass (2026-10-02)

**The contract.** `mapgen::street_props::place_street_props(&plan, &request, &templates, &catalog, &presets)`
answers the bodies to add to a built plan's `props`, and `generate` runs it after the
parcel pass. A placed body is an ordinary map prop, in the plan and in the map alike:
`{ kind, center, yaw, half_extents }`, with no id, so it takes the ids after the
buildings' parts; a saved map stores it in `map.json`'s `props`. What is placed is data:
`street_props` in `fixtures/map-presets.json` (each body's box and the room it keeps,
the lane, kerb, wall and door margins, the parking rule, the construction site) and a
`props` row in each district kind (its share of kerb parked along, its verge rows, its
yard stock, its chance of a site). The generation boundary gained a fourth document,
the unit and prop catalog (CLI: `fixtures/catalog.json`; Wasm: the documents the rules
carry as `catalog`): its widest hull sizes the lane, and every placed kind must be one
of its prop types. Generator `layout-8`, presets `layout-presets-7`.

**The answer to the slice's question** is yes, at a price: street placement keeps
playable space only where it stands back from the street. The decided design put
parked cars at the kerb, on the carriageway. The simulation does not hold that:

- its vehicles keep to the right of a road's middle, so on a 7 m street the widest
  hull's lane already overhangs the kerb (a hand-drawn street in the tests shows a
  vehicle shoving cars parked on the carriageway);
- a road journey checks that lane on the 2 m planning grid, and where a body that
  stops the mover stands within a few metres of it the check fails, the detour must
  land on a lane point that is itself beside a body, and the road closes to that mover.

With the cars on the carriageway's outer edge, as designed, the widest hull shoved a
car on 235 of its 443 street drives in the sweep, and 140 drives by the other hulls
took a detour of more than 30 m. With every body 0.9 m past the widest hull from the
middle (just off the kerb), the sweep's simulation answered: 8 of 17 maps no longer took the assault ("a jeep has no
route"), 77 of about 1 240 street drives took a detour of more than 30 m, and 5 drives
by the widest hull shoved a body. At 3.7 m past the widest hull (7.3 m from the middle
with today's catalog) every claim holds. So that is the shipped rule, as one preset
number (`lane_margin_m`): furniture stands on the verge and the front of the parcels,
about 3.9 m off a street's kerb. **What that costs the picture:** before a row of
terraces, whose walls are 7.5 m from the middle with a door every 6 m, nothing stands;
cars are in runs along the open sides of blocks, before detached houses and on the
aprons of apartment blocks. A street fight between terraces still has no cover from
this pass. Kerbside furniture needs the simulation's road journey to tolerate a body
beside its lane; that is navigation's to change, and then `lane_margin_m` goes back
to about 0.9.

**The rules, as placed.**

- Parked cars stand parallel to the street in runs of 2 to 5, bumper to bumper, a
  squad's width or more (3 m) between runs, along one side of a street and both of an
  avenue (10 m), on paved streets only.
- Nothing beside a street, car or lamp or tree, stands within 8 m of another
  carriageway's edge: a junction's corners are open. This was the cars' rule; the
  critique found a lamp and two trees crowding one corner, so it is every body's.
- No body stands within 3 m either side of a door's line to the street. The first
  trial at 0.75 m left 89 doors in the sweep where a squad could no longer stand.
- Lamps stand at an even spacing on alternate sides and street trees (the forests'
  `trunk` body, as the prop type `street_tree`) at an even spacing along both sides of an avenue; both are placed before the
  cars, so a run ends at one. Bins, benches, planters, bollards, hydrants, utility
  boxes, scooters and, on avenues, the odd bus shelter are scattered in what is left.
- Industrial parcels get skips and pallet stacks against a wall with no door in it.
- A parcel left open in a centre, apartment or core district is a construction site
  now and then, three to a settlement at most: a cabin, a Heras fence round the parcel
  with a gate on the street, a skip and pallets. The gate's way to the street is kept
  open like a door's (the first picture of a site had a tree in the gateway).
- Every body goes through one check: on the map, off every carriageway and the lane
  beside its middle, a soldier's width from a wall, out of every door's way, apart
  from other bodies by the room each keeps, off the water, off bridges and the run onto
  them, out of every measured open approach.
- Not placed: roadblocks, wrecks and road barriers. A roadblock is something the
  defender prepares, so it belongs to the encounter planner.

**Routes survive** (`crates/mapgen/tests/street_props.rs`, two seeds of each of the
nine cells, bare against dressed, asked of the simulation's own planner and
navigation): 18 of 18 assaults still plan; a squad still stands at each of 121 266
doors and walks to each of 736 sampled from its settlement's centre; each hull of the
catalog still drives each of 1 328 sampled street stretches by a way no more than 30 m
longer, the widest hull (443 drives) without shoving a body.

**Cost** (`city_report`, seed 1, 120 s; instructions retired):

| map | bodies gained | world build G | battle build G | 120 s of ticks and packing G | resident MiB |
|---|---|---|---|---|---|
| Mixed Small | 3 694 | 1.05 → 1.07 | 6.14 → 6.20 | 45.1 → 45.3 (+0.4%) | 222 → 230 |
| Metro Large | 16 825 | 2.51 → 2.58 | 15.78 → 16.07 | 66.0 → 66.4 (+0.6%) | 633 → 647 |

Each row is one map generated twice by the same build, without furniture and with it.

The report's own "crossing" aggregate rises more (79.0 → 82.2 G and 134.9 → 148.1 G, +9.8%):
between ticks the report calls `Battle::load`, which counts wrecks by walking every
prop, so that figure grows with the map's bodies whatever the battle does. The first
densities (30 440 bodies on Metro Large) cost +8.8% of ticks on the build they were
measured on. They were lowered twice,
to under three fifths of the bodies: once for cost, lamps and small furniture most, and once
for admission. Over 100 seeds of each cell every map still generates
(`layout_sweep --seeds 100`), and the largest Metro Large holds 52 049 authored bodies
(25 197 of them furniture) against the game's allowance of 60 000; at the middle
densities it held 57 484.

**Every placed body is drawn.** These kinds have no art (`systems_only`), and a map
prop with no appearance drew nothing. `PropAppearances` (the renderer's one owner of
which appearance draws a prop) now gives such a kind a stand-in: the prototype kit's
unit box, the same metre cube an artless building's parts are drawn from, stretched to
the prop's own box and tinted by its kind (`presentation.stand_ins.tints`), as an
ordinary model instance. So it takes the path props with art take: one the side has seen
shoved is drawn where it was last seen, one it has seen burnt out as its wreck's box,
one destroyed not at all. A forest's trees stay the scenery's; a street's trees are
their own prop type (`street_tree`, the forests' `trunk` body with its own binding), so
nothing asks which trees a forest stood.

**Open.** The models (C45). Kerbside placement, as above. The slice's GIF of a block
traversal and its frame-cost row were not made.
