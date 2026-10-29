# Decisions

These are the user's decisions from the 2026-10-27 unknowns walk
(`explore-unknowns`), each with its reason. Slices carry them as givens. Entries
marked *agent* were answered on the user's behalf from their confirmed
preferences, and the user may overrule them. A *revised* entry replaces an
earlier answer; the note says what changed it.

**Confirmed preferences** (they scope the *agent* entries):

- Easy to balance comes first. Each number means exactly what it says.
- A filmic distinction is worth a number when it changes which unit is right for
  the job.
- Land that reads as real beats simpler generation.
- Behaviour that emerges from physical rules beats a hard-coded one.
- Otherwise, take the recommendation.
- Where the user picks several looks, the map uses them all as variants.

## Terrain and rule

Only **swamps** are terrain. Puddles are look only (decision 29).

1. **Swamps dig their own shallow dip.** Each swamp carries a depth. The ground
   lowers inside its outline with soft edges, and the water sits flat at the
   old ground level. This joins the river and bridge carving in
   `crates/sim/src/world/carve.rs`; it is not a second terrain owner. The
   simulation's height, slope and line of sight see the dip too. *Why:* it works
   on any ground, and the simulation and the look read the same depth.
2. **The slowdown comes from the swamp, not its depth.** Swamps have a row in
   the game rules, and the whole area slows by it, from edge to middle. Depth
   shapes the terrain and the look, and through the terrain it shapes sight
   (decision 6). It never shapes speed. The route planner counts slow tiles
   (`navigation.rs` `SlowGround`). *Why:* depth-driven speed would be hard to
   balance.
3. **The swamp has one factor per drive:** foot, wheeled and tracked, from
   `Mobility` in `crates/contract/src/catalog.rs`. That makes three numbers. Air
   units are unaffected. *Why:* trucks bogging down worse than tanks is the film
   picture, and it makes choosing the unit for a swamp part of the game.
   Forest's existing two-way split (`forest_infantry_multiplier`,
   `forest_vehicle_multiplier`) is unchanged. *Revised:* this was six numbers
   while puddles were terrain.
4. **A road always keeps road speed, and roads are never carved.** A road
   across a swamp is a causeway. Where a swamp and a forest overlap, the slower
   factor applies. Craters stack on top as they do today. *Why:* each number
   means what it says. The user confirmed "roads always win" for question 8.
5. **Placement:** swamps form in flat ground near the river, and sometimes away
   from it on maps with no river. Each map type in `fixtures/map-presets.json`
   has its own chance, like `river_chance`. Swamps stay clear of settlements,
   buildings and bridge approaches. *Why:* it reads as real land, and it can be
   tuned in the map workbench. Puddle placement is decision 29.
6. **There is no concealment rule for swamps, but the dip may hide units.** No
   sight rule names a swamp. Sight is the general ray from an eye to a target
   over the terrain (`crates/sim/src/visibility.rs`, `sensing.rs`
   `sees_point`). A dip lowers both the target and the eyes of units inside
   it, so a deep enough swamp hides its occupants from distant observers on
   flat ground. They also see less themselves. A vehicle's higher eye
   (`hull.eye_m`) sees in better. Any reeds drawn stay knee-high, so the screen
   never suggests cover the terrain doesn't give. *Revised:* the user liked this
   emergent hiding ("emergent property without hardcoding concealment"), but
   depth is chosen for the look, not for sight (decision 30). At that depth,
   the hiding is marginal, and the user accepts that: swamps are mostly about
   movement.
7. *agent* **Swamps take no track marks,** as water should refuse them. Marks on
   the dry ground around a swamp still show. *Why:* water keeps one rule for
   marks, and the wake is the trace a unit leaves in water. Today only `burst`
   refuses river water; `wear` has no surface check (`crates/sim/src/ground.rs`).
8. *agent* **A hull in standing water raises no dust; it makes a wake instead.**
   *Why:* this is the film picture. Today dust ignores the ground type
   (`presentation/src/effects/frame.rs` `note_movers`).

Rivers stay impassable (`specs/done/battle-foundation/requirements.md`). This
feature adds no fording.

## Look

9. **River water has two variants: temperate green-grey and murky lowland.** No
   tropical turquoise. "Same quality as RedGPU" means the same techniques
   (depth colour, bed, refraction, caustics, ripples) in our palette, not their
   colours.
10. **Swamps have two variants: flooded meadow and low reed marsh.** Reeds stay
    knee-high (decision 6).
11. **Puddles have two variants: muddy and mirror.**
12. **How a variant is chosen:**
    - each river and swamp gets one variant, chosen from weights in the biome
      data (decision 27 says how it is seeded);
    - a puddle's variant follows the ground under it (decision 28);
    - variants are look only.

    *Why:* replays stay exact, two swamps on one map can differ, and puddles
    read as cause and effect.
13. **Wakes show clear rings, plus a V-wake once the unit moves.** The style
    follows the unit's speed: standing or wading slowly gives rings, and a
    moving hull or a squad on the move drags a V. *Why:* this is what a viewer
    expects, and it needs no placement rule.
14. *agent* **The wake area follows the camera and is sized to the tactical view.
    Wakes fade out as you zoom far out.** The exact size comes from a cost
    measurement in the wake slice. *Why:* at far zoom a wake is smaller than a
    pixel.
15. *agent* **Screen-space reflections are not in the first version.** They are
    reconsidered after the highlights slice, on the low-angle water shots
    (decision 17). *Why:* from a steep RTS camera most reflected rays leave the
    screen.
16. *agent* **Normal maps are generated in a shader once at load,** not shipped as
    image files. *Why:* RedGPU's maps carry no licence, and a generated texture
    comes through the shared shader export, so both clients get it with no new
    runtime assets.

## Scope and acceptance

17. **The water must look its best close up (~65 m), at tactical play (~260 m)
    and at low cinematic angles.** Far zoom only has to not look wrong. The
    low angles are shot in the water capture scene (decision 31).
18. **The Low graphics setting drops refraction, caustics and wakes, and keeps
    the depth colour.**
19. **Native must always run, and gets as close to parity as each step
    allows.** The browser leads each step. Every step leaves native starting and
    drawing water at least as well as before, and ports what it reasonably can
    in the same step. Whatever turns out too complicated goes to the
    [water native follow-up](../../water-native/README.md), which is updated
    during implementation. *Revised:* the user replaced "browser first; native
    catches up step by step", saying not to spend too long on native.
20. **Done is a side-by-side read:** at our camera, next to RedGPU, ours reads as
    good. Slice 0's captures are the target for that comparison.

## From the landmine sweep

The sweep of unknown unknowns (2026-10-27) covered the simulation and map
generation, the renderer and native, and presentation. The findings that need
no taste call are folded into the README's
[principles and invariants](README.md#principles-and-invariants). These are the
ones decided from the confirmed preferences or by the user.

21. *agent* **Swamps have wide, soft edges.** The edge spans several 4 m height
    cells, and admission checks depth against edge width when the map loads.
    Depth is set by decision 30. *Why:* a steep rim would add the slope slowdown
    (`slope_multiplier`, `navigation.rs`) on top of the swamp's factor, so speed
    would leak back to depth (decision 2). An edge steeper than
    `steepest_grade` would be an impassable ring. A test proves the sight
    mechanic on a test dip deep enough to show it, not on the tuned depth: from
    flat ground, a far observer loses a unit standing in the dip, and a near or
    high observer still sees it. *Revised:* the depth cap under the fog
    target height was dropped with decision 6.
22. *agent* **A swamp has a minimum size of about three height cells across.**
    *Why:* anything smaller can't be carved, and a hull's speed is sampled at
    its centre only, so a tiny swamp would slow some tanks and not others.
23. *agent* **Rounds landing in a swamp burst at the water's surface and leave no
    crater.** They make a splash, which is a later wake source. Swamps refuse
    craters and track wear alike. *Why:* water keeps one rule for marks
    (decision 7).
24. *agent* **A wake already on the water may keep showing a unit's heading for
    a few seconds after the unit leaves sight.** The wave field clears on a
    side switch, world reload or reset. *Why:* the side saw the wake being
    made, and dust puffs already behave the same way. Clearing on a side
    switch stops one side's waves leaking to the other.
25. *agent* **Water is drawn single-sampled after the MSAA resolve, unless slice
    02's cost probe shows that storing the samples is cheap.** *Why:* the soft
    shoreline blurs water edges anyway, and the world pass discards its MSAA
    samples on purpose to save bandwidth on Apple GPUs (commit `54bc94c03`).
    Glass stays in the MSAA world pass, before the copy. *Revised* while
    drafting: moving glass after water would cost MSAA on every window edge to
    fix water seen through two panes of glass, which an RTS camera almost
    never shows. Slice 02 checks a vehicle-by-the-river crop.
26. *agent* **The one wind keeps the grass heading (35°).** Smoke and dust drift
    turns about 8° to match. The record carries both gust speed and drift
    speed, because they are different quantities. *Why:* grass is the most
    visible wind in the menu reel's liked shots, so it stays put.
27. *agent* **Variants are picked from the biome seed plus each river's or
    swamp's identity,** as tree stands are seeded per cell. This needs a river
    index and a swamp index in the export. *Why:* presentation has no map seed,
    and this follows the existing pattern.
28. *agent* **A puddle's look follows the surface kind at its centre:** mirror
    on `road` and `paving`, muddy on `country_road`, `dirt_track` and open
    ground. *Why:* "hard track" wasn't a real kind; this maps decision 12 onto
    the kinds that exist.
29. **Puddles are look only.** They have no dip, no slowdown and no rules row.
    *agent* They are scattered by the presentation's ground walk from biome
    data, on tracks and fields, like plots and scenery. They are not in the map
    contract, so they change no battle digest. A unit seen crossing one still
    makes a small splash and rings (decision 13), as a cosmetic wake. *Why:*
    the user chose "roads always win" and "make small puddles cosmetic only" to
    simplify. A puddle is too small to carve on a 4 m grid anyway (decision
    22). Anything big enough to matter is a swamp.

## Closing the walk

30. **Swamp depth is chosen for how a unit looks standing in it: about 0.6 m.**
    The water reaches a soldier's thighs, not most of the body. It is a tuning
    number in the data, judged on screen with units in the swamp. *Why:* the
    user cares more about how units look in a swamp than about balancing depth.
    Depth's effect on balance is marginal; swamps are mostly for the movement
    slowdown.
31. **The low-angle shots are taken in a dedicated water capture scene:** a
    fixed test map with a river and a swamp, framed at close, tactical and low
    angles. It is the side-by-side target against RedGPU (decision 20). The
    menu reel stays untouched. *Why:* neither reel map has water, and a new reel
    shot would need directing and approval on its own. Sharp edge: the
    automatic graphics-tier check runs on reel maps, so it won't pay the new
    water's cost until a reel shot has water.

## From slicing

32. *agent* **Swamp water stops rounds and sight at its surface, as river water
    does,** through the one `water_hit` (`crates/sim/src/world/mod.rs`). This
    adds no concealment: the simulation has no prone posture, and a standing
    soldier's eye (1.6 m), aim (1.0 m) and centre (0.9 m) all sit above 0.6 m
    of water. A test proves a soldier standing in a swamp is seen and can be
    hit. *Why:* water keeps one rule. This answers a drafter's objection that
    the water plane might hide low targets.
33. *agent* **A wading soldier is never drawn lying down.** Presentation's pose
    stance skips prone wherever the standing water is deeper than a lying
    soldier's height. *Why:* the film picture; a soldier lying underwater looks
    silly. It is a physical comparison, not a swamp rule.
34. *agent* **Puddle placement is a pure function of the biome seed and the
    ground walk's cell.** *Why:* wakes, grass and both clients must agree on
    where a puddle is whatever the camera has walked (decision 29 needs it).
35. *agent* **The water scene ships the river first. The swamp joins it in slice
    10,** with the swamp contract. *Why:* the scene can't contain a swamp
    before the contract exists, and a placeholder dip would be scaffolding.
